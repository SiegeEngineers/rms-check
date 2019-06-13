use crate::tokens::{ArgType, TokenContext, TokenType, TOKENS};
use crate::wordize::Word;
use codespan::{ByteIndex, ByteSpan};
pub use codespan_reporting::{Diagnostic, Label, Severity};
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use strsim::levenshtein;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Compatibility {
    Conquerors,
    UserPatch14,
    UserPatch15,
    HDEdition,
    All,
}

impl Default for Compatibility {
    fn default() -> Compatibility {
        Compatibility::Conquerors
    }
}

/// Describes the next expected token.
#[derive(Clone, Copy)]
pub enum Expect<'a> {
    /// No expectations!
    None,
    /// A #define name.
    DefineName,
    /// A #const name.
    ConstName,
    /// The second part of an incorrectly formatted `rnd(A,B)` call.
    UnfinishedRnd(ByteIndex, &'a str),
}
impl<'a> Default for Expect<'a> {
    fn default() -> Self {
        Expect::None
    }
}

pub enum AutoFixReplacement {
    None,
    Safe(String),
    Unsafe(String),
}

impl AutoFixReplacement {
    pub fn is_fixable(&self) -> bool {
        match self {
            AutoFixReplacement::Safe(_) => true,
            _ => false,
        }
    }
    pub fn is_fixable_unsafe(&self) -> bool {
        match self {
            AutoFixReplacement::None => false,
            _ => true,
        }
    }
}

/// A suggestion that may fix a warning.
pub struct Suggestion {
    /// The piece of source code that this suggestion would replace.
    span: ByteSpan,
    /// Human-readable suggestion message.
    message: String,
    /// A replacement string that could fix the problem.
    replacement: AutoFixReplacement,
}

impl Suggestion {
    /// Get the starting position this suggestion applies to.
    pub fn start(&self) -> ByteIndex {
        self.span.start()
    }
    /// Get the end position this suggestion applies to.
    pub fn end(&self) -> ByteIndex {
        self.span.end()
    }
    /// Get the suggestion message.
    pub fn message(&self) -> &str {
        &self.message
    }
    /// Get the replacement string that could fix the problem.
    pub fn replacement(&self) -> &AutoFixReplacement {
        &self.replacement
    }

    /// Create a suggestion.
    pub fn new(start: ByteIndex, end: ByteIndex, message: impl ToString) -> Self {
        let message = message.to_string();
        Suggestion {
            span: ByteSpan::new(start, end),
            message,
            replacement: AutoFixReplacement::None,
        }
    }
    /// Create a suggestion applying to a specific token.
    pub fn from(token: &Word, message: impl ToString) -> Self {
        let message = message.to_string();
        Suggestion {
            span: token.span,
            message,
            replacement: AutoFixReplacement::None,
        }
    }
    /// Specify a possible fix for the problem.
    pub fn replace(mut self, replacement: impl ToString) -> Self {
        self.replacement = AutoFixReplacement::Safe(replacement.to_string());
        self
    }
    /// Specify a possible fix for the problem, but one that may not be correct and requires some
    /// manual intervention.
    pub fn replace_unsafe(mut self, replacement: impl ToString) -> Self {
        self.replacement = AutoFixReplacement::Unsafe(replacement.to_string());
        self
    }
}

/// A warning.
pub struct Warning {
    diagnostic: Diagnostic,
    /// A change suggestion: when present, the problem can be fixed by replacing the
    /// range of text this warning applies to by the string in this suggestion.
    suggestions: Vec<Suggestion>,
}

