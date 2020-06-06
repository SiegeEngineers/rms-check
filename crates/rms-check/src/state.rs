//! State tracker while parsing AoE2 random map scripts.

use crate::diagnostic::{Diagnostic, Label, SourceLocation};
use crate::parser::{Atom, AtomKind, Parser};
use crate::tokenizer::Word;
use crate::tokens::TokenType;
use crate::RMSFile;
use cow_utils::CowUtils;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

/// The target compatibility for a map script.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum Compatibility {
    /// The Conquerors.
    Conquerors = 1,
    /// Target UserPatch 1.4, accept the features it added.
    UserPatch14 = 3,
    /// Target UserPatch 1.5, accept the features it added.
    UserPatch15 = 4,
    /// Target WololoKingdoms: use UserPatch 1.5, constants for HD Edition DLC units and terrains,
    /// and auto-use UserPatch-specific constants.
    WololoKingdoms = 5,
    /// Target HD Edition (assumes all DLCs).
    HDEdition = 2,
    /// Target Definitive Edition.
    DefinitiveEdition = 6,
    /// Try to be maximally compatible. This is basically the same as targeting Conquerors.
    All = 0,
}

impl Default for Compatibility {
    fn default() -> Compatibility {
        Compatibility::Conquerors
    }
}

/// Enum for the different atoms that introduce nested contexts.
#[derive(Debug, Clone)]
pub enum Nesting<'a> {
    /// An opening `if` statement.
    If(Atom<'a>),
    /// An `elseif` statement.
    ElseIf(Atom<'a>),
    /// An `else` statement.
    Else(Atom<'a>),
    /// An opening `start_random` statement.
    StartRandom(Atom<'a>),
    /// An opening `percent_chance` statement.
    PercentChance(Atom<'a>),
    /// An opening brace (`{`), introducing a block with attributes.
    Brace(Atom<'a>),
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
enum HeaderName {
    Compatibility,
}

impl FromStr for HeaderName {
    type Err = ();

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let lower_name = name.cow_to_ascii_lowercase();
        match lower_name.trim() {
            "compatibility" => Ok(HeaderName::Compatibility),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConstDefinition<'a> {
    atom: Atom<'a>,
    value: Option<Word<'a>>,
    // depends_on: Vec<ConstDependency>,
}

impl<'a> ConstDefinition<'a> {
    /// Get the name of this definition.
    pub fn name(&self) -> &'a str {
        match &self.atom.kind {
            AtomKind::Const { name, .. } => name.value,
            AtomKind::Define { name, .. } => name.value,
            _ => unreachable!(),
        }
    }

    /// Get the location where this const is defined.
    pub fn location(&self) -> SourceLocation {
        self.atom.location
    }
}

#[derive(Debug)]
pub struct ParseState<'a> {
    /// The files.
    pub rms: &'a RMSFile<'a>,
    /// The target compatibility for this map script.
    pub compatibility: Compatibility,
    /// Whether this map should be treated as a builtin map. If true, #include and #include_drs should be made available.
    pub is_builtin_map: bool,
    /// The amount of nested statements we entered, like `if`, `start_random`.
    pub nesting: Vec<Nesting<'a>>,
    /// The token type that we are currently reading arguments for.
    pub current_token: Option<&'static TokenType>,
    /// The current <SECTION>, as well as its opening token.
    pub current_section: Option<Atom<'a>>,
    /// List of builtin #const definitions.
    builtin_consts: HashSet<String>,
    /// List of builtin #define definitions.
    builtin_defines: HashSet<String>,
    /// List of user-mode #const definitions we've seen so far.
    consts: HashMap<&'a str, ConstDefinition<'a>>,
    /// List of user-mode #define definitions we've seen so far.
    defines: HashMap<&'a str, ConstDefinition<'a>>,
    /// List of builtin optional definitions.
    pub option_defines: HashSet<String>,
    /// Are we still parsing header comments?
    end_of_headers: bool,
}

impl<'a> ParseState<'a> {
    pub(crate) fn new(rms: &'a RMSFile<'a>, compatibility: Compatibility) -> Self {
        let mut state = Self {
            rms,
            compatibility,
            is_builtin_map: false,
            nesting: vec![],
            current_token: None,
            current_section: None,
            builtin_consts: HashSet::new(),
            builtin_defines: HashSet::new(),
            consts: HashMap::new(),
            defines: HashMap::new(),
            option_defines: HashSet::new(),
            end_of_headers: false,
        };
        state.set_compatibility(compatibility);
        state
    }

    /// Track that a `#define` name may or may not exist from this point.
    ///
    /// These defines are valid in `if` statements, but not in commands, for example.
    pub fn optional_define(&mut self, name: impl ToString) {
        self.option_defines.insert(name.to_string());
    }
    /// Track that a `#define` name exists.
    pub fn define(&mut self, definition: ConstDefinition<'a>) {
        self.defines.insert(definition.name(), definition);
    }
    /// Track that a `#const` name exists.
    pub fn define_const(&mut self, definition: ConstDefinition<'a>) {
        self.consts.insert(definition.name(), definition);
    }
    /// Does a given `#define` name exist?
    pub fn has_define(&self, name: &str) -> bool {
        self.defines.contains_key(name) || self.builtin_defines.contains(name)
    }
    /// May a given `#define` name exist at this point?
    pub fn may_have_define(&self, name: &str) -> bool {
        self.has_define(name) || self.option_defines.contains(name)
    }
    /// Does a given `#const` name exist?
    pub fn has_const(&self, name: &str) -> bool {
        self.consts.contains_key(name) || self.builtin_consts.contains(name)
    }
    /// List all the `#const` names that are currently available.
    pub fn consts(&self) -> impl Iterator<Item = &str> {
        self.consts
            .keys()
            .copied()
            .chain(self.builtin_consts.iter().map(|string| string.as_ref()))
    }
    /// List all the `#define` names that are currently available.
    pub fn defines(&self) -> impl Iterator<Item = &str> {
        self.defines
            .keys()
            .copied()
            .chain(self.builtin_defines.iter().map(|string| string.as_ref()))
    }

    pub fn get_define(&self, name: &str) -> Option<&ConstDefinition<'_>> {
        self.defines.get(name)
    }

    pub fn get_const(&self, name: &str) -> Option<&ConstDefinition<'_>> {
        self.consts.get(name)
    }

    /// Get the compatibility mode the parser runs in.
    pub const fn compatibility(&self) -> Compatibility {
        self.compatibility
    }

    /// Set the compatibility mode the parser should run in.
    ///
    /// This affects the available builtin `#define` and `#const` names.
    pub fn set_compatibility(&mut self, compatibility: Compatibility) {
        self.compatibility = compatibility;

        self.builtin_consts.clear();
        self.builtin_defines.clear();

        let (file_id, content) = self.rms.definitions(compatibility);

        for (atom, _) in Parser::new(file_id, content) {
            match atom.kind {
                AtomKind::Const { name, .. } => {
                    self.builtin_consts.insert(name.value.to_string());
                }
                AtomKind::Define { name, .. } => {
                    self.builtin_defines.insert(name.value.to_string());
                }
                _ => (),
            }
        }
    }

    /// Update the parse state upon reading a new Atom.
    pub(crate) fn update(&mut self, atom: &Atom<'a>) {
        self.update_headers(atom);

        match atom.kind {
            AtomKind::Section { .. } => {
                self.current_section = Some(atom.clone());
            }
            AtomKind::Define { .. } => {
                self.define(ConstDefinition {
                    atom: atom.clone(),
                    value: None,
                });
            }
            AtomKind::Const { value, .. } => {
                self.define_const(ConstDefinition {
                    atom: atom.clone(),
                    value,
                });
            }
            _ => (),
        }
    }

    fn set_header(&mut self, name: HeaderName, value: &str) {
        match name {
            HeaderName::Compatibility => {
                let lower_value = value.cow_to_ascii_lowercase();
                let compat = match lower_value.trim() {
                    "hd edition" | "hd" => Compatibility::HDEdition,
                    "conquerors" | "aoc" => Compatibility::Conquerors,
                    "userpatch 1.5" | "up 1.5" => Compatibility::UserPatch15,
                    "userpatch 1.4" | "up 1.4" | "userpatch" | "up" => Compatibility::UserPatch14,
                    "wololokingdoms" | "wk" => Compatibility::WololoKingdoms,
                    "definitive edition" | "de" => Compatibility::DefinitiveEdition,
                    _ => return,
                };
                self.set_compatibility(compat);
            }
        }
    }

    fn parse_header_comment(&mut self, content: &str) {
        for mut line in content.lines() {
            line = line.trim();
            if line.starts_with("* ") {
                line = &line[2..];
            }

            let mut parts = line.splitn(2, ": ");
            if let (Some(name), Some(val)) = (parts.next(), parts.next()) {
                if let Ok(header) = name.parse() {
                    self.set_header(header, val);
                }
            }
        }
    }

    fn update_headers(&mut self, atom: &Atom<'a>) {
        if self.end_of_headers {
            return;
        }
        if let AtomKind::Comment { content, .. } = &atom.kind {
            self.parse_header_comment(content);
        } else {
            self.end_of_headers = true;
        }
    }

    /// Update the nesting state upon reading a new Atom.
    pub(crate) fn update_nesting(&mut self, atom: &Atom<'a>) -> Option<Diagnostic> {
        fn unbalanced_error(name: &str, end: &Atom<'_>, nest: Option<&Nesting<'_>>) -> Diagnostic {
            let msg = format!("Unbalanced `{}`", name);
            match nest {
                Some(Nesting::Brace(start)) => Diagnostic::error(end.location, msg)
                    .add_label(Label::new(start.location, "Matches this open brace `{`")),
                Some(Nesting::If(start)) => Diagnostic::error(end.location, msg)
                    .add_label(Label::new(start.location, "Matches this `if`")),
                Some(Nesting::ElseIf(start)) => Diagnostic::error(end.location, msg)
                    .add_label(Label::new(start.location, "Matches this `elseif`")),
                Some(Nesting::Else(start)) => Diagnostic::error(end.location, msg)
                    .add_label(Label::new(start.location, "Matches this `else`")),
                Some(Nesting::StartRandom(start)) => Diagnostic::error(end.location, msg)
                    .add_label(Label::new(start.location, "Matches this `start_random`")),
                Some(Nesting::PercentChance(start)) => Diagnostic::error(end.location, msg)
                    .add_label(Label::new(start.location, "Matches this `percent_chance`")),
                None => Diagnostic::error(end.location, format_args!("{}–nothing is open", msg)),
            }
        }

        match atom.kind {
            AtomKind::OpenBlock { .. } => {
                self.nesting.push(Nesting::Brace(atom.clone()));
            }
            AtomKind::CloseBlock { .. } => match self.nesting.last() {
                Some(Nesting::Brace(_)) => {
                    self.nesting.pop();
                }
                nest => {
                    return Some(unbalanced_error("}", atom, nest));
                }
            },
            AtomKind::If { .. } => self.nesting.push(Nesting::If(atom.clone())),
            AtomKind::ElseIf { .. } => {
                match self.nesting.last() {
                    Some(Nesting::If(_)) | Some(Nesting::ElseIf(_)) => {
                        self.nesting.pop();
                    }
                    nest => {
                        return Some(unbalanced_error("elseif", atom, nest));
                    }
                }
                self.nesting.push(Nesting::ElseIf(atom.clone()));
            }
            AtomKind::Else { .. } => {
                match self.nesting.last() {
                    Some(Nesting::If(_)) | Some(Nesting::ElseIf(_)) => {
                        self.nesting.pop();
                    }
                    nest => {
                        return Some(unbalanced_error("else", atom, nest));
                    }
                }
                self.nesting.push(Nesting::Else(atom.clone()));
            }
            AtomKind::EndIf { .. } => match self.nesting.last() {
                Some(Nesting::If(_)) | Some(Nesting::ElseIf(_)) | Some(Nesting::Else(_)) => {
                    self.nesting.pop();
                }
                nest => {
                    return Some(unbalanced_error("endif", atom, nest));
                }
            },
            AtomKind::StartRandom { .. } => self.nesting.push(Nesting::StartRandom(atom.clone())),
            AtomKind::PercentChance { .. } => {
                if let Some(Nesting::PercentChance(_)) = self.nesting.last() {
                    self.nesting.pop();
                }

                match self.nesting.last() {
                    Some(Nesting::StartRandom(_)) => {}
                    nest => {
                        return Some(unbalanced_error("percent_chance", atom, nest));
                    }
                }

                self.nesting.push(Nesting::PercentChance(atom.clone()));
            }
            AtomKind::EndRandom { .. } => {
                if let Some(Nesting::PercentChance(_)) = self.nesting.last() {
                    self.nesting.pop();
                };

                match self.nesting.last() {
                    Some(Nesting::StartRandom(_)) => {
                        self.nesting.pop();
                    }
                    nest => {
                        return Some(unbalanced_error("end_random", atom, nest));
                    }
                }
            }
            _ => (),
        }

        None
    }
}
