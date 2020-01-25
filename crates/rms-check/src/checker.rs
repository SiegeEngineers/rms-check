use crate::parser::Atom;
use crate::state::{Compatibility, ParseState};
use crate::wordize::Word;
use crate::RMSFile;
use codespan::{ByteIndex, FileId, Span};
pub use codespan_reporting::diagnostic::{Diagnostic, Label, Severity};
use lazy_static::lazy_static;

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
    /// The file this suggestion applies to.
    file_id: FileId,
    /// The piece of source code that this suggestion would replace.
    span: Span,
    /// Human-readable suggestion message.
    message: String,
    /// A replacement string that could fix the problem.
    replacement: AutoFixReplacement,
}

impl Suggestion {
    /// Get the codespan file ID that would be updated by this suggestion.
    #[inline]
    pub const fn file_id(&self) -> FileId {
        self.file_id
    }
    /// Get the span this suggestion applies to.
    #[inline]
    pub const fn span(&self) -> Span {
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
    pub const fn replacement(&self) -> &AutoFixReplacement {
        &self.replacement
    }

    /// Create a suggestion.
    #[inline]
    pub fn new(file_id: FileId, span: Span, message: impl ToString) -> Self {
        let message = message.to_string();
        Suggestion {
            file_id,
            span,
            message,
            replacement: AutoFixReplacement::None,
        }
    }
    /// Create a suggestion applying to a specific token.
    #[inline]
    pub fn from(token: &Word<'_>, message: impl ToString) -> Self {
        let message = message.to_string();
        Suggestion {
            file_id: token.file,
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
    /// Get the diagnostic for this warning.
    #[inline]
    pub const fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }
    /// Get the severity of this warning.
    #[inline]
    pub const fn severity(&self) -> Severity {
        self.diagnostic.severity
    }
    /// Get additional labels for this warning.
    #[inline]
    pub const fn labels(&self) -> &Vec<Label> {
        &self.diagnostic.secondary_labels
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
    pub const fn suggestions(&self) -> &Vec<Suggestion> {
        &self.suggestions
    }

    /// Create a new warning with severity "Warning".
    #[allow(unused)]
    #[must_use]
    pub(crate) fn warning(file_id: FileId, span: Span, message: impl Into<String>) -> Self {
        let message: String = message.into();
        Warning {
            diagnostic: Diagnostic::new_warning(
                message.clone(),
                Label::new(file_id, span, message),
            ),
            suggestions: vec![],
        }
    }

    /// Create a new warning with severity "Error".
    #[must_use]
    pub(crate) fn error(file_id: FileId, span: Span, message: impl Into<String>) -> Self {
        let message: String = message.into();
        Warning {
            diagnostic: Diagnostic::new_error(message.clone(), Label::new(file_id, span, message)),
            suggestions: vec![],
        }
    }

    /// Define a replacement suggestion for this warning.
    pub(crate) fn suggest(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Add a note referencing a snippet of code.
    pub(crate) fn note_at(mut self, file_id: FileId, span: Span, message: &str) -> Self {
        self.diagnostic = self
            .diagnostic
            .with_secondary_labels(vec![Label::new(file_id, span, message)]);
        self
    }

    /// Set the lint that emitted this warning.
    pub(crate) fn lint(mut self, lint: &str) -> Self {
        self.diagnostic = self.diagnostic.with_code(lint);
        self
    }
}

impl Word<'_> {
    /// Create a warning applying to this token.
    #[must_use]
    pub(crate) fn warning(&self, message: impl Into<String>) -> Warning {
        let message: String = message.into();
        Warning {
            diagnostic: Diagnostic::new_warning(
                message.clone(),
                Label::new(self.file, self.span, message),
            ),
            suggestions: vec![],
        }
    }
    /// Create an error applying to this token.
    #[must_use]
    pub(crate) fn error(&self, message: impl Into<String>) -> Warning {
        let message: String = message.into();
        Warning {
            diagnostic: Diagnostic::new_error(
                message.clone(),
                Label::new(self.file, self.span, message),
            ),
            suggestions: vec![],
        }
    }
}

impl Atom<'_> {
    /// Create a warning applying to this token.
    #[must_use]
    pub(crate) fn warning(&self, message: impl Into<String>) -> Warning {
        let message: String = message.into();
        Warning {
            diagnostic: Diagnostic::new_warning(
                message.clone(),
                Label::new(self.file, self.span, message),
            ),
            suggestions: vec![],
        }
    }
    /// Create an error applying to this token.
    #[must_use]
    pub(crate) fn error(&self, message: impl Into<String>) -> Warning {
        let message: String = message.into();
        Warning {
            diagnostic: Diagnostic::new_error(
                message.clone(),
                Label::new(self.file, self.span, message),
            ),
            suggestions: vec![],
        }
    }
}

pub trait Lint {
    fn name(&self) -> &'static str;
    fn run_inside_comments(&self) -> bool {
        false
    }
    fn lint_atom(&mut self, _state: &mut ParseState<'_>, _atom: &Atom<'_>) -> Vec<Warning> {
        Default::default()
    }
}

/// Builtin #define or #const names for AoE2: The Age of Conquerors.
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
    /// Builtin #define or #const names for UserPatch.
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

#[derive(Default)]
pub struct CheckerBuilder {
    lints: Vec<Box<dyn Lint>>,
    compatibility: Compatibility,
}

impl CheckerBuilder {
    pub fn build(self, rms: &RMSFile) -> Checker<'_> {
        // Default to UP 1.5 if it's a ZR@ map
        let compatibility = if rms.is_zip_rms() && self.compatibility < Compatibility::UserPatch15 {
            Compatibility::UserPatch15
        } else {
            self.compatibility
        };

        let state = ParseState::new(rms, compatibility);
        Checker {
            lints: self.lints,
            state,
        }
    }

    pub fn with_lint(mut self, lint: Box<dyn Lint>) -> Self {
        self.lints.push(lint);
        self
    }

    pub const fn compatibility(mut self, compatibility: Compatibility) -> Self {
        self.compatibility = compatibility;
        self
    }
}

pub struct Checker<'a> {
    lints: Vec<Box<dyn Lint>>,
    state: ParseState<'a>,
}

impl<'a> Checker<'a> {
    pub fn builder() -> CheckerBuilder {
        CheckerBuilder::default()
    }

    pub fn write_atom(&mut self, atom: &Atom<'a>) -> Vec<Warning> {
        let mut state = &mut self.state;
        let mut warnings = vec![];
        for lint in self.lints.iter_mut() {
            let new_warnings = lint
                .lint_atom(&mut state, atom)
                .into_iter()
                .map(move |warning| warning.lint(lint.name()));
            warnings.extend(new_warnings);
        }

        self.state.update(atom);
        if let Some(nest_warning) = self.state.update_nesting(atom) {
            warnings.push(nest_warning);
        }

        warnings
    }
}
