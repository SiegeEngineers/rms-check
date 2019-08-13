use crate::{
    parser::{Atom, Parser},
    tokens::{TokenType, TOKENS},
    wordize::Word,
};
use codespan::{ByteIndex, ByteSpan, CodeMap, FileName};
pub use codespan_reporting::{Diagnostic, Label, Severity};
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum Compatibility {
    Conquerors = 1,
    UserPatch14 = 3,
    UserPatch15 = 4,
    WololoKingdoms = 5,
    HDEdition = 2,
    All = 0,
}

impl Default for Compatibility {
    #[inline]
    fn default() -> Compatibility {
        Compatibility::Conquerors
    }
}

/// Describes the next expected token.
#[derive(Debug, Clone, Copy)]
enum Expect<'a> {
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
    #[inline]
    fn default() -> Self {
        Expect::None
    }
}

#[derive(Debug, Clone)]
pub enum AutoFixReplacement {
    None,
    Safe(String),
    Unsafe(String),
}

impl AutoFixReplacement {
    #[inline]
    pub fn is_fixable(&self) -> bool {
        match self {
            AutoFixReplacement::Safe(_) => true,
            _ => false,
        }
    }
    #[inline]
    pub fn is_fixable_unsafe(&self) -> bool {
        match self {
            AutoFixReplacement::None => false,
            _ => true,
        }
    }
}

/// A suggestion that may fix a warning.
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// The piece of source code that this suggestion would replace.
    span: ByteSpan,
    /// Human-readable suggestion message.
    message: String,
    /// A replacement string that could fix the problem.
    replacement: AutoFixReplacement,
}

impl Suggestion {
    #[inline]
    pub fn span(&self) -> ByteSpan {
        self.span
    }
    /// Get the starting position this suggestion applies to.
    #[inline]
    pub fn start(&self) -> ByteIndex {
        self.span.start()
    }
    /// Get the end position this suggestion applies to.
    #[inline]
    pub fn end(&self) -> ByteIndex {
        self.span.end()
    }
    /// Get the suggestion message.
    #[inline]
    pub fn message(&self) -> &str {
        &self.message
    }
    /// Get the replacement string that could fix the problem.
    #[inline]
    pub fn replacement(&self) -> &AutoFixReplacement {
        &self.replacement
    }

    /// Create a suggestion.
    #[inline]
    pub fn new(start: ByteIndex, end: ByteIndex, message: impl ToString) -> Self {
        let message = message.to_string();
        Suggestion {
            span: ByteSpan::new(start, end),
            message,
            replacement: AutoFixReplacement::None,
        }
    }
    /// Create a suggestion applying to a specific token.
    #[inline]
    pub fn from(token: &Word<'_>, message: impl ToString) -> Self {
        let message = message.to_string();
        Suggestion {
            span: token.span,
            message,
            replacement: AutoFixReplacement::None,
        }
    }
    /// Specify a possible fix for the problem.
    #[inline]
    pub fn replace(mut self, replacement: impl ToString) -> Self {
        self.replacement = AutoFixReplacement::Safe(replacement.to_string());
        self
    }
    /// Specify a possible fix for the problem, but one that may not be correct and requires some
    /// manual intervention.
    #[inline]
    pub fn replace_unsafe(mut self, replacement: impl ToString) -> Self {
        self.replacement = AutoFixReplacement::Unsafe(replacement.to_string());
        self
    }
}

/// A warning.
#[derive(Debug, Clone)]
pub struct Warning {
    diagnostic: Diagnostic,
    /// A change suggestion: when present, the problem can be fixed by replacing the
    /// range of text this warning applies to by the string in this suggestion.
    suggestions: Vec<Suggestion>,
}

impl Warning {
    #[inline]
    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }
    /// Get the severity of this warning.
    #[inline]
    pub fn severity(&self) -> Severity {
        self.diagnostic.severity
    }
    #[inline]
    pub fn labels(&self) -> &Vec<Label> {
        &self.diagnostic.labels
    }
    /// Get the human-readable error message.
    #[inline]
    pub fn message(&self) -> &str {
        &self.diagnostic.message
    }
    /// Check whether any suggestions could be made.
    #[inline]
    pub fn has_suggestions(&self) -> bool {
        !self.suggestions.is_empty()
    }
    /// Get any suggestions that may help to fix the problem.
    #[inline]
    pub fn suggestions(&self) -> &Vec<Suggestion> {
        &self.suggestions
    }

    /// Create a new warning with severity "Warning".
    #[allow(unused)]
    pub(crate) fn warning<S: AsRef<str>>(span: ByteSpan, message: S) -> Self {
        Warning {
            diagnostic: Diagnostic::new_warning(message.as_ref().to_string())
                .with_label(Label::new_primary(span)),
            suggestions: vec![],
        }
    }

    /// Create a new warning with severity "Error".
    pub(crate) fn error<S: AsRef<str>>(span: ByteSpan, message: S) -> Self {
        Warning {
            diagnostic: Diagnostic::new_error(message.as_ref().to_string())
                .with_label(Label::new_primary(span)),
            suggestions: vec![],
        }
    }

    /// Define a replacement suggestion for this warning.
    pub(crate) fn suggest(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Add a note referencing a snippet of code.
    pub(crate) fn note_at(mut self, span: ByteSpan, message: &str) -> Self {
        self.diagnostic = self
            .diagnostic
            .with_label(Label::new_secondary(span).with_message(message));
        self
    }

    pub(crate) fn lint(mut self, lint: &str) -> Self {
        self.diagnostic = self.diagnostic.with_code(lint);
        self
    }
}