impl Warning {
    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }
    /// Get the severity of this warning.
    pub fn severity(&self) -> Severity {
        self.diagnostic.severity
    }
    pub fn labels(&self) -> &Vec<Label> {
        &self.diagnostic.labels
    }
    /// Get the human-readable error message.
    pub fn message(&self) -> &str {
        &self.diagnostic.message
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
    #[allow(unused)]
    pub fn warning<S: AsRef<str>>(span: ByteSpan, message: S) -> Self {
        Warning {
            diagnostic: Diagnostic::new_warning(message.as_ref().to_string())
                .with_label(Label::new_primary(span)),
            suggestions: vec![],
        }
    }

    /// Create a new warning with severity "Error".
    pub fn error<S: AsRef<str>>(span: ByteSpan, message: S) -> Self {
        Warning {
            diagnostic: Diagnostic::new_error(message.as_ref().to_string())
                .with_label(Label::new_primary(span)),
            suggestions: vec![],
        }
    }

    /// Define a replacement suggestion for this warning.
    pub fn suggest(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Add a note referencing a snippet of code.
    pub fn note_at(mut self, span: ByteSpan, message: &str) -> Self {
        self.diagnostic = self
            .diagnostic
            .with_label(Label::new_secondary(span).with_message(message));
        self
    }

    fn lint(mut self, lint: &str) -> Self {
        self.diagnostic = self.diagnostic.with_code(lint);
        self
    }
}

impl<'a> Word<'a> {
    /// Create a warning applying to this token.
    pub fn warning<S: AsRef<str>>(&self, message: S) -> Warning {
        Warning {
            diagnostic: Diagnostic::new_warning(message.as_ref().to_string())
                .with_label(Label::new_primary(self.span)),
            suggestions: vec![],
        }
    }
    /// Create an error applying to this token.
    pub fn error<S: AsRef<str>>(&self, message: S) -> Warning {
        Warning {
            diagnostic: Diagnostic::new_error(message.as_ref().to_string())
                .with_label(Label::new_primary(self.span)),
            suggestions: vec![],
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

fn meant<'a>(actual: &str, possible: impl Iterator<Item = &'a String>) -> Option<&'a String> {
    let mut lowest = 10000;
    let mut result = None;

    for expected in possible {
        let lev = levenshtein(actual, expected);
        if lev < lowest {
            result = Some(expected);
            lowest = lev;
        }
    }

    result
}

#[derive(Debug)]
pub enum Nesting {
    If(ByteSpan),
    ElseIf(ByteSpan),
    Else(ByteSpan),
    StartRandom(ByteSpan),
    PercentChance(ByteSpan),
    Brace(ByteSpan),
}

pub trait Lint {
    fn name(&self) -> &'static str;
    fn run_inside_comments(&self) -> bool {
        false
    }
    fn lint_token(&mut self, state: &mut ParseState, token: &Word) -> Option<Warning>;
}

#[derive(Default)]
pub struct ParseState<'a> {
    /// Whether we're currently inside a comment.
    pub is_comment: bool,
    /// The amount of nested statements we entered, like `if`, `start_random`.
    pub nesting: Vec<Nesting>,
    /// The token type that we are currently reading arguments for.
    pub current_token: Option<&'static TokenType>,
    /// The amount of arguments we've read.
    pub token_arg_index: u8,
    /// The type of token we expect to see next.
    pub expect: Expect<'a>,
    /// The current <SECTION>, as well as its opening token.
    pub current_section: Option<(Word<'a>, &'static str)>,
    /// List of #const definitions we've seen so far.
    pub seen_consts: HashSet<String>,
    /// List of #define definitions we've seen so far.
    pub seen_defines: HashSet<String>,
    /// List of builtin optional definitions.
    pub option_defines: HashSet<String>,
}

impl<'a> ParseState<'a> {
    pub fn optional_define(&mut self, name: impl ToString) {
        self.option_defines.insert(name.to_string());
    }
    pub fn define(&mut self, name: impl ToString) {
        self.seen_defines.insert(name.to_string());
    }
    pub fn define_const(&mut self, name: impl ToString) {
        self.seen_consts.insert(name.to_string());
    }
    pub fn has_define(&self, name: &str) -> bool {
        self.seen_defines.contains(name)
    }
    pub fn may_have_define(&self, name: &str) -> bool {
        self.has_define(name) || self.option_defines.contains(name)
    }
    pub fn has_const(&self, name: &str) -> bool {
        self.seen_consts.contains(name)
    }
    pub fn expect(&mut self, expect: Expect<'a>) {
        self.expect = expect;
    }
}

#[derive(Default)]
pub struct Checker<'a> {
    compatibility: Compatibility,
    lints: HashMap<String, Box<dyn Lint>>,
    state: ParseState<'a>,
}

/// Builtin #define or #const names.
const AOC_OPTION_DEFINES: [&str; 6] = [
    "TINY_MAP",
    "SMALL_MAP",
    "MEDIUM_MAP",
    "LARGE_MAP",
    "HUGE_MAP",
    "GIGANTIC_MAP",
];

lazy_static! {
    static ref UP_OPTION_DEFINES: Vec<String> = {
        let mut list = vec![
            "UP_EXTENSION".to_string(),
            "FIXED_POSITIONS".to_string(),
            "AI_PLAYERS".to_string(),
            "CAPTURE_RELIC".to_string(),
            "DEATH_MATCH".to_string(),
            "DEFEND_WONDER".to_string(),
            "KING_OT_HILL".to_string(),
            "RANDOM_MAP".to_string(),
            "REGICIDE".to_string(),
            "TURBO_RANDOM_MAP".to_string(),
            "WONDER_RACE".to_string(),
        ];

        for i in 1..=8 {
            list.push(format!("{}_PLAYER_GAME", i));
        }
        for i in 0..=4 {
            list.push(format!("{}_TEAM_GAME", i));
        }
        for team in 0..=4 {
            for player in 1..=8 {
                list.push(format!("PLAYER{}_TEAM{}", player, team));
            }
        }
        for team in 0..=4 {
            for size in 0..=8 {
                list.push(format!("TEAM{}_SIZE{}", team, size));
            }
        }

        list
    };
}

