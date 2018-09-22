use std::collections::HashSet;
use wordize::{Pos, Range, Word};
use tokens::{ArgType, TokenType, TokenContext, TOKENS};

/// Describes the next expected token.
#[derive(Clone, Copy)]
enum Expect<'a> {
    /// No expectations!
    None,
    /// A #define name.
    DefineName,
    /// A #const name.
    ConstName,
    /// The second part of an incorrectly formatted `rnd(A,B)` call.
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
    /// The piece of source code that this suggestion would replace.
    range: Range,
    /// Human-readable suggestion message.
    message: String,
    /// A replacement string that could fix the problem.
    replacement: Option<String>,
}

impl Suggestion {
    /// Get the starting position this suggestion applies to.
    pub fn start(&self) -> Pos {
        self.range.start()
    }
    /// Get the end position this suggestion applies to.
    pub fn end(&self) -> Pos {
        self.range.end()
    }
    /// Get the suggestion message.
    pub fn message(&self) -> &str {
        &self.message
    }
    /// Get the replacement string that could fix the problem.
    pub fn replacement(&self) -> &Option<String> {
        &self.replacement
    }

    /// Create a suggestion.
    fn new(start: Pos, end: Pos, message: String) -> Self {
        Suggestion { range: Range(start, end), message, replacement: None }
    }
    /// Create a suggestion applying to a specific token.
    fn from(token: &Word, message: String) -> Self {
        Suggestion { range: token.range, message, replacement: None }
    }
    /// Specify a possible fix for the problem.
    fn replace(mut self, replacement: String) -> Self {
        self.replacement = Some(replacement);
        self
    }
}

pub struct Note {
    /// An optional range that this note applies to.
    range: Option<Range>,
    /// Human-readable text.
    message: String,
}

impl Note {
    /// Get the range of source code that this note applies to.
    pub fn range(&self) -> &Option<Range> {
        &self.range
    }
    /// Get the note text.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// A warning.
pub struct Warning {
    /// Warning severity.
    severity: Severity,
    /// The part of the source code that this warning applies to.
    range: Range,
    /// Human-readable warning text.
    message: String,
    /// Additional notes, hints, context, etc.
    notes: Vec<Note>,
    /// A change suggestion: when present, the problem can be fixed by replacing the
    /// range of text this warning applies to by the string in this suggestion.
    suggestions: Vec<Suggestion>,
}

impl Warning {
    /// Get the severity of this warning.
    pub fn severity(&self) -> Severity {
        self.severity
    }
    /// Get the range this warning applies to.
    pub fn range(&self) -> &Range {
        &self.range
    }
    /// Get the starting position this warning applies to.
    pub fn start(&self) -> Pos {
        self.range.start()
    }
    /// Get the end position this warning applies to.
    pub fn end(&self) -> Pos {
        self.range.end()
    }
    /// Get the human-readable error message.
    pub fn message(&self) -> &str {
        &self.message
    }
    /// Get any notes that may help clarify the problem or find a solution.
    pub fn notes(&self) -> &Vec<Note> {
        &self.notes
    }
    /// Check whether any suggestions could be made.
    pub fn has_suggestions(&self) -> bool {
        !self.suggestions.is_empty()
    }
    /// Get any suggestions that may help to fix the problem.
    pub fn suggestions(&self) -> &Vec<Suggestion> {
        &self.suggestions
    }

    /// Create a new warning with severity "Warning".
    fn warning(start: Pos, end: Pos, message: String) -> Self {
        Warning {
            severity: Severity::Warning,
            range: Range(start, end),
            message,
            notes: vec![],
            suggestions: vec![],
        }
    }

    /// Create a new warning with severity "Error".
    fn error(start: Pos, end: Pos, message: String) -> Self {
        Warning {
            severity: Severity::Error,
            range: Range(start, end),
            message,
            notes: vec![],
            suggestions: vec![],
        }
    }

