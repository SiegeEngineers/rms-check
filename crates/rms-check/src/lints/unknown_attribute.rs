use super::super::{Lint, ParseState, Warning, Word, TOKENS};

#[allow(unused)]
pub struct UnknownAttributeLint {}
impl Lint for UnknownAttributeLint {
    fn name(&self) -> &'static str {
        "unknown-attribute"
    }
    fn lint_token(&mut self, state: &mut ParseState, token: &Word) -> Option<Warning> {
        if state.current_token.is_none() && !TOKENS.contains_key(&token.value.to_lowercase()) {
            return Some(token.error(format!("Unknown attribute `{}`", token.value)));
        }
        None
    }
}