impl<'a> Checker<'a> {
    pub fn build(mut self) -> Self {
        if self.compatibility == Compatibility::UserPatch15 {
            for name in UP_OPTION_DEFINES.iter() {
                self.state.optional_define(name);
            }
        }

        for name in AOC_OPTION_DEFINES.iter() {
            self.state.optional_define(name);
        }

        self
    }

    pub fn with_lint(mut self, lint: Box<dyn Lint>) -> Self {
        self.lints.insert(lint.name().to_string(), lint);
        self
    }

    pub fn compatibility(self, compatibility: Compatibility) -> Self {
        Checker {
            compatibility,
            ..self
        }
    }

    /// Check if a constant was ever defined using #define.
    fn check_ever_defined(&self, token: &Word) -> Option<Warning> {
        if !self.state.may_have_define(token.value) {
            let warn = token.warning(format!(
                "Token `{}` is never defined, this condition will always fail",
                token.value
            ));
            Some(
                if let Some(similar) = meant(token.value, self.state.seen_defines.iter()) {
                    warn.suggest(
                        Suggestion::from(token, format!("Did you mean `{}`?", similar))
                            .replace_unsafe(similar.to_string()),
                    )
                } else {
                    warn
                },
            )
        } else {
            None
        }
    }

    /// Check if a constant was ever defined with a value (using #const)
    fn check_defined_with_value(&self, token: &Word) -> Option<Warning> {
        // 1. Check if this may or may not be defined—else warn
        if !self.state.has_const(token.value) {
            if self.state.has_define(token.value) {
                // 2. Check if this has a value (is defined using #const)—else warn
                Some(token.warning(format!("Expected a valued token (defined using #const), got a valueless token `{}` (defined using #define)", token.value)))
            } else {
                let warn = token.warning(format!("Token `{}` is never defined", token.value));
                Some(
                    if let Some(similar) = meant(token.value, self.state.seen_consts.iter()) {
                        warn.suggest(
                            Suggestion::from(token, format!("Did you mean `{}`?", similar))
                                .replace_unsafe(similar.to_string()),
                        )
                    } else {
                        warn
                    },
                )
            }
        } else {
            None
        }
    }

    /// Check if a word is a valid number.
    fn check_number(&mut self, token: &Word<'a>) -> Option<Warning> {
        // This may be a valued (#const) constant,
        // or a number (12, -35),
        token
            .value
            .parse::<i32>()
            .err()
            .map(|_| {
                let TokenType { name, .. } = self.state.current_token.unwrap();
                let warn = token.error(format!(
                    "Expected a number argument to {}, but got {}",
                    name, token.value
                ));
                if token.value.starts_with('(') {
                    let (_, replacement) = is_valid_rnd(&format!("rnd{}", token.value));
                    warn.suggest(
                        Suggestion::from(token, "Did you forget the `rnd`?")
                            .replace(replacement.unwrap_or_else(|| format!("rnd{}", token.value))),
                    )
                } else {
                    warn
                }
            })
            .and_then(|warn| {
                // or rnd(\d+,\d+)
                if let (true, _) = is_valid_rnd(token.value) {
                    None
                } else if
                // probably "rnd(\d+, \d+)"
                (token.value.starts_with("rnd(") && token.value.ends_with(',')) ||
                    // probably "rnd (\d+,\d+)"
                    (token.value == "rnd")
                {
                    self.state
                        .expect(Expect::UnfinishedRnd(token.start(), token.value));
                    None
                } else {
                    Some(warn)
                }
            })
    }

    /// Check if a token is the correct argument type.
    fn check_arg_type(&mut self, arg_type: ArgType, token: &Word<'a>) -> Option<Warning> {
        match arg_type {
            ArgType::Number => self.check_number(token),
            ArgType::Word => {
                token.value.parse::<i32>()
                    .ok()
                    .map(|_| token.warning(format!("Expected a word, but got a number {}. This uses the number as the constant *name*, so it may not do what you expect.", token.value)))
                    .or_else(|| if token.value.chars().any(char::is_lowercase) {
                        Some(token.warning("Using lowercase for constant names may cause confusion with attribute or command names.")
                             .suggest(Suggestion::from(token, "Use uppercase for constants.")
                                      .replace(token.value.to_uppercase())))
                    } else {
                        None
                    })
            },
            ArgType::OptionalToken => self.check_ever_defined(token),
            ArgType::Token => self.check_defined_with_value(token),
            _ => None,
        }
    }

