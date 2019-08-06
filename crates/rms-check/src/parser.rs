use crate::tokens::{ArgType, TOKENS};
use crate::wordize::{Word, Wordize};
use codespan::{ByteIndex, ByteOffset, ByteSpan, FileMap};
use itertools::MultiPeek;
use std::ops::RangeBounds;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningKind {
    MissingConstName,
    MissingConstValue,
    MissingDefineName,
    MissingCommandArgs,
    MissingIfCondition,
    UnclosedComment,
}

#[derive(Debug, Clone)]
pub struct Warning {
    kind: WarningKind,
    span: ByteSpan,
}

impl Warning {
    fn new(span: ByteSpan, kind: WarningKind) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone)]
pub enum Atom<'a> {
    Const(Word<'a>, Word<'a>, Option<Word<'a>>),
    Define(Word<'a>, Word<'a>),
    Section(Word<'a>),
    If(Word<'a>, Word<'a>),
    ElseIf(Word<'a>, Word<'a>),
    Else(Word<'a>),
    EndIf(Word<'a>),
    OpenBlock(Word<'a>),
    CloseBlock(Word<'a>),
    Command(Word<'a>, Vec<Word<'a>>),
    Comment(Word<'a>, String, Option<Word<'a>>),
    Other(Word<'a>),
}

/// A forgiving random map script parser, turning a stream of words into a stream of atoms.
#[derive(Debug)]
pub struct Parser<'a> {
    file_map: &'a FileMap,
    iter: MultiPeek<Wordize<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(file_map: &'a FileMap) -> Self {
        Parser {
            file_map,
            iter: itertools::multipeek(Wordize::new(file_map)),
        }
    }

    fn slice(&self, range: impl RangeBounds<ByteIndex>) -> String {
        use std::ops::Bound::*;
        let start = match range.start_bound() {
            Unbounded => self.file_map.span().start(),
            Included(n) => *n,
            Excluded(n) => *n + ByteOffset(1),
        };
        let end = match range.end_bound() {
            Unbounded => self.file_map.span().end(),
            Included(n) => *n,
            Excluded(n) => *n - ByteOffset(1),
        };
        self.file_map
            .src_slice(ByteSpan::new(start, end))
            .unwrap()
            .to_string()
    }

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
            _ => (),
        }

        Some(token)
    }

    fn read_arg(&mut self) -> Option<Word<'a>> {
        match self.peek_arg() {
            Some(_) => self.iter.next(),
            None => None,
        }
    }

    fn read_comment(&mut self, open_comment: Word<'a>) -> Option<(Atom<'a>, Vec<Warning>)> {
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
                        vec![Warning::new(
                            open_comment.span.to(last_span),
                            WarningKind::UnclosedComment,
                        )],
                    ))
                }
            }
        }
    }

    fn read_command(&mut self, command_name: Word<'a>) -> Option<(Atom<'a>, Vec<Warning>)> {
        // token is guaranteed to exist at this point
        let token_type = &TOKENS[command_name.value];
        let mut args = vec![];
        for _ in 0..token_type.arg_len() {
            match self.read_arg() {
                Some(arg) => args.push(arg),
                _ => break,
            }
        }

        if args.len() == token_type.arg_len() as usize {
            Some((Atom::Command(command_name, args), vec![]))
        } else {
            let span = match args.last() {
                Some(arg) => command_name.span.to(arg.span),
                _ => command_name.span,
            };
            Some((
                Atom::Command(command_name, args),
                vec![Warning::new(span, WarningKind::MissingCommandArgs)],
            ))
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = (Atom<'a>, Vec<Warning>);
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

        match word.value {
            "{" => t(Atom::OpenBlock(word)),
            "}" => t(Atom::CloseBlock(word)),
            "/*" => self.read_comment(word),
            "if" => match self.read_arg() {
                Some(condition) => t(Atom::If(word, condition)),
                None => Some((
                    Atom::Other(word),
                    vec![Warning::new(word.span, WarningKind::MissingIfCondition)],
                )),
            },
            "elseif" => match self.read_arg() {
                Some(condition) => t(Atom::ElseIf(word, condition)),
                None => Some((
                    Atom::Other(word),
                    vec![Warning::new(word.span, WarningKind::MissingIfCondition)],
                )),
            },
            "else" => t(Atom::Else(word)),
            "endif" => t(Atom::EndIf(word)),
            "#define" => match self.read_arg() {
                Some(name) => t(Atom::Define(word, name)),
                None => Some((
                    Atom::Other(word),
                    vec![Warning::new(word.span, WarningKind::MissingDefineName)],
                )),
            },
            "#const" => match (self.read_arg(), self.peek_arg()) {
                (Some(name), Some(_)) => t(Atom::Const(word, name, self.iter.next())),
                (Some(name), None) => Some((
                    Atom::Const(word, name, None),
                    vec![Warning::new(
                        word.span.to(name.span),
                        WarningKind::MissingConstValue,
                    )],
                )),
                (None, _) => Some((
                    Atom::Other(word),
                    vec![Warning::new(word.span, WarningKind::MissingConstName)],
                )),
            },
            command_name if TOKENS.contains_key(command_name) => self.read_command(word),
            _ => t(Atom::Other(word)),
        }
    }
}

/// Check if a string is numeric.
fn is_numeric(s: &str) -> bool {
    s.parse::<i32>().is_ok()
}

/// Check if a string contains a valid rnd(1,10) call.
///
/// Returns a tuple with values:
///
///   0. whether the string was valid
///   1. an optional valid replacement value
fn is_valid_rnd(s: &str) -> (bool, Option<String>) {
    if s.starts_with("rnd(") && s.ends_with(')') && s[4..s.len() - 1].split(',').all(is_numeric) {
        return (true, None);
    } else if s.chars().any(char::is_whitespace) {
        let no_ws = s
            .chars()
            .filter(|c| !char::is_whitespace(*c))
            .collect::<String>();
        if let (true, _) = is_valid_rnd(&no_ws) {
            return (false, Some(no_ws));
        }
    }
    (false, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codespan::{FileMap, FileName};
    fn filemap(source: &str) -> FileMap {
        FileMap::new(FileName::Virtual("test.rms".into()), source.to_string())
    }

    #[test]
    fn const_ok() {
        let mut f = filemap("#const A B");
        let atoms = Parser::new(&mut f).collect::<Vec<(Atom, Vec<Warning>)>>();
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
        let mut f = filemap("#const B");
        let atoms = Parser::new(&mut f).collect::<Vec<(Atom, Vec<Warning>)>>();
        if let (Atom::Const(def, name, val), warnings) = &atoms[0] {
            assert_eq!(def.value, "#const");
            assert_eq!(name.value, "B");
            assert!(val.is_none());
            assert_eq!(warnings.len(), 1);
            assert_eq!(warnings[0].kind, WarningKind::MissingConstValue);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn const_missing_name() {
        let mut f = filemap("#const");
        let atoms = Parser::new(&mut f).collect::<Vec<(Atom, Vec<Warning>)>>();
        if let (Atom::Other(token), warnings) = &atoms[0] {
            assert_eq!(token.value, "#const");
            assert_eq!(warnings.len(), 1);
            assert_eq!(warnings[0].kind, WarningKind::MissingConstName);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn define_ok() {
        let mut f = filemap("#define B");
        let atoms = Parser::new(&mut f).collect::<Vec<(Atom, Vec<Warning>)>>();
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
        let mut f = filemap("#define");
        let atoms = Parser::new(&mut f).collect::<Vec<(Atom, Vec<Warning>)>>();
        if let (Atom::Other(token), warnings) = &atoms[0] {
            assert_eq!(token.value, "#define");
            assert_eq!(warnings.len(), 1);
            assert_eq!(warnings[0].kind, WarningKind::MissingDefineName);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn command_noargs() {
        let mut f = filemap("random_placement");
        let atoms = Parser::new(&mut f).collect::<Vec<(Atom, Vec<Warning>)>>();
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
        let mut f = filemap("land_percent 10 grouped_by_team");
        let atoms = Parser::new(&mut f).collect::<Vec<(Atom, Vec<Warning>)>>();
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

    #[test]
    fn command_block() {
        let mut f = filemap("create_terrain SNOW { base_size 15 }");
        let mut atoms = Parser::new(&mut f).collect::<Vec<(Atom, Vec<Warning>)>>();
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
        let mut f = filemap("create_terrain SNOW /* this is a comment */ { }");
        let mut atoms = Parser::new(&mut f).collect::<Vec<(Atom, Vec<Warning>)>>();
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
}
