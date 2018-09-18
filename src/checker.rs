use std::collections::HashSet;
use wordize::{Pos, Word};
use tokens::{ArgType, TokenType, TOKENS};

#[derive(Clone, Copy)]
enum Expect<'a> {
    None,
    DefineName,
    ConstName,
    UnfinishedRnd(Pos, &'a str),
}
impl<'a> Default for Expect<'a> {
    fn default() -> Self {
        Expect::None
    }
}

/// Warning severity.
#[derive(Clone, Copy)]
pub enum Severity {
    /// This needs attention but may not be incorrect.
    Warning,
    /// This is 100% incorrect.
    Error,
}

/// A suggestion that may fix a warning.
pub struct Suggestion {
    start: Pos,
    end: Pos,
    message: String,
    replacement: Option<String>,
}

impl Suggestion {
    pub fn start(&self) -> &Pos {
        &self.start
    }
    pub fn end(&self) -> &Pos {
        &self.end
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn replacement(&self) -> &Option<String> {
        &self.replacement
    }
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
    suggestions: Vec<Suggestion>,
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
    pub fn has_suggestions(&self) -> bool {
        self.suggestions.len() > 0
    }
    pub fn suggestions(&self) -> &Vec<Suggestion> {
        &self.suggestions
    }

    /// Create a new warning with severity "Warning".
    fn warning(start: Pos, end: Pos, message: String) -> Self {
        Warning {
            severity: Severity::Warning,
            start,
            end,
            message,
            suggestions: vec![],
        }
    }

    /// Create a new warning with severity "Error".
    fn error(start: Pos, end: Pos, message: String) -> Self {
        Warning {
            severity: Severity::Error,
            start,
            end,
            message,
            suggestions: vec![],
        }
    }

    /// Define a replacement suggestion for this warning.
    fn suggest(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Print this warning to the screen.
    fn print(&self) -> () {
        eprintln!("({}:{}) {}", self.start.line(), self.start.column(), &self.message);
        for suggestion in &self.suggestions {
            eprintln!("  SUGGESTION {}", suggestion.message);
            match suggestion.replacement {
                Some(ref msg) => eprintln!("  ! Suggested replacement: {}", msg),
                None => (),
            }
        }
    }
}

impl<'a> Word<'a> {
    pub fn warning(&self, message: String) -> Warning {
        Warning::warning(self.start, self.end, message)
    }
    pub fn error(&self, message: String) -> Warning {
        Warning::error(self.start, self.end, message)
    }
}

#[derive(Default)]
pub struct Checker<'a> {
    is_comment: bool,
    if_depth: u32,
    current_token: Option<&'static TokenType>,
    token_arg_index: u8,
    expect: Expect<'a>,
    seen_consts: HashSet<String>,
    seen_defines: HashSet<String>,
}

const BUILTIN_NAMES: [&str; 8] = [
    "TINY_MAP",
    "SMALL_MAP",
    "MEDIUM_MAP",
    "LARGE_MAP",
    "HUGE_MAP",
    "GIGANTIC_MAP",

    "REGICIDE",
    "DEATH_MATCH",
];

impl<'a> Checker<'a> {
    /// Create an RMS syntax checker.
    pub fn new() -> Self {
        let mut seen_defines = HashSet::new();
        for name in BUILTIN_NAMES.iter() {
            seen_defines.insert((*name).into());
        }

        Checker {
            seen_defines,
            ..Default::default()
        }
    }

    fn check_ever_defined(&self, token: &Word) -> Option<Warning> {
        if !self.seen_defines.contains(token.value) {
            Some(token.warning(format!("Token `{}` is never defined, this condition will always fail", token.value)))
        } else {
            None
        }
    }

    fn check_defined_with_value(&self, token: &Word) -> Option<Warning> {
        // 1. Check if this may or may not be defined—else warn
        if !self.seen_consts.contains(token.value) {
            if self.seen_defines.contains(token.value) {
                // 2. Check if this has a value (is defined using #const)—else warn
                Some(token.warning(format!("Expected a valued token (defined using #const), got a valueless token `{}` (defined using #define)", token.value)))
            } else {
                Some(token.warning(format!("Token `{}` is never defined", token.value)))
            }
        } else {
            None
        }
    }

