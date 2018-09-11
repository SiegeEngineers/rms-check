use std::collections::HashSet;
use wordize::{Pos, Word};
use tokens::{ArgType, TokenType, TOKENS};

/// Warning severity.
#[derive(Clone, Copy)]
pub enum Severity {
    /// This needs attention but may not be incorrect.
    Warning,
    /// This is 100% incorrect.
    Error,
}

/// A warning.
pub struct Warning {
    /// Warning severity.
    severity: Severity,
    /// The first character in the source code that this warning applies to.
    start: Pos,
    /// The last character in the source code that this warning applies to.
    end: Pos,
    /// Human-readable warning text.
    message: String,
    /// A change suggestion: when present, the problem can be fixed by replacing the
    /// range of text this warning applies to by the string in this suggestion.
    suggestion: Option<String>,
}

impl Warning {
    pub fn severity(&self) -> Severity {
        self.severity
    }
    pub fn start(&self) -> &Pos {
        &self.start
    }
    pub fn end(&self) -> &Pos {
        &self.end
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn suggestion(&self) -> &Option<String> {
        &self.suggestion
    }

    /// Create a new warning with severity "Warning".
    fn warning(token: &Word, message: String) -> Self {
        Warning {
            severity: Severity::Warning,
            start: token.start.clone(),
            end: token.end.clone(),
            message,
            suggestion: None,
        }
    }

    /// Create a new warning with severity "Error".
    fn error(token: &Word, message: String) -> Self {
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
        eprintln!("({}:{}) {}", self.start.line(), self.start.column(), &self.message);
        match &self.suggestion {
            Some(ref msg) => eprintln!("  ! Suggested replacement: {}", msg),
            None => (),
        }
    }
}

pub struct Checker {
    is_comment: bool,
    if_depth: u32,
    current_token: Option<&'static TokenType>,
    token_arg_index: u8,
    next_is_const: bool,
    next_is_define: bool,
    seen_consts: HashSet<String>,
    seen_defines: HashSet<String>,
}

const BUILTIN_NAMES: [&str; 6] = [
    "TINY_MAP",
    "SMALL_MAP",
    "MEDIUM_MAP",
    "LARGE_MAP",
    "HUGE_MAP",
    "GIGANTIC_MAP",
];

impl Checker {
    /// Create an RMS syntax checker.
    pub fn new() -> Self {
        let mut seen_defines = HashSet::new();
        for name in BUILTIN_NAMES.iter() {
            seen_defines.insert((*name).into());
        }

        Checker {
            is_comment: false,
            if_depth: 0,
            current_token: None,
            token_arg_index: 0,
            next_is_const: false,
            next_is_define: false,
            seen_consts: HashSet::new(),
            seen_defines,
        }
    }

    fn check_arg_type(&self, arg_type: &ArgType, token: &Word) -> Option<Warning> {
        match arg_type {
            ArgType::Number => {
                // Check if this is
                // 1. a number
                // 2. a #const-ed constant
                None
            },
            ArgType::Word => None,
            ArgType::OptionalToken => {
                if !self.seen_defines.contains(token.value) {
                    Some(Warning::warning(token, format!("Token `{}` is never defined", token.value)))
                } else {
                    None
                }
            },
            ArgType::Token => {
                // 1. Check if this may or may not be defined—else warn
                if !self.seen_consts.contains(token.value) {
                    if self.seen_defines.contains(token.value) {
                        // 2. Check if this has a value (is defined using #const)—else warn
                        Some(Warning::warning(token, format!("Expected a valued token (defined using #const), got a valueless token `{}` (defined using #define)", token.value)))
                    } else {
                        Some(Warning::warning(token, format!("Token `{}` is never defined", token.value)))
                    }
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    /// Check an incoming token.
    fn lint_token(&mut self, token: &Word) -> Option<Warning> {
        if token.value == "*/" && !self.is_comment {
            return Some(Warning::error(token, "Unexpected closing `*/`".into()))
        }

        // "/**" does not work to open a comment
        if token.value.len() > 2 && token.value.starts_with("/*") {
            let warning = Warning::error(token, "Incorrect comment: there must be a space after the opening /*".into());
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
            return Some(Warning::warning(token, "Possibly unclosed comment".into())
                        .replacement(&format!("{} */", &token.value[2..token.value.len() - 2])));
        }

        if self.if_depth == 0 && token.value == "endif" {
            return Some(Warning::warning(token, "Unexpected `endif`–no open if".into()));
        }

        if let Some(current_token) = self.current_token {
            let current_arg_type = current_token.arg_type(self.token_arg_index);
            match current_arg_type {
                Some(arg_type) => {
                    match self.check_arg_type(arg_type, &token) {
                        Some(warning) => return Some(warning),
                        None => (),
                    };
                },
                None => return Some(Warning::error(token, format!("Too many arguments ({}) to command `{}`", self.token_arg_index + 1, current_token.name))),
            }
        }

        None
    }

    pub fn write_token(&mut self, token: &Word) -> Option<Warning> {
        if let Some(current_token) = self.current_token {
            if self.token_arg_index >= current_token.arg_len() {
                self.current_token = None;
                self.token_arg_index = 0;
            }
        }

        let lint_warning = self.lint_token(token);

        if self.next_is_const {
            self.seen_consts.insert(token.value.into());
            self.next_is_const = false;
        }

        if self.next_is_define {
            self.seen_defines.insert(token.value.into());
            self.next_is_define = false;
        }

        match token.value {
            "/*" => self.is_comment = true,
            "*/" => self.is_comment = false,
            "if" => self.if_depth += 1,
            "endif" => {
                if self.if_depth > 0 {
                    self.if_depth -= 1;
                }
            },
            "#const" => self.next_is_const = true,
            "#define" => self.next_is_define = true,
            _ => (),
        }

        if let Some(current_token) = self.current_token {
            self.token_arg_index += 1;
        }

        match TOKENS.get(token.value) {
            Some(ref token_type) => {
                self.current_token = Some(token_type);
                self.token_arg_index = 0;
            },
            None => (),
        }

        lint_warning
    }
}