    /// Define a replacement suggestion for this warning.
    fn suggest(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Add a note.
    fn note(mut self, message: &str) -> Self {
        self.notes.push(Note {
            range: None,
            message: message.into(),
        });
        self
    }

    /// Add a note referencing a snippet of code.
    fn note_at(mut self, range: Range, message: &str) -> Self {
        self.notes.push(Note {
            range: Some(range),
            message: message.into(),
        });
        self
    }
}

impl<'a> Word<'a> {
    /// Create a warning applying to this token.
    pub fn warning(&self, message: String) -> Warning {
        Warning::warning(self.start(), self.end(), message)
    }
    /// Create an error applying to this token.
    pub fn error(&self, message: String) -> Warning {
        Warning::error(self.start(), self.end(), message)
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
    if s.starts_with("rnd(") && s.ends_with(')') && s[4..s.len() - 1].split(",").all(is_numeric) {
        return (true, None)
    } else if s.chars().any(char::is_whitespace) {
        let no_ws = s.chars().filter(|c| !char::is_whitespace(*c)).collect::<String>();
        if let (true, _) = is_valid_rnd(&no_ws) {
            return (false, Some(no_ws))
        }
    }
    (false, None)
}

#[derive(Default)]
pub struct Checker<'a> {
    /// Whether we're currently inside a comment.
    is_comment: bool,
    /// The amount of nested `if` statements we entered.
    if_depth: u32,
    /// The token type that we are currently reading arguments for.
    current_token: Option<&'static TokenType>,
    /// The amount of arguments we've read.
    token_arg_index: u8,
    /// The type of token we expect to see next.
    expect: Expect<'a>,
    /// The current <SECTION>, as well as its opening token.
    current_section: Option<(Word<'a>, &'static str)>,
    /// List of #const definitions we've seen so far.
    seen_consts: HashSet<String>,
    /// List of #define definitions we've seen so far.
    seen_defines: HashSet<String>,
}

/// Builtin #define or #const names.
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

    /// Check if a constant was ever defined using #define.
    fn check_ever_defined(&self, token: &Word) -> Option<Warning> {
        if !self.seen_defines.contains(token.value) {
            Some(token.warning(format!("Token `{}` is never defined, this condition will always fail", token.value)))
        } else {
            None
        }
    }

    /// Check if a constant was ever defined with a value (using #const)
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