    /// Check an incoming token.
    fn lint_token(&mut self, token: &Word<'a>) -> Option<Warning> {
        let warning = {
            let is_comment = self.state.is_comment;
            let mut state = &mut self.state;
            self.lints
                .iter_mut()
                .filter(|(_, lint)| !is_comment || lint.run_inside_comments())
                .find_map(|(name, lint)| {
                    lint.lint_token(&mut state, token)
                        .map(|warning| warning.lint(&name))
                })
        };
        if warning.is_some() {
            return warning;
        }

        if token.value.starts_with('<')
            && token.value.ends_with('>')
            && !TOKENS.contains_key(token.value)
        {
            return Some(token.error(format!("Invalid section {}", token.value)));
        }

        if let Some(current_token) = self.state.current_token {
            let current_arg_type = current_token.arg_type(self.state.token_arg_index);
            match current_arg_type {
                Some(arg_type) => {
                    if let Some(warning) = self.check_arg_type(*arg_type, &token) {
                        return Some(warning);
                    }
                }
                None => {
                    return Some(token.error(format!(
                        "Too many arguments ({}) to command `{}`",
                        self.state.token_arg_index + 1,
                        current_token.name
                    )))
                }
            }
        }

        None
    }

    /// Parse and lint the next token.
    pub fn write_token(&mut self, token: &Word<'a>) -> Option<Warning> {
        // Clear current token if we're past the end of its arguments list.
        if let Some(current_token) = self.state.current_token {
            if self.state.token_arg_index >= current_token.arg_len() {
                self.state.current_token = None;
                self.state.token_arg_index = 0;
            }
        }

        let mut parse_error = None;

        match self.state.expect {
            Expect::ConstName => {
                self.state.define_const(token.value);
                self.state.expect(Expect::None);
            }
            Expect::DefineName => {
                self.state.define(token.value);
                self.state.expect(Expect::None);
            }
            Expect::UnfinishedRnd(pos, val) => {
                let suggestion = Suggestion::new(pos, token.end(), "rnd() must not contain spaces");
                parse_error = Some(
                    Warning::error(ByteSpan::new(pos, token.end()), "Incorrect rnd() call")
                        .suggest(match is_valid_rnd(&format!("{} {}", val, token.value)).1 {
                            Some(replacement) => suggestion.replace(replacement),
                            None => suggestion,
                        }),
                );
                self.state.expect(Expect::None);
            }
            _ => (),
        }

        if token.value.starts_with("/*") {
            // Technically incorrect but the user most likely attempted to open a comment here,
            // so _not_ treating it as one would give lots of useless errors.
            // Instead we only mark this token as an incorrect comment.
            self.state.is_comment = true;
            if token.value.len() > 2 {
                let warning =
                    token.error("Incorrect comment: there must be a space after the opening /*");
                let (message, replacement) = if token.value.ends_with("*/") {
                    (
                        "Add spaces at the start and end of the comment",
                        format!("/* {} */", &token.value[2..token.value.len() - 2]),
                    )
                } else {
                    (
                        "Add a space after the /*",
                        format!("/* {}", &token.value[2..]),
                    )
                };
                parse_error =
                    Some(warning.suggest(Suggestion::from(token, message).replace(replacement)));
            }
        }

        let lint_warning = self.lint_token(token);

        if token.value.ends_with("*/") {
            if !self.state.is_comment {
                parse_error = Some(token.error("Unexpected closing `*/`"));
            } else {
                self.state.is_comment = false;
                // "**/" was probably meant to be a closing comment, but only <whitespace>*/ actually closes
                // comments.
                if token.value.len() > 2 && parse_error.is_none() {
                    parse_error = Some(
                        token
                            .error("Possibly unclosed comment, */ must be preceded by whitespace")
                            .suggest(
                                Suggestion::from(token, "Add a space before the */").replace(
                                    format!("{} */", &token.value[2..token.value.len() - 2]),
                                ),
                            ),
                    );
                }
            }
        }

        // TODO check whether this should happen
        // Before UP1.5 a parser bug could cause things inside comments to be parsed
        if self.state.is_comment {
            return parse_error.or(lint_warning);
        }

        fn unbalanced_error(name: &str, token: &Word, nest: Option<&Nesting>) -> Warning {
            let msg = format!("Unbalanced `{}`", name);
            match nest {
                Some(Nesting::Brace(loc)) => token
                    .error(msg)
                    .note_at(*loc, "Matches this open brace `{`"),
                Some(Nesting::If(loc)) => token.error(msg).note_at(*loc, "Matches this `if`"),
                Some(Nesting::ElseIf(loc)) => {
                    token.error(msg).note_at(*loc, "Matches this `elseif`")
                }
                Some(Nesting::Else(loc)) => token.error(msg).note_at(*loc, "Matches this `else`"),
                Some(Nesting::StartRandom(loc)) => token
                    .error(msg)
                    .note_at(*loc, "Matches this `start_random`"),
                Some(Nesting::PercentChance(loc)) => token
                    .error(msg)
                    .note_at(*loc, "Matches this `percent_chance`"),
                None => token.error(format!("{}–nothing is open", msg)),
            }
        }
        match token.value {
            "{" => self.state.nesting.push(Nesting::Brace(token.span)),
            "}" => {
                let mut is_ok = false;
                match self.state.nesting.last() {
                    Some(Nesting::Brace(_)) => {
                        is_ok = true;
                    }
                    nest => {
                        parse_error = Some(unbalanced_error("}", token, nest));
                    }
                }
                if is_ok {
                    self.state.nesting.pop();
                }
            }
            "if" => self.state.nesting.push(Nesting::If(token.span)),
            "elseif" => {
                let mut is_ok = false;
                match self.state.nesting.last() {
                    Some(Nesting::If(_)) | Some(Nesting::ElseIf(_)) => {
                        is_ok = true;
                    }
                    nest => {
                        parse_error = Some(unbalanced_error("elseif", token, nest));
                    }
                }
                if is_ok {
                    self.state.nesting.pop();
                }
                self.state.nesting.push(Nesting::ElseIf(token.span));
            }
            "else" => {
                let mut is_ok = false;
                match self.state.nesting.last() {
                    Some(Nesting::If(_)) | Some(Nesting::ElseIf(_)) => {
                        is_ok = true;
                    }
                    nest => {
                        parse_error = Some(unbalanced_error("else", token, nest));
                    }
                }
                if is_ok {
                    self.state.nesting.pop();
                }
                self.state.nesting.push(Nesting::Else(token.span));
            }
            "endif" => {
                let mut is_ok = false;
                match self.state.nesting.last() {
                    Some(Nesting::If(_)) | Some(Nesting::ElseIf(_)) | Some(Nesting::Else(_)) => {
                        is_ok = true;
                    }
                    nest => {
                        parse_error = Some(unbalanced_error("endif", token, nest));
                    }
                }
                if is_ok {
                    self.state.nesting.pop();
                }
            }
            "start_random" => self.state.nesting.push(Nesting::StartRandom(token.span)),
            "percent_chance" => {
                let is_sibling_branch = match self.state.nesting.last() {
                    Some(Nesting::PercentChance(_)) => true,
                    _ => false,
                };
                if is_sibling_branch {
                    self.state.nesting.pop();
                }

                match self.state.nesting.last() {
                    Some(Nesting::StartRandom(_)) => {}
                    nest => {
                        parse_error = Some(unbalanced_error("percent_chance", token, nest));
                    }
                }

                self.state.nesting.push(Nesting::PercentChance(token.span));
            }
            "end_random" => {
                let needs_double_close =
                    if let Some(Nesting::PercentChance(_)) = self.state.nesting.last() {
                        true
                    } else {
                        false
                    };
                if needs_double_close {
                    self.state.nesting.pop();
                }

                let mut is_ok = false;
                match self.state.nesting.last() {
                    Some(Nesting::StartRandom(_)) => {
                        is_ok = true;
                    }
                    nest => {
                        parse_error = Some(unbalanced_error("end_random", token, nest));
                    }
                }
                if is_ok {
                    self.state.nesting.pop();
                }
            }
            "#const" => self.state.expect(Expect::ConstName),
            "#define" => self.state.expect(Expect::DefineName),
            _ => (),
        }

        if self.state.current_token.is_some() {
            self.state.token_arg_index += 1;
        }

        if let Some(ref token_type) = TOKENS.get(token.value) {
            self.state.current_token = Some(token_type);
            self.state.token_arg_index = 0;

            if let TokenContext::Section = token_type.context() {
                self.state.current_section = Some((*token, token_type.name));
            }
        }

        // A parse error is more important than a lint warning, probably…
        // chances are they're related anyway.
        parse_error.or(lint_warning)
    }
}
