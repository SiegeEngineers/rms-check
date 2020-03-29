use crate::diagnostic::{Diagnostic, Label};
use crate::{Atom, AtomKind, Lint, ParseState, TokenContext, TOKENS};
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

    fn lint_atom(&mut self, state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Diagnostic> {
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
                            return vec![Diagnostic::error(
                                atom.location,
                                format_args!(
                                    "Command is invalid in section {}, it can only appear in {}",
                                    section_name.value, expected_section
                                ),
                            )
                            .add_label(Label::new(
                                current_section.location,
                                "Section started here",
                            ))];
                        }
                    }
                    None => {
                        return vec![Diagnostic::error(atom.location, format_args!("Command can only appear in section {}, but no section has been started.", expected_section))];
                    }
                }
            }
        }
        Default::default()
    }
}
