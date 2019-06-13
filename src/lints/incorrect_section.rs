use super::super::{Lint, ParseState, TokenContext, Warning, Word};

pub struct IncorrectSectionLint {}
impl Lint for IncorrectSectionLint {
    fn name(&self) -> &'static str {
        "incorrect-section"
    }
    fn lint_token(&mut self, state: &mut ParseState, token: &Word) -> Option<Warning> {
        if let Some(current_token) = state.current_token {
            if let TokenContext::Command(Some(expected_section)) = current_token.context() {
                match state.current_section {
                    Some((section_token, ref current_section)) => {
                        if current_section != expected_section {
                            return Some(token.error(format!("Command is invalid in section {}, it can only appear in {}", current_section, expected_section))
                                        .note_at(section_token.span, "Section started here"));
                        }
                    }
                    None => {
                        return Some(token.error(format!("Command can only appear in section {}, but no section has been started.", expected_section)));
                    }
                }
            }
        }
        None
    }
}
