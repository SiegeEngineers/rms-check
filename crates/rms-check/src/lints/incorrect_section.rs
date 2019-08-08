use crate::{Atom, Lint, ParseState, TokenContext, Warning, TOKENS};

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
        if let Atom::Command(cmd, _) = atom {
            let token_type = &TOKENS[&cmd.value.to_lowercase()];
            if let TokenContext::Command(Some(expected_section)) = token_type.context() {
                match state.current_section {
                    Some(ref current_section) => {
                        let name = match current_section {
                            Atom::Section(ref name) => name,
                            _ => unreachable!(),
                        };
                        if name.value != *expected_section {
                            return vec![atom
                                .error(format!(
                                    "Command is invalid in section {}, it can only appear in {}",
                                    name.value, expected_section
                                ))
                                .note_at(current_section.span(), "Section started here")];
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
