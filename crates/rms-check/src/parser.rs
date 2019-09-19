use crate::{
    tokens::TOKENS,
    wordize::{Word, Wordize},
};
use codespan::{ByteIndex, ByteOffset, FileId, Span};
use itertools::MultiPeek;
use std::{
    fmt::{self, Display},
    ops::RangeBounds,
};

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
    pub span: Span,
}

impl ParseError {
    const fn new(span: Span, kind: ParseErrorKind) -> Self {
        ParseError { kind, span }
    }
}

/// A parsed piece of source code.
#[derive(Debug, Clone)]
pub enum Atom<'a> {
    /// A #const definition, with an optional value for incomplete #const statements.
    Const(Word<'a>, Word<'a>, Option<Word<'a>>),
    /// A #define definition.
    Define(Word<'a>, Word<'a>),
    /// A <SECTION> token.
    Section(Word<'a>),
    /// An if statement with a condition.
    If(Word<'a>, Word<'a>),
    /// An elseif statement with a condition.
    ElseIf(Word<'a>, Word<'a>),
    /// An else statement.
    Else(Word<'a>),
    /// An endif statement.
    EndIf(Word<'a>),
    /// A start_random statement.
    StartRandom(Word<'a>),
    /// A percent_chance statement with a chance value.
    PercentChance(Word<'a>, Word<'a>),
    /// An end_random statement.
    EndRandom(Word<'a>),
    /// The start of a block, `{`.
    OpenBlock(Word<'a>),
    /// The end of a block, `}`.
    CloseBlock(Word<'a>),
    /// A command, with a name and arguments.
    Command(Word<'a>, Vec<Word<'a>>),
    /// A comment, with an optional close token in case the comment was not closed.
    Comment(Word<'a>, String, Option<Word<'a>>),
    /// An unrecognised token.
    Other(Word<'a>),
}

impl Atom<'_> {
    /// Get the file this atom was parsed from.
    pub fn file_id(&self) -> FileId {
        use Atom::*;
        match self {
            Section(def) | Else(def) | EndIf(def) | StartRandom(def) | EndRandom(def)
            | OpenBlock(def) | CloseBlock(def) | Other(def) => def.file,
            Const(def, _, _) => def.file,
            Define(def, _) | If(def, _) | ElseIf(def, _) | PercentChance(def, _) => def.file,
            Command(name, _) => name.file,
            Comment(left, _, _) => left.file,
        }
    }

    /// Get the full span for an atom.
    pub fn span(&self) -> Span {
        use Atom::*;
        match self {
            Section(def) | Else(def) | EndIf(def) | StartRandom(def) | EndRandom(def)
            | OpenBlock(def) | CloseBlock(def) | Other(def) => def.span,
            Const(def, name, val) => def.span.merge(val.unwrap_or(*name).span),
            Define(def, arg) | If(def, arg) | ElseIf(def, arg) | PercentChance(def, arg) => {
                def.span.merge(arg.span)
            }
            Command(name, args) => match args.last() {
                Some(arg) => name.span.merge(arg.span),
                None => name.span,
            },
            Comment(left, content, right) => match right {
                Some(right) => left.span.merge(right.span),
                None => Span::new(
                    left.span.start(),
                    left.span.end() + ByteOffset(content.as_bytes().len() as i64),
                ),
            },
        }
    }
}

impl Display for Atom<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Atom::*;
        match self {
            Const(_, name, val) => write!(
                f,
                "Const<{}, {}>",
                name.value,
                val.map(|v| v.value).unwrap_or("()")
            ),
            Define(_, name) => write!(f, "Define<{}>", name.value),
            Section(name) => write!(f, "Section{}", name.value),
            If(_, condition) => write!(f, "If<{}>", condition.value),
            ElseIf(_, condition) => write!(f, "ElseIf<{}>", condition.value),
            Else(_) => write!(f, "Else"),
            EndIf(_) => write!(f, "EndIf"),
            StartRandom(_) => write!(f, "StartRandom"),
            PercentChance(_, chance) => write!(f, "PercentChance<{}>", chance.value),
            EndRandom(_) => write!(f, "EndRandom"),
            OpenBlock(_) => write!(f, "OpenBlock"),
            CloseBlock(_) => write!(f, "CloseBlock"),
            Command(name, args) => write!(
                f,
                "Command<{}{}>",
                name.value,
                args.iter()
                    .map(|a| format!(", {}", a.value))
                    .collect::<String>()
            ),
            Comment(_, content, _) => write!(f, "Comment<{:?}>", content),
            Other(other) => write!(f, "Other<{}>", other.value),
        }
    }
}

/// A forgiving random map script parser, turning a stream of words into a stream of atoms.
#[derive(Debug)]
pub struct Parser<'a> {
    source: &'a str,
    iter: MultiPeek<Wordize<'a>>,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given source code. The FileId is stored on parsed `Atom`s so
    /// their position in the source file can be resolved later on to generate warning messages.
    #[inline]
    pub fn new(file_id: FileId, source: &'a str) -> Self {
        Parser {
            source,
            iter: itertools::multipeek(Wordize::new(file_id, source)),
        }
    }

    /// Take a slice of the source code.
    fn slice(&self, range: impl RangeBounds<ByteIndex>) -> String {
        use std::ops::Bound::*;
        let start = match range.start_bound() {
            Unbounded => ByteIndex(0),
            Included(n) => *n,
            Excluded(n) => *n + ByteOffset(1),
        };
        let end = match range.end_bound() {
            Unbounded => ByteIndex(self.source.as_bytes().len() as u32),
            Included(n) => *n,
            Excluded(n) => *n - ByteOffset(1),
        };
        self.source[start.to_usize()..end.to_usize()].to_string()
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
        let mut last_span = open_comment.span;
        loop {
            match self.iter.next() {
                Some(token @ Word { value: "*/", .. }) => {
                    return Some((
                        Atom::Comment(
                            open_comment,
                            self.slice(open_comment.end()..=token.span.start()),
                            Some(token),
                        ),
                        vec![],
                    ));
                }
                Some(token) => {
                    last_span = token.span;
                }
                None => {
                    return Some((
                        Atom::Comment(open_comment, self.slice(open_comment.end()..), None),
                        vec![ParseError::new(
                            open_comment.span.merge(last_span),
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
        command_name: Word<'a>,
        lower_name: &str,
    ) -> Option<(Atom<'a>, Vec<ParseError>)> {
        let mut warnings = vec![];

        // token is guaranteed to exist at this point
        let token_type = &TOKENS[lower_name];
        let mut args = vec![];
        for _ in 0..token_type.arg_len() {
            match self.read_arg() {
                Some(arg) => args.push(arg),
                _ => break,
            }
        }

        if args.len() != token_type.arg_len() as usize {
            let span = match args.last() {
                Some(arg) => command_name.span.merge(arg.span),
                _ => command_name.span,
            };
            warnings.push(ParseError::new(span, ParseErrorKind::MissingCommandArgs));
        }
        Some((Atom::Command(command_name, args), warnings))
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
            return t(Atom::Section(word));
        }

        match word.value.to_ascii_lowercase().as_str() {
            "{" => t(Atom::OpenBlock(word)),
            "}" => t(Atom::CloseBlock(word)),
            "/*" => self.read_comment(word),
            "if" => match self.read_arg() {
                Some(condition) => t(Atom::If(word, condition)),
                None => Some((
                    Atom::Other(word),
                    vec![ParseError::new(
                        word.span,
                        ParseErrorKind::MissingIfCondition,
                    )],
                )),
            },
            "elseif" => match self.read_arg() {
                Some(condition) => t(Atom::ElseIf(word, condition)),
                None => Some((
                    Atom::Other(word),
                    vec![ParseError::new(
                        word.span,
                        ParseErrorKind::MissingIfCondition,
                    )],
                )),
            },
            "else" => t(Atom::Else(word)),
            "endif" => t(Atom::EndIf(word)),
            "start_random" => t(Atom::StartRandom(word)),
            "percent_chance" => match self.read_arg() {
                Some(chance) => t(Atom::PercentChance(word, chance)),
                None => Some((
                    Atom::Other(word),
                    vec![ParseError::new(
                        word.span,
                        ParseErrorKind::MissingPercentChance,
                    )],
                )),
            },
            "end_random" => t(Atom::EndRandom(word)),
            "#define" => match self.read_arg() {
                Some(name) => t(Atom::Define(word, name)),
                None => Some((
                    Atom::Other(word),
                    vec![ParseError::new(
                        word.span,
                        ParseErrorKind::MissingDefineName,
                    )],
                )),
            },
            "#const" => match (self.read_arg(), self.peek_arg()) {
                (Some(name), Some(_)) => t(Atom::Const(word, name, self.iter.next())),
                (Some(name), None) => Some((
                    Atom::Const(word, name, None),
                    vec![ParseError::new(
                        word.span.merge(name.span),
                        ParseErrorKind::MissingConstValue,
                    )],
                )),
                (None, _) => Some((
                    Atom::Other(word),
                    vec![ParseError::new(word.span, ParseErrorKind::MissingConstName)],
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
                t(Atom::Comment(
                    Word {
                        file: word.file,
                        value: &word.value[0..2],
                        span: Span::new(word.span.start(), word.span.start() + ByteOffset(2)),
                    },
                    word.value[2..word.value.len() - 2].to_string(),
                    Some(Word {
                        file: word.file,
                        value: &word.value[word.value.len() - 2..],
                        span: Span::new(word.span.end() - ByteOffset(2), word.span.end()),
                    }),
                ))
            }
            _ => Some((
                Atom::Other(word),
                vec![ParseError::new(word.span, ParseErrorKind::UnknownWord)],
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_id() -> FileId {
        codespan::Files::new().add("", "")
    }

    #[test]
    fn const_ok() {
        let atoms =
            Parser::new(file_id(), "#const A B").collect::<Vec<(Atom<'_>, Vec<ParseError>)>>();
        if let (Atom::Const(def, name, val), warnings) = &atoms[0] {
            assert_eq!(def.value, "#const");
            assert_eq!(name.value, "A");
            assert!(val.is_some());
            assert_eq!(val.unwrap().value, "B");
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
    }

    #[test]
    fn const_missing_value() {
        let atoms =
            Parser::new(file_id(), "#const B").collect::<Vec<(Atom<'_>, Vec<ParseError>)>>();
        if let (Atom::Const(def, name, val), warnings) = &atoms[0] {
            assert_eq!(def.value, "#const");
            assert_eq!(name.value, "B");
            assert!(val.is_none());
            assert_eq!(warnings.len(), 1);
            assert_eq!(warnings[0].kind, ParseErrorKind::MissingConstValue);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn const_missing_name() {
        let atoms = Parser::new(file_id(), "#const").collect::<Vec<(Atom<'_>, Vec<ParseError>)>>();
        if let (Atom::Other(token), warnings) = &atoms[0] {
            assert_eq!(token.value, "#const");
            assert_eq!(warnings.len(), 1);
            assert_eq!(warnings[0].kind, ParseErrorKind::MissingConstName);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn define_ok() {
        let atoms =
            Parser::new(file_id(), "#define B").collect::<Vec<(Atom<'_>, Vec<ParseError>)>>();
        if let (Atom::Define(def, name), warnings) = &atoms[0] {
            assert_eq!(def.value, "#define");
            assert_eq!(name.value, "B");
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
    }

    #[test]
    fn define_missing_name() {
        let atoms = Parser::new(file_id(), "#define").collect::<Vec<(Atom<'_>, Vec<ParseError>)>>();
        if let (Atom::Other(token), warnings) = &atoms[0] {
            assert_eq!(token.value, "#define");
            assert_eq!(warnings.len(), 1);
            assert_eq!(warnings[0].kind, ParseErrorKind::MissingDefineName);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn command_noargs() {
        let atoms = Parser::new(file_id(), "random_placement")
            .collect::<Vec<(Atom<'_>, Vec<ParseError>)>>();
        assert_eq!(atoms.len(), 1);
        if let (Atom::Command(name, args), warnings) = &atoms[0] {
            assert_eq!(name.value, "random_placement");
            assert!(args.is_empty());
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
    }

    #[test]
    fn command_1arg() {
        let atoms = Parser::new(file_id(), "land_percent 10 grouped_by_team")
            .collect::<Vec<(Atom<'_>, Vec<ParseError>)>>();
        assert_eq!(atoms.len(), 2);
        if let (Atom::Command(name, args), warnings) = &atoms[0] {
            assert_eq!(name.value, "land_percent");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].value, "10");
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
        if let (Atom::Command(name, args), warnings) = &atoms[1] {
            assert_eq!(name.value, "grouped_by_team");
            assert!(args.is_empty());
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
    }

    /// It should accept wrong casing, a linter can fix it up.
    #[test]
    fn command_wrong_case() {
        let atoms = Parser::new(file_id(), "land_Percent 10 grouped_BY_team")
            .collect::<Vec<(Atom<'_>, Vec<ParseError>)>>();
        assert_eq!(atoms.len(), 2);
        if let (Atom::Command(name, args), warnings) = &atoms[0] {
            assert_eq!(name.value, "land_Percent");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].value, "10");
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
        if let (Atom::Command(name, args), warnings) = &atoms[1] {
            assert_eq!(name.value, "grouped_BY_team");
            assert!(args.is_empty());
            assert!(warnings.is_empty());
        } else {
            assert!(false);
        }
    }

    #[test]
    fn command_block() {
        let mut atoms = Parser::new(file_id(), "create_terrain SNOW { base_size 15 }")
            .collect::<Vec<(Atom<'_>, Vec<ParseError>)>>();
        assert_eq!(atoms.len(), 4);
        if let (Atom::Command(name, args), _) = atoms.remove(0) {
            assert_eq!(name.value, "create_terrain");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].value, "SNOW");
        } else {
            assert!(false)
        }
        if let (Atom::OpenBlock(tok), _) = atoms.remove(0) {
            assert_eq!(tok.value, "{");
        } else {
            assert!(false)
        }
        if let (Atom::Command(name, args), _) = atoms.remove(0) {
            assert_eq!(name.value, "base_size");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].value, "15");
        } else {
            assert!(false)
        }
        if let (Atom::CloseBlock(tok), _) = atoms.remove(0) {
            assert_eq!(tok.value, "}");
        } else {
            assert!(false)
        }
    }

    #[test]
    fn comment_basic() {
        let mut atoms = Parser::new(file_id(), "create_terrain SNOW /* this is a comment */ { }")
            .collect::<Vec<(Atom<'_>, Vec<ParseError>)>>();
        assert_eq!(atoms.len(), 4);
        if let (Atom::Command(_, _), _) = atoms.remove(0) {
            // ok
        } else {
            assert!(false)
        }
        if let (Atom::Comment(start, content, end), _) = atoms.remove(0) {
            assert_eq!(start.value, "/*");
            assert_eq!(content, " this is a comment ");
            assert_eq!(end.unwrap().value, "*/");
        } else {
            assert!(false)
        }
        if let (Atom::OpenBlock(tok), _) = atoms.remove(0) {
            assert_eq!(tok.value, "{");
        } else {
            assert!(false)
        }
        if let (Atom::CloseBlock(tok), _) = atoms.remove(0) {
            assert_eq!(tok.value, "}");
        } else {
            assert!(false)
        }
    }

    #[test]
    fn dry_arabia() {
        let f = std::fs::read("tests/rms/Dry Arabia.rms").unwrap();
        let source = std::str::from_utf8(&f).unwrap();
        for (atom, _) in Parser::new(file_id(), source) {
            if let Atom::Other(_) = atom {
                panic!("unrecognised atom");
            }
        }
    }
}
