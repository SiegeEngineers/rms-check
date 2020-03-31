//! The checker that runs lints and keeps track of warnings.

use crate::diagnostic::Diagnostic;
use crate::parser::Atom;
use crate::state::{Compatibility, ParseState};
use crate::RMSFile;
use lazy_static::lazy_static;

pub trait Lint {
    fn name(&self) -> &'static str;
    fn run_inside_comments(&self) -> bool {
        false
    }
    fn lint_atom(&mut self, _state: &mut ParseState<'_>, _atom: &Atom<'_>) -> Vec<Diagnostic> {
        Default::default()
    }
}

/// Builtin #define or #const names for AoE2: The Age of Conquerors.
#[allow(dead_code)] // need to use this at some point?
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
    #[allow(dead_code)] // need to use this at some point?
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
    pub fn build<'source>(self, rms: &'source RMSFile<'source>) -> Checker<'source> {
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

    pub fn write_atom(&mut self, atom: &Atom<'a>) -> Vec<Diagnostic> {
        let mut state = &mut self.state;
        let mut warnings = vec![];
        for lint in self.lints.iter_mut() {
            let new_warnings = lint
                .lint_atom(&mut state, atom)
                .into_iter()
                .map(move |warning| warning.with_code(lint.name()));
            warnings.extend(new_warnings);
        }

        self.state.update(atom);
        if let Some(nest_warning) = self.state.update_nesting(atom) {
            warnings.push(nest_warning);
        }

        warnings
    }
}
