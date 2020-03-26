use crate::{Atom, AtomKind, Lint, ParseState, TokenContext, Warning, TOKENS};
use cow_utils::CowUtils;

#[derive(Default)]
pub struct IncorrectSectionLint {}

impl IncorrectSectionLint {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Lint for IncorrectSectionLint {
    fn name(&self) -> &'static str {
        "incorrect-section"
    }

    fn lint_atom(&mut self, state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Warning> {
        if let AtomKind::Command { name, .. } = atom.kind {
            let token_type = &TOKENS[name.value.cow_to_ascii_lowercase().as_ref()];
            if let TokenContext::Command(Some(expected_section)) = token_type.context() {
                match &state.current_section {
                    Some(current_section) => {
                        let section_name = match &current_section.kind {
                            AtomKind::Section { name } => name,
                            kind => unreachable!("Expected AtomKind::Section, got {:?}", kind),
                        };
                        if section_name.value != *expected_section {
                            return vec![atom
                                .error(format!(
                                    "Command is invalid in section {}, it can only appear in {}",
                                    section_name.value, expected_section
                                ))
                                .note_at(
                                    current_section.file,
                                    current_section.span,
                                    "Section started here",
                                )];
                        }
                    }
                    None => {
                        return vec![atom.error(format!("Command can only appear in section {}, but no section has been started.", expected_section))];
                    }
                }
            }
        }
        Default::default()
    }
}
