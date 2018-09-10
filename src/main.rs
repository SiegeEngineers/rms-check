use std::iter::Iterator;
use std::str::CharIndices;

/// Source code position.
#[derive(Clone, Copy)]
struct Pos(usize, u32, u32);
impl Pos {
    /// Get the current character index.
    fn index(&self) -> usize { self.0 }
    /// Get the current line.
    fn line(&self) -> u32 { self.1 }
    /// Get the current column.
    fn column(&self) -> u32 { self.2 }

    /// Advance by a line.
    fn next_line(&mut self) -> () {
        self.1 += 1;
        self.2 = 0;
    }
    /// Advance by a column.
    fn next_column(&mut self) -> () {
        self.2 += 1;
    }
}

/// Iterator over words in a string, with their start and end positions.
struct Wordize<'a> {
    pos: Pos,
    source: &'a str,
    chars: CharIndices<'a>,
}

/// Represents a word.
struct Word<'a> {
    /// Position of the first character in the source code.
    start: Pos,
    /// Position just past the last character.
    end: Pos,
    /// The characters in this word.
    value: &'a str,
}

impl<'a> Wordize<'a> {
    /// Create an iterator over the `source` string's words.
    fn new(source: &'a str) -> Self {
        Wordize {
            pos: Pos(0, 0, 0),
            source,
            chars: source.char_indices(),
        }
    }
}

impl<'a> Iterator for Wordize<'a> {
    type Item = Word<'a>;

    /// Get the next word.
    fn next(&mut self) -> Option<Self::Item> {
        let mut start = Pos(0, 0, 0);
        let mut end = Pos(0, 0, 0);
        let mut saw_word = false;
        while let Some((index, c)) = self.chars.next() {
            if !saw_word {
                if c == '\n' {
                    self.pos.next_line();
                    continue;
                }
                if !char::is_whitespace(c) {
                    saw_word = true;
                    start = Pos(index, self.pos.line(), self.pos.column());
                }
                self.pos.next_column();
                continue;
            }

            if char::is_whitespace(c) {
                end = Pos(index, self.pos.line(), self.pos.column());
                if c == '\n' {
                    self.pos.next_line();
                } else {
                    self.pos.next_column();
                }
                break;
            }

            self.pos.next_column();
        }

        if saw_word {
            // HACK to detect the final token
            if end.index() == 0 {
                end = Pos(self.source.len(), self.pos.line(), self.pos.column());
            }

            let value = &self.source[start.index()..end.index()];
            Some(Word {
                start,
                end,
                value,
            })
        } else {
            None
        }
    }
}

/// Warning severity.
enum Severity {
    /// This needs attention but may not be incorrect.
    Warning,
    /// This is 100% incorrect.
    Error,
}

/// A warning.
struct Warning<'a> {
    /// Warning severity.
    severity: Severity,
    /// The first character in the source code that this warning applies to.
    start: Pos,
    /// The last character in the source code that this warning applies to.
    end: Pos,
    /// Human-readable warning text.
    message: &'a str,
    /// A change suggestion: when present, the problem can be fixed by replacing the
    /// range of text this warning applies to by the string in this suggestion.
    suggestion: Option<String>,
}

impl<'a> Warning<'a> {
    /// Create a new warning with severity "Warning".
    fn warning(token: &Word, message: &'a str) -> Self {
        Warning {
            severity: Severity::Warning,
            start: token.start.clone(),
            end: token.end.clone(),
            message,
            suggestion: None,
        }
    }

    /// Create a new warning with severity "Error".
    fn error(token: &Word, message: &'a str) -> Self {
        Warning {
            severity: Severity::Error,
            start: token.start.clone(),
            end: token.end.clone(),
            message,
            suggestion: None,
        }
    }

    /// Define a replacement suggestion for this warning.
    fn replacement(self, suggestion: &str) -> Self {
        Warning {
            suggestion: Some(suggestion.into()),
            ..self
        }
    }

    /// Print this warning to the screen.
    fn print(&self) -> () {
        eprintln!("({}:{}) {}", self.start.line(), self.start.column(), self.message);
        match &self.suggestion {
            Some(ref msg) => eprintln!("  ! Suggested replacement: {}", msg),
            None => (),
        }
    }
}

struct Checker {
    is_comment: bool,
}
impl Checker {
    /// Create an RMS syntax checker.
    fn new() -> Self {
        Checker {
            is_comment: false,
        }
    }

    /// Check an incoming token.
    fn lint_token(&mut self, token: &Word) -> Option<Warning> {
        if token.value == "*/" && !self.is_comment {
            return Some(Warning::error(token, "Unexpected closing `*/`"))
        }

        // "/**" does not work to open a comment
        if token.value.len() > 2 && token.value.starts_with("/*") {
            let warning = Warning::error(token, "Incorrect comment: there must be a space after the opening /*");
            let replacement = if token.value.ends_with("*/") {
                format!("/* {} */", &token.value[2..token.value.len() - 2])
            } else {
                format!("/* {}", &token.value[2..])
            };
            return Some(warning.replacement(&replacement));
        }

        // "**/" was probably meant to be a closing comment, but only <whitespace>*/ actually closes
        // comments.
        if token.value.len() > 2 && token.value.ends_with("*/") {
            return Some(Warning::warning(token, "Possibly unclosed comment")
                        .replacement(&format!("{} */", &token.value[2..token.value.len() - 2])));
        }

        None
    }

    pub fn write_token(&mut self, token: &Word) -> () {
        match self.lint_token(token) {
            Some(warning) => warning.print(),
            None => (),
        }

        if token.value == "/*" {
            self.is_comment = true;
        } else if token.value == "*/" {
            self.is_comment = false;
        }
    }
}

fn main() {
    let words = Wordize::new(include_str!("../CM_Houseboat_v2.rms"));
    let mut checker = Checker::new();
    words.for_each(|w| checker.write_token(&w));
}