    fn check_arg_type(&mut self, arg_type: &ArgType, token: &Word<'a>) -> Option<Warning> {
        match arg_type {
            ArgType::Number => {
                // This may be a valued (#const) constant,
                self.check_defined_with_value(token).and_then(|_warn| {
                    // or a number (12, -35),
                    token.value.parse::<i32>()
                        .err()
                        .map(|_| {
                            let warn = token.warning(format!("Expected a number, but got {}", token.value));
                            if token.value.starts_with("(") {
                                warn.suggest(Suggestion {
                                    start: token.start,
                                    end: token.end,
                                    message: "Did you forget the rnd()?".into(),
                                    replacement: format!("rnd{}", token.value).into(),
                                })
                            } else {
                                warn
                            }
                        })
                }).and_then(|warn| {
                    // or rnd(\d+,\d+)
                    if token.value.starts_with("rnd(") && token.value.ends_with(")") && token.value[4..token.value.len() - 1].split(",").all(|part| part.parse::<i32>().is_ok()) {
                        None
                    } else if token.value.starts_with("rnd(") && token.value.ends_with(",") {
                        // probably "rnd(\d+, \d+)"
                        self.expect = Expect::UnfinishedRnd(token.start, token.value);
                        None
                    } else if token.value == "rnd" {
                        // probably "rnd (\d+,\d+)"
                        self.expect = Expect::UnfinishedRnd(token.start, token.value);
                        None
                    } else {
                        Some(warn)
                    }
                })
            },
            ArgType::Word => {
                token.value.parse::<i32>()
                    .ok()
                    .map(|_| token.warning(format!("Expected a word, but got a number {}. This uses the number as the constant *name*, so it may not do what you expect.", token.value)))
            },
            ArgType::OptionalToken => self.check_ever_defined(token),
            ArgType::Token => self.check_defined_with_value(token),
            _ => None,
        }
    }

    /// Check an incoming token.
    fn lint_token(&mut self, token: &Word<'a>) -> Option<Warning> {
        if token.value == "*/" && !self.is_comment {
            return Some(token.error("Unexpected closing `*/`".into()))
        }

        // "/**" does not work to open a comment
        if token.value.len() > 2 && token.value.starts_with("/*") {
            let warning = token.error("Incorrect comment: there must be a space after the opening /*".into());
            let (message, replacement) = if token.value.ends_with("*/") {
                ("Add spaces at the start and end of the comment".into(),
                 Some(format!("/* {} */", &token.value[2..token.value.len() - 2])))
            } else {
                ("Add a space after the /*".into(),
                 Some(format!("/* {}", &token.value[2..])))
            };
            return Some(warning.suggest(Suggestion {
                start: token.start,
                end: token.end,
                message,
                replacement,
            }));
        }

        // "**/" was probably meant to be a closing comment, but only <whitespace>*/ actually closes
        // comments.
        if token.value.len() > 2 && token.value.ends_with("*/") {
            return Some(token.warning("Possibly unclosed comment, */ must be preceded by whitespace".into())
                .suggest(Suggestion {
                    start: token.start,
                    end: token.end,
                    message: "Add a space before the */".into(),
                    replacement: Some(format!("{} */", &token.value[2..token.value.len() - 2])),
                }));
        }

        if self.if_depth == 0 && token.value == "endif" {
            return Some(token.warning("Unexpected `endif`–no open if".into()));
        }

        if token.value == "#include_drs" {
            return Some(token.warning("#include_drs can only be used by builtin maps".into())
                .suggest(Suggestion {
                    start: token.start,
                    end: token.end,
                    message: "Move the included file to the Random/ folder and #include it normally".into(),
                    replacement: None,
                }));
        }

        if token.value.starts_with("<") && token.value.ends_with(">") && !TOKENS.contains_key(token.value) {
            return Some(token.error(format!("Invalid section {}", token.value)));
        }

        if let Expect::UnfinishedRnd(pos, val) = self.expect {
            self.expect = Expect::None;
            return Some(Warning::error(pos, token.end, format!("Incorrect rnd() call")).suggest(Suggestion {
                start: pos,
                end: token.end,
                message: "rnd() must not contain spaces".into(),
                replacement: format!("{}{}", val, token.value).into(),
            }));
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
                None => return Some(token.error(format!("Too many arguments ({}) to command `{}`", self.token_arg_index + 1, current_token.name))),
            }
        }

        None
    }

    pub fn write_token(&mut self, token: &Word<'a>) -> Option<Warning> {
        if let Some(current_token) = self.current_token {
            if self.token_arg_index >= current_token.arg_len() {
                self.current_token = None;
                self.token_arg_index = 0;
            }
        }

        let lint_warning = self.lint_token(token);

        match self.expect {
            Expect::ConstName => {
                self.seen_consts.insert(token.value.into());
                self.expect = Expect::None;
            },
            Expect::DefineName => {
                self.seen_defines.insert(token.value.into());
                self.expect = Expect::None;
            },
            _ => (),
        }

        match token.value {
            "/*" => self.is_comment = true,
            "*/" => self.is_comment = false,
            _ => (),
        }

        // TODO check whether this should happen
        // Before UP1.5 a parser bug could cause things inside comments to be parsed
        if self.is_comment { return None }

        match token.value {
            "if" => self.if_depth += 1,
            "endif" => {
                if self.if_depth > 0 {
                    self.if_depth -= 1;
                }
            },
            "#const" => self.expect = Expect::ConstName,
            "#define" => self.expect = Expect::DefineName,
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