impl Word<'_> {
    /// Create a warning applying to this token.
    pub(crate) fn warning<S: AsRef<str>>(&self, message: S) -> Warning {
        Warning {
            diagnostic: Diagnostic::new_warning(message.as_ref().to_string())
                .with_label(Label::new_primary(self.span)),
            suggestions: vec![],
        }
    }
    /// Create an error applying to this token.
    pub(crate) fn error<S: AsRef<str>>(&self, message: S) -> Warning {
        Warning {
            diagnostic: Diagnostic::new_error(message.as_ref().to_string())
                .with_label(Label::new_primary(self.span)),
            suggestions: vec![],
        }
    }
}

impl Atom<'_> {
    /// Create a warning applying to this token.
    pub(crate) fn warning<S: AsRef<str>>(&self, message: S) -> Warning {
        Warning {
            diagnostic: Diagnostic::new_warning(message.as_ref().to_string())
                .with_label(Label::new_primary(self.span())),
            suggestions: vec![],
        }
    }
    /// Create an error applying to this token.
    pub(crate) fn error<S: AsRef<str>>(&self, message: S) -> Warning {
        Warning {
            diagnostic: Diagnostic::new_error(message.as_ref().to_string())
                .with_label(Label::new_primary(self.span())),
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

#[derive(Debug, Clone)]
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
    fn lint_token(&mut self, _state: &mut ParseState<'_>, _token: &Word<'_>) -> Option<Warning> {
        Default::default()
    }
    fn lint_atom(&mut self, _state: &mut ParseState<'_>, _atom: &Atom<'_>) -> Vec<Warning> {
        Default::default()
    }
}

#[derive(Debug, Default, Clone)]
pub struct ParseState<'a> {
    /// The target compatibility for this map script.
    pub compatibility: Compatibility,
    /// Whether this map should be treated as a builtin map. If true, #include and #include_drs should be made available.
    pub is_builtin_map: bool,
    /// Whether we're currently inside a comment.
    pub is_comment: bool,
    /// The amount of nested statements we entered, like `if`, `start_random`.
    pub nesting: Vec<Nesting>,
    /// The token type that we are currently reading arguments for.
    pub current_token: Option<&'static TokenType>,
    /// The amount of arguments we've read.
    pub token_arg_index: u8,
    /// The type of token we expect to see next.
    expect: Expect<'a>,
    /// The current <SECTION>, as well as its opening token.
    pub current_section: Option<Atom<'a>>,
    /// List of builtin #const definitions.
    builtin_consts: HashSet<String>,
    /// List of builtin #define definitions.
    builtin_defines: HashSet<String>,
    /// List of user-mode #const definitions we've seen so far.
    pub seen_consts: HashSet<String>,
    /// List of user-mode #define definitions we've seen so far.
    pub seen_defines: HashSet<String>,
    /// List of builtin optional definitions.
    pub option_defines: HashSet<String>,
}