    /// Check if a token is the correct argument type.
    fn check_arg_type(&mut self, arg_type: &ArgType, token: &Word<'a>) -> Option<Warning> {
        match arg_type {
            ArgType::Number => {
                // This may be a valued (#const) constant,
                self.check_defined_with_value(token).and_then(|_warn| {
                    // or a number (12, -35),
                    token.value.parse::<i32>()
                        .err()
                        .map(|_| {
                            let TokenType { name, .. } = self.current_token.unwrap();
                            let warn = token.warning(format!("Expected a number argument to {}, but got {}", name, token.value));
                            if token.value.starts_with("(") {
                                let (_, replacement) = is_valid_rnd(&format!("rnd{}", token.value));
                                warn.suggest(
                                    Suggestion::from(token, "Did you forget the `rnd`?".into())
                                    .replace(replacement.unwrap_or_else(|| format!("rnd{}", token.value)))
                                )
                            } else {
                                warn
                            }
                        })
                }).and_then(|warn| {
                    // or rnd(\d+,\d+)
                    if let (true, _) = is_valid_rnd(token.value) {
                        None
                    } else if
                        // probably "rnd(\d+, \d+)"
                        (token.value.starts_with("rnd(") && token.value.ends_with(',')) ||
                        // probably "rnd (\d+,\d+)"
                        (token.value == "rnd") {
                        self.expect = Expect::UnfinishedRnd(token.start(), token.value);
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
        // "/**" does not work to open a comment
        if token.value.len() > 2 && token.value.starts_with("/*") {
            let warning = token.error("Incorrect comment: there must be a space after the opening /*".into());
            let (message, replacement) = if token.value.ends_with("*/") {
                ("Add spaces at the start and end of the comment".into(),
                 format!("/* {} */", &token.value[2..token.value.len() - 2]))
            } else {
                ("Add a space after the /*".into(),
                 format!("/* {}", &token.value[2..]))
            };
            return Some(warning.suggest(Suggestion::from(token, message).replace(replacement)));
        }

        // "**/" was probably meant to be a closing comment, but only <whitespace>*/ actually closes
        // comments.
        if token.value.len() > 2 && token.value.ends_with("*/") {
            return Some(token.warning("Possibly unclosed comment, */ must be preceded by whitespace".into())
                .suggest(Suggestion::from(token, "Add a space before the */".into())
                    .replace(format!("{} */", &token.value[2..token.value.len() - 2]))));
        }

        if token.value == "#include_drs" {
            return Some(token.warning("#include_drs can only be used by builtin maps".into())
                .suggest(Suggestion::from(token, "Move the included file to the Random/ folder and #include it normally".into())));
        }

        if token.value.starts_with('<') && token.value.ends_with('>') && !TOKENS.contains_key(token.value) {
            return Some(token.error(format!("Invalid section {}", token.value)));
        }

        if let Some(current_token) = self.current_token {
            if let TokenContext::Command(Some(expected_section)) = current_token.context() {
                match self.current_section {
                    Some((section_token, ref current_section)) => {
                        if current_section != expected_section {
                            return Some(token.error(format!("Command is invalid in section {}, it can only appear in {}", current_section, expected_section))
                                        .note_at(section_token.range, "Section started here"));
                        }
                    },
                    None => {
                        return Some(token.error(format!("Command can only appear in section {}, but no section has been started.", expected_section)));
                    }
                }
            }

            let current_arg_type = current_token.arg_type(self.token_arg_index);
            match current_arg_type {
                Some(arg_type) => {
                    if let Some(warning) = self.check_arg_type(arg_type, &token) {
                        return Some(warning);
                    }
                },
                None => return Some(token.error(format!("Too many arguments ({}) to command `{}`", self.token_arg_index + 1, current_token.name))),
            }
        }

        None
    }

    /// Parse and lint the next token.
    pub fn write_token(&mut self, token: &Word<'a>) -> Option<Warning> {
        if let Some(current_token) = self.current_token {
            if self.token_arg_index >= current_token.arg_len() {
                self.current_token = None;
                self.token_arg_index = 0;
            }
        }

        let lint_warning = self.lint_token(token);
        let mut parse_error = None;

        match self.expect {
            Expect::ConstName => {
                self.seen_consts.insert(token.value.into());
                self.expect = Expect::None;
            },
            Expect::DefineName => {
                self.seen_defines.insert(token.value.into());
                self.expect = Expect::None;
            },
            Expect::UnfinishedRnd(pos, val) => {
                let suggestion = Suggestion::new(pos, token.end(), "rnd() must not contain spaces".into());
                parse_error = Some(Warning::error(pos, token.end(), "Incorrect rnd() call".into()).suggest(
                    match is_valid_rnd(&format!("{} {}", val, token.value)).1 {
                        Some(replacement) => suggestion.replace(replacement),
                        None => suggestion,
                    }
                ));
                self.expect = Expect::None;
            },
            _ => (),
        }

        match token.value {
            "/*" => self.is_comment = true,
            "*/" => if !self.is_comment {
                parse_error = Some(token.error("Unexpected closing `*/`".into()));
            } else {
                self.is_comment = false
            },
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
                } else {
                    parse_error = Some(token.error("Unexpected `endif`–no open if".into()));
                }
            },
            "#const" => self.expect = Expect::ConstName,
            "#define" => self.expect = Expect::DefineName,
            _ => (),
        }

        if self.current_token.is_some() {
            self.token_arg_index += 1;
        }

        if let Some(ref token_type) = TOKENS.get(token.value) {
            self.current_token = Some(token_type);
            self.token_arg_index = 0;

            if let TokenContext::Section = token_type.context() {
                self.current_section = Some((token.clone(), token_type.name));
            }
        }

        parse_error.or(lint_warning)
    }
}
