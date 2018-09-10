use wordize::{Pos, Word};

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

pub struct Checker {
    is_comment: bool,
    if_depth: u32,
}

impl Checker {
    /// Create an RMS syntax checker.
    pub fn new() -> Self {
        Checker {
            is_comment: false,
            if_depth: 0,
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

        if self.if_depth == 0 && token.value == "endif" {
            return Some(Warning::warning(token, "Unexpected `endif`–no open if"));
        }

        None
    }

    pub fn write_token(&mut self, token: &Word) -> () {
        match self.lint_token(token) {
            Some(warning) => warning.print(),
            None => (),
        }

        match token.value {
            "/*" => self.is_comment = true,
            "*/" => self.is_comment = false,
            "if" => self.if_depth += 1,
            "endif" => {
                if self.if_depth > 0 {
                    self.if_depth -= 1;
                }
            }
            _ => (),
        }
    }
}