fn get_builtin_consts(compatibility: Compatibility) -> (HashSet<String>, HashSet<String>) {
    let mut consts = HashSet::new();
    let mut defines = HashSet::new();

    let mut codemap = CodeMap::new();
    match compatibility {
        Compatibility::WololoKingdoms => {
            codemap.add_filemap(
                FileName::virtual_("random_map.def"),
                include_str!("def_wk.rms"),
            );
        }
        Compatibility::HDEdition => {
            codemap.add_filemap(
                FileName::virtual_("random_map.def"),
                include_str!("def_hd.rms"),
            );
        },
        Compatibility::UserPatch15 => {
            codemap.add_filemap(
                FileName::virtual_("random_map.def"),
                include_str!("def_aoc.rms"),
            );
            codemap.add_filemap(
                FileName::virtual_("UserPatchConst.rms"),
                include_str!("def_up15.rms"),
            );
        }
        _ => {
            codemap.add_filemap(
                FileName::virtual_("random_map.def"),
                include_str!("def_aoc.rms"),
            );
        }
    };

    for filemap in codemap.iter() {
        for (atom, _) in Parser::new(filemap) {
            match atom {
                Atom::Const(_, name, _) => {
                    consts.insert(name.value.to_string());
                }
                Atom::Define(_, name) => {
                    defines.insert(name.value.to_string());
                }
                _ => (),
            }
        }
    }

    (consts, defines)
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
        self.seen_defines.contains(name) || self.builtin_defines.contains(name)
    }
    pub fn may_have_define(&self, name: &str) -> bool {
        self.has_define(name) || self.option_defines.contains(name)
    }
    pub fn has_const(&self, name: &str) -> bool {
        self.seen_consts.contains(name) || self.builtin_consts.contains(name)
    }
    pub fn compatibility(&self) -> Compatibility {
        self.compatibility
    }
    pub fn set_compatibility(&mut self, compatibility: Compatibility) {
        self.compatibility = compatibility;
        let (consts, defines) = get_builtin_consts(compatibility);
        self.builtin_consts = consts;
        self.builtin_defines = defines;
    }
    fn expect(&mut self, expect: Expect<'a>) {
        self.expect = expect;
    }
}

#[derive(Default)]
pub struct Checker<'a> {
    lints: HashMap<String, Box<dyn Lint>>,
    state: ParseState<'a>,
}

/// Builtin #define or #const names.
const AOC_OPTION_DEFINES: [&str; 8] = [
    "TINY_MAP",
    "SMALL_MAP",
    "MEDIUM_MAP",
    "LARGE_MAP",
    "HUGE_MAP",
    "GIGANTIC_MAP",
    "UP_AVAILABLE",
    "UP_EXTENSION",
];

lazy_static! {
    static ref UP_OPTION_DEFINES: Vec<String> = {
        let mut list = vec![
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
        if self.state.compatibility == Compatibility::UserPatch15 {
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

    pub fn compatibility(mut self, compatibility: Compatibility) -> Self {
        if self.state.compatibility != compatibility {
            self.state.set_compatibility(compatibility);
        }
        self
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

        None
    }

    pub fn write_atom(&mut self, atom: &Atom<'a>) -> Vec<Warning> {
        let mut state = &mut self.state;
        let mut warnings = vec![];
        for (name, lint) in self.lints.iter_mut() {
            warnings.extend(
                lint.lint_atom(&mut state, atom)
                    .into_iter()
                    .map(move |warning| warning.lint(&name)),
            );
        }

        match atom {
            Atom::Section(_) => self.state.current_section = Some(atom.clone()),
            _ => (),
        }

        warnings
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

        fn unbalanced_error(name: &str, token: &Word<'_>, nest: Option<&Nesting>) -> Warning {
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
            "}" => match self.state.nesting.last() {
                Some(Nesting::Brace(_)) => {
                    self.state.nesting.pop();
                }
                nest => {
                    parse_error = Some(unbalanced_error("}", token, nest));
                }
            },
            "if" => self.state.nesting.push(Nesting::If(token.span)),
            "elseif" => {
                match self.state.nesting.last() {
                    Some(Nesting::If(_)) | Some(Nesting::ElseIf(_)) => {
                        self.state.nesting.pop();
                    }
                    nest => {
                        parse_error = Some(unbalanced_error("elseif", token, nest));
                    }
                }
                self.state.nesting.push(Nesting::ElseIf(token.span));
            }
            "else" => {
                match self.state.nesting.last() {
                    Some(Nesting::If(_)) | Some(Nesting::ElseIf(_)) => {
                        self.state.nesting.pop();
                    }
                    nest => {
                        parse_error = Some(unbalanced_error("else", token, nest));
                    }
                }
                self.state.nesting.push(Nesting::Else(token.span));
            }
            "endif" => match self.state.nesting.last() {
                Some(Nesting::If(_)) | Some(Nesting::ElseIf(_)) | Some(Nesting::Else(_)) => {
                    self.state.nesting.pop();
                }
                nest => {
                    parse_error = Some(unbalanced_error("endif", token, nest));
                }
            },
            "start_random" => self.state.nesting.push(Nesting::StartRandom(token.span)),
            "percent_chance" => {
                if let Some(Nesting::PercentChance(_)) = self.state.nesting.last() {
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
                if let Some(Nesting::PercentChance(_)) = self.state.nesting.last() {
                    self.state.nesting.pop();
                };

                match self.state.nesting.last() {
                    Some(Nesting::StartRandom(_)) => {
                        self.state.nesting.pop();
                    }
                    nest => {
                        parse_error = Some(unbalanced_error("end_random", token, nest));
                    }
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
        }

        // A parse error is more important than a lint warning, probably…
        // chances are they're related anyway.
        parse_error.or(lint_warning)
    }
}
