//! AoE2 random map script parser, turning a source string into a sequence of parsed units called "atoms".

use crate::diagnostic::{ByteIndex, FileId, SourceLocation};
use crate::tokenizer::{Tokenizer, Word};
use crate::tokens::TOKENS;
use cow_utils::CowUtils;
use itertools::MultiPeek;
use std::fmt::{self, Display};
use std::ops::RangeBounds;

/// The kind of error that generated a Parser warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// A #const token was found, but no constant name followed it.
    MissingConstName,
    /// A #const token was found, but it was declared without a value.
    MissingConstValue,
    /// A #define token was found, but no constant name followed it.
    MissingDefineName,
    /// A command was found, but it is missing arguments.
    MissingCommandArgs,
    /// An if token was found, but no condition followed it.
    MissingIfCondition,
    /// A percent_chance token was found, but no number followed it.
    MissingPercentChance,
    /// A comment was not closed before the end of the file.
    UnclosedComment,
    /// Found an unknown word.
    UnknownWord,
}

/// An error that can occur during parsing. The Parser will keep going after encountering parse
/// errors.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub location: SourceLocation,
}

impl ParseError {
    const fn new(location: SourceLocation, kind: ParseErrorKind) -> Self {
        ParseError { kind, location }
    }
}

/// A parsed piece of source code.
#[derive(Debug, Clone)]
pub enum AtomKind<'a> {
    /// A #const definition, with an optional value for incomplete #const statements.
    Const {
        head: Word<'a>,
        name: Word<'a>,
        value: Option<Word<'a>>,
    },
    /// A #define definition.
    Define { head: Word<'a>, name: Word<'a> },
    /// An #undefine statement.
    Undefine { head: Word<'a>, name: Word<'a> },
    /// A <SECTION> token.
    Section { name: Word<'a> },
    /// An if statement with a condition.
    If { head: Word<'a>, condition: Word<'a> },
    /// An elseif statement with a condition.
    ElseIf { head: Word<'a>, condition: Word<'a> },
    /// An else statement.
    Else { head: Word<'a> },
    /// An endif statement.
    EndIf { head: Word<'a> },
    /// A start_random statement.
    StartRandom { head: Word<'a> },
    /// A percent_chance statement with a chance value.
    PercentChance { head: Word<'a>, chance: Word<'a> },
    /// An end_random statement.
    EndRandom { head: Word<'a> },
    /// The start of a block, `{`.
    OpenBlock { head: Word<'a> },
    /// The end of a block, `}`.
    CloseBlock { head: Word<'a> },
    /// A command, with a name and arguments.
    Command {
        name: Word<'a>,
        arguments: Vec<Word<'a>>,
    },
    /// A comment, with an optional close token in case the comment was not closed.
    Comment {
        open: Word<'a>,
        content: String,
        close: Option<Word<'a>>,
    },
    /// An unrecognised token.
    Other { value: Word<'a> },
}

#[derive(Debug, Clone)]
pub struct Atom<'a> {
    /// The kind of atom, and data about this kind of atom.
    pub kind: AtomKind<'a>,
    /// The source code location this atom was parsed from.
    pub location: SourceLocation,
}

impl<'a> Atom<'a> {
    /// Construct an atom with the given kind from a word, inheriting its location information.
    const fn from_word(kind: AtomKind<'a>, word: Word<'_>) -> Self {
        Self {
            kind,
            location: word.location,
        }
    }

    /// Construct an unknown atom from a word, inheriting its location information.
    const fn other(word: Word<'a>) -> Self {
        Self::from_word(AtomKind::Other { value: word }, word)
    }

    /// Get the ID of the file this atom was parsed from.
    pub const fn file(&self) -> FileId {
        self.location.file()
    }

    /// Get the full span for this atom.
    pub const fn range(&self) -> std::ops::Range<ByteIndex> {
        self.location.range()
    }
}

impl Display for Atom<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            AtomKind::Const { name, value, .. } => write!(
                f,
                "Const<{}, {}>",
                name.value,
                value.map(|v| v.value).unwrap_or("()")
            ),
            AtomKind::Define { name, .. } => write!(f, "Define<{}>", name.value),
            AtomKind::Undefine { name, .. } => write!(f, "Undefine<{}>", name.value),
            AtomKind::Section { name } => write!(f, "Section{}", name.value),
            AtomKind::If { condition, .. } => write!(f, "If<{}>", condition.value),
            AtomKind::ElseIf { condition, .. } => write!(f, "ElseIf<{}>", condition.value),
            AtomKind::Else { .. } => write!(f, "Else"),
            AtomKind::EndIf { .. } => write!(f, "EndIf"),
            AtomKind::StartRandom { .. } => write!(f, "StartRandom"),
            AtomKind::PercentChance { chance, .. } => write!(f, "PercentChance<{}>", chance.value),
            AtomKind::EndRandom { .. } => write!(f, "EndRandom"),
            AtomKind::OpenBlock { .. } => write!(f, "OpenBlock"),
            AtomKind::CloseBlock { .. } => write!(f, "CloseBlock"),
            AtomKind::Command { name, arguments } => write!(
                f,
                "Command<{}{}>",
                name.value,
                arguments
                    .iter()
                    .map(|a| format!(", {}", a.value))
                    .collect::<String>()
            ),
            AtomKind::Comment { content, .. } => write!(f, "Comment<{:?}>", content),
            AtomKind::Other { value } => write!(f, "Other<{}>", value.value),
        }
    }
}

/// A forgiving random map script parser, turning a stream of words into a stream of atoms.
#[derive(Debug)]
pub struct Parser<'a> {
    source: &'a str,
    iter: MultiPeek<Tokenizer<'a>>,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given source code. The FileId is stored on parsed `Atom`s so
    /// their position in the source file can be resolved later on to generate warning messages.
    pub fn new(file_id: FileId, source: &'a str) -> Self {
        Parser {
            source,
            iter: itertools::multipeek(Tokenizer::new(file_id, source)),
        }
    }

    /// Take a slice of the source code.
    fn slice(&self, range: impl RangeBounds<ByteIndex>) -> String {
        use std::ops::Bound::*;
        let start = match range.start_bound() {
            Unbounded => ByteIndex::from(0),
            Included(n) => *n,
            Excluded(n) => *n + 1,
        };
        let end = match range.end_bound() {
            Unbounded => ByteIndex::from(self.source.as_bytes().len()),
            Included(n) => *n,
            Excluded(n) => *n - 1,
        };
        self.source[start.into()..end.into()].to_string()
    }

    /// Check if the next word could be a command argument. If yes, return it; else return None.
    fn peek_arg(&mut self) -> Option<&Word<'a>> {
        let token = match self.iter.peek() {
            Some(token) => token,
            None => return None,
        };

        // Things that should never be args
        match token.value {
            "/*" | "*/" | "{" | "}" => return None,
            "if" | "elseif" | "else" | "endif" => return None,
            "start_random" | "percent_chance" | "end_random" => return None,
            command_name if TOKENS.contains_key(command_name) => return None,
            // incorrect comment syntax but ok
            val if val.starts_with("/*") || val.ends_with("*/") => return None,
            _ => (),
        }

        Some(token)
    }

    /// Read a command argument.
    fn read_arg(&mut self) -> Option<Word<'a>> {
        match self.peek_arg() {
            Some(_) => self.iter.next(),
            None => None,
        }
    }

    /// Read a comment.
    fn read_comment(&mut self, open_comment: Word<'a>) -> Option<(Atom<'a>, Vec<ParseError>)> {
        let mut last_span = open_comment.location;
        loop {
            match self.iter.next() {
                Some(token @ Word { value: "*/", .. }) => {
                    return Some((
                        Atom {
                            kind: AtomKind::Comment {
                                open: open_comment,
                                content: self.slice(open_comment.end()..=token.start()),
                                close: Some(token),
                            },
                            location: SourceLocation::new(
                                open_comment.location.file(),
                                open_comment.location.start()..token.location.end(),
                            ),
                        },
                        vec![],
                    ));
                }
                Some(token) => {
                    last_span = token.location;
                }
                None => {
                    return Some((
                        Atom {
                            kind: AtomKind::Comment {
                                open: open_comment,
                                content: self.slice(open_comment.end()..),
                                close: None,
                            },
                            location: SourceLocation::new(
                                open_comment.location.file(),
                                open_comment.location.start()..last_span.end(),
                            ),
                        },
                        vec![ParseError::new(
                            SourceLocation::new(
                                open_comment.location.file(),
                                open_comment.location.start()..last_span.end(),
                            ),
                            ParseErrorKind::UnclosedComment,
                        )],
                    ))
                }
            }
        }
    }

    /// Read a command with arguments.
    fn read_command(
        &mut self,
        name: Word<'a>,
        lower_name: &str,
    ) -> Option<(Atom<'a>, Vec<ParseError>)> {
        let mut warnings = vec![];

        // token is guaranteed to exist at this point
        let token_type = &TOKENS[lower_name];
        let mut arguments = vec![];
        for _ in 0..token_type.arg_len() {
            match self.read_arg() {
                Some(arg) => arguments.push(arg),
                _ => break,
            }
        }

        let range = match arguments.last() {
            Some(arg) => name.location.start()..arg.location.end(),
            _ => name.location.range(),
        };
        if arguments.len() != token_type.arg_len() as usize {
            warnings.push(ParseError::new(
                SourceLocation::new(name.location.file(), range.clone()),
                ParseErrorKind::MissingCommandArgs,
            ));
        }
        Some((
            Atom {
                kind: AtomKind::Command { name, arguments },
                location: SourceLocation::new(name.location.file(), range),
            },
            warnings,
        ))
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = (Atom<'a>, Vec<ParseError>);
    fn next(&mut self) -> Option<Self::Item> {
        let word = match self.iter.next() {
            Some(word) => word,
            None => return None,
        };

        let t = |atom| Some((atom, vec![]));

        if word.value.starts_with('<')
            && word.value.ends_with('>')
            && TOKENS.contains_key(word.value)
        {
            return t(Atom::from_word(AtomKind::Section { name: word }, word));
        }

        match word.value.cow_to_ascii_lowercase().as_ref() {
            "{" => t(Atom::from_word(AtomKind::OpenBlock { head: word }, word)),
            "}" => t(Atom::from_word(AtomKind::CloseBlock { head: word }, word)),
            "/*" => self.read_comment(word),
            "if" => match self.read_arg() {
                Some(condition) => t(Atom {
                    kind: AtomKind::If {
                        head: word,
                        condition,
                    },
                    location: SourceLocation::new(
                        word.location.file(),
                        word.start()..condition.end(),
                    ),
                }),
                None => Some((
                    Atom::other(word),
                    vec![ParseError::new(
                        word.location,
                        ParseErrorKind::MissingIfCondition,
                    )],
                )),
            },
            "elseif" => match self.read_arg() {
                Some(condition) => t(Atom {
                    kind: AtomKind::ElseIf {
                        head: word,
                        condition,
                    },
                    location: SourceLocation::new(
                        word.location.file(),
                        word.start()..condition.end(),
                    ),
                }),
                None => Some((
                    Atom::other(word),
                    vec![ParseError::new(
                        word.location,
                        ParseErrorKind::MissingIfCondition,
                    )],
                )),
            },
            "else" => t(Atom::from_word(AtomKind::Else { head: word }, word)),
            "endif" => t(Atom::from_word(AtomKind::EndIf { head: word }, word)),
            "start_random" => t(Atom::from_word(AtomKind::StartRandom { head: word }, word)),
            "percent_chance" => match self.read_arg() {
                Some(chance) => t(Atom {
                    kind: AtomKind::PercentChance { head: word, chance },
                    location: SourceLocation::new(word.location.file(), word.start()..chance.end()),
                }),
                None => Some((
                    Atom::other(word),
                    vec![ParseError::new(
                        word.location,
                        ParseErrorKind::MissingPercentChance,
                    )],
                )),
            },
            "end_random" => t(Atom {
                kind: AtomKind::EndRandom { head: word },
                location: word.location,
            }),
            "#define" => match self.read_arg() {
                Some(name) => t(Atom {
                    kind: AtomKind::Define { head: word, name },
                    location: SourceLocation::new(word.location.file(), word.start()..name.end()),
                }),
                None => Some((
                    Atom::other(word),
                    vec![ParseError::new(
                        word.location,
                        ParseErrorKind::MissingDefineName,
                    )],
                )),
            },
            "#undefine" => match self.read_arg() {
                Some(name) => t(Atom {
                    kind: AtomKind::Undefine { head: word, name },
                    location: SourceLocation::new(word.location.file(), word.start()..name.end()),
                }),
                None => Some((
                    Atom::other(word),
                    vec![ParseError::new(
                        word.location,
                        ParseErrorKind::MissingDefineName,
                    )],
                )),
            },
            "#const" => match (self.read_arg(), self.peek_arg()) {
                (Some(name), Some(_)) => {
                    let value = self.iter.next();
                    let range = word.start()..value.unwrap().end();
                    t(Atom {
                        kind: AtomKind::Const {
                            head: word,
                            name,
                            value,
                        },
                        location: SourceLocation::new(word.location.file(), range),
                    })
                }
                (Some(name), None) => Some((
                    Atom {
                        kind: AtomKind::Const {
                            head: word,
                            name,
                            value: None,
                        },
                        location: SourceLocation::new(
                            word.location.file(),
                            word.start()..name.end(),
                        ),
                    },
                    vec![ParseError::new(
                        SourceLocation::new(word.location.file(), word.start()..name.end()),
                        ParseErrorKind::MissingConstValue,
                    )],
                )),
                (None, _) => Some((
                    Atom::other(word),
                    vec![ParseError::new(
                        word.location,
                        ParseErrorKind::MissingConstName,
                    )],
                )),
            },
            command_name if TOKENS.contains_key(command_name) => {
                self.read_command(word, command_name)
            }
            // a common mistake is to do /****/ on a line, which is not strictly a comment because
            // of missing spaces. Effectively it's still ignored by the game though, so we can
            // pretend that it is a comment.
            val if val.starts_with("/*") && val.ends_with("*/") => {
                // Split the word up
                t(Atom::from_word(
                    AtomKind::Comment {
                        open: Word {
                            value: &word.value[0..2],
                            location: SourceLocation::new(
                                word.location.file(),
                                word.start()..word.start() + 2,
                            ),
                        },
                        content: word.value[2..word.value.len() - 2].to_string(),
                        close: Some(Word {
                            value: &word.value[word.value.len() - 2..],
                            location: SourceLocation::new(
                                word.location.file(),
                                word.end() - 2..word.end(),
                            ),
                        }),
                    },
                    word,
                ))
            }
            _ => Some((
                Atom::other(word),
                vec![ParseError::new(word.location, ParseErrorKind::UnknownWord)],
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_ok() {
        let atoms = Parser::new(FileId::new(0), "#const A B")
            .map(|(atom, errors)| (atom.kind, errors))
            .collect::<Vec<_>>();
        if let (AtomKind::Const { head, name, value }, warnings) = &atoms[0] {
            assert_eq!(head.value, "#const");
            assert_eq!(name.value, "A");
            assert!(value.is_some());
            assert_eq!(value.unwrap().value, "B");
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
    }

    #[test]
    fn const_missing_value() {
        let atoms = Parser::new(FileId::new(0), "#const B")
            .map(|(atom, errors)| (atom.kind, errors))
            .collect::<Vec<_>>();
        if let (AtomKind::Const { head, name, value }, warnings) = &atoms[0] {
            assert_eq!(head.value, "#const");
            assert_eq!(name.value, "B");
            assert!(value.is_none());
            assert_eq!(warnings.len(), 1);
            assert_eq!(warnings[0].kind, ParseErrorKind::MissingConstValue);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn const_missing_name() {
        let atoms = Parser::new(FileId::new(0), "#const")
            .map(|(atom, errors)| (atom.kind, errors))
            .collect::<Vec<_>>();
        if let (AtomKind::Other { value }, warnings) = &atoms[0] {
            assert_eq!(value.value, "#const");
            assert_eq!(warnings.len(), 1);
            assert_eq!(warnings[0].kind, ParseErrorKind::MissingConstName);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn define_ok() {
        let atoms = Parser::new(FileId::new(0), "#define B")
            .map(|(atom, errors)| (atom.kind, errors))
            .collect::<Vec<_>>();
        if let (AtomKind::Define { head, name }, warnings) = &atoms[0] {
            assert_eq!(head.value, "#define");
            assert_eq!(name.value, "B");
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
    }

    #[test]
    fn define_missing_name() {
        let atoms = Parser::new(FileId::new(0), "#define")
            .map(|(atom, errors)| (atom.kind, errors))
            .collect::<Vec<_>>();
        if let (AtomKind::Other { value }, warnings) = &atoms[0] {
            assert_eq!(value.value, "#define");
            assert_eq!(warnings.len(), 1);
            assert_eq!(warnings[0].kind, ParseErrorKind::MissingDefineName);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn command_noargs() {
        let atoms = Parser::new(FileId::new(0), "random_placement")
            .map(|(atom, errors)| (atom.kind, errors))
            .collect::<Vec<_>>();
        assert_eq!(atoms.len(), 1);
        if let (AtomKind::Command { name, arguments }, warnings) = &atoms[0] {
            assert_eq!(name.value, "random_placement");
            assert!(arguments.is_empty());
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
    }

    #[test]
    fn command_1arg() {
        let atoms = Parser::new(FileId::new(0), "land_percent 10 grouped_by_team")
            .map(|(atom, errors)| (atom.kind, errors))
            .collect::<Vec<_>>();
        assert_eq!(atoms.len(), 2);
        if let (AtomKind::Command { name, arguments }, warnings) = &atoms[0] {
            assert_eq!(name.value, "land_percent");
            assert_eq!(arguments.len(), 1);
            assert_eq!(arguments[0].value, "10");
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
        if let (AtomKind::Command { name, arguments }, warnings) = &atoms[1] {
            assert_eq!(name.value, "grouped_by_team");
            assert!(arguments.is_empty());
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
    }

    /// It should accept wrong casing, a linter can fix it up.
    #[test]
    fn command_wrong_case() {
        let atoms = Parser::new(FileId::new(0), "land_Percent 10 grouped_BY_team")
            .map(|(atom, errors)| (atom.kind, errors))
            .collect::<Vec<_>>();
        assert_eq!(atoms.len(), 2);
        if let (AtomKind::Command { name, arguments }, warnings) = &atoms[0] {
            assert_eq!(name.value, "land_Percent");
            assert_eq!(arguments.len(), 1);
            assert_eq!(arguments[0].value, "10");
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
        if let (AtomKind::Command { name, arguments }, warnings) = &atoms[1] {
            assert_eq!(name.value, "grouped_BY_team");
            assert!(arguments.is_empty());
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
    }

    #[test]
    fn command_block() {
        let mut atoms = Parser::new(FileId::new(0), "create_terrain SNOW { base_size 15 }")
            .map(|(atom, errors)| (atom.kind, errors))
            .collect::<Vec<_>>();
        assert_eq!(atoms.len(), 4);
        if let (AtomKind::Command { name, arguments }, _) = atoms.remove(0) {
            assert_eq!(name.value, "create_terrain");
            assert_eq!(arguments.len(), 1);
            assert_eq!(arguments[0].value, "SNOW");
        } else {
            assert!(false)
        }
        if let (AtomKind::OpenBlock { head }, _) = atoms.remove(0) {
            assert_eq!(head.value, "{");
        } else {
            assert!(false)
        }
        if let (AtomKind::Command { name, arguments }, _) = atoms.remove(0) {
            assert_eq!(name.value, "base_size");
            assert_eq!(arguments.len(), 1);
            assert_eq!(arguments[0].value, "15");
        } else {
            assert!(false)
        }
        if let (AtomKind::CloseBlock { head }, _) = atoms.remove(0) {
            assert_eq!(head.value, "}");
        } else {
            assert!(false)
        }
    }

    #[test]
    fn comment_basic() {
        let mut atoms = Parser::new(
            FileId::new(0),
            "create_terrain SNOW /* this is a comment */ { }",
        )
        .map(|(atom, errors)| (atom.kind, errors))
        .collect::<Vec<_>>();
        assert_eq!(atoms.len(), 4);
        if let (AtomKind::Command { .. }, _) = atoms.remove(0) {
            // ok
        } else {
            assert!(false)
        }
        if let (
            AtomKind::Comment {
                open,
                content,
                close,
            },
            _,
        ) = atoms.remove(0)
        {
            assert_eq!(open.value, "/*");
            assert_eq!(content, " this is a comment ");
            assert_eq!(close.unwrap().value, "*/");
        } else {
            assert!(false)
        }
        if let (AtomKind::OpenBlock { head }, _) = atoms.remove(0) {
            assert_eq!(head.value, "{");
        } else {
            assert!(false)
        }
        if let (AtomKind::CloseBlock { head }, _) = atoms.remove(0) {
            assert_eq!(head.value, "}");
        } else {
            assert!(false)
        }
    }

    #[test]
    fn dry_arabia() {
        let f = std::fs::read("tests/rms/Dry Arabia.rms").unwrap();
        let source = std::str::from_utf8(&f).unwrap();
        for (atom, _) in Parser::new(FileId::new(0), source) {
            if let AtomKind::Other { .. } = atom.kind {
                panic!("unrecognised atom");
            }
        }
    }
}
