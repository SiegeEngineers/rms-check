use super::super::{Lint, ParseState, Suggestion, Warning, Word, TOKENS};

pub struct AttributeCaseLint {}

impl AttributeCaseLint {
    fn is_wrong_case(&self, value: &str) -> bool {
        !TOKENS.contains_key(value) && TOKENS.contains_key(&value.to_lowercase())
    }
}

impl Lint for AttributeCaseLint {
    fn name(&self) -> &'static str {
        "attribute-case"
    }
    fn lint_token(&mut self, state: &mut ParseState, token: &Word) -> Option<Warning> {
        if state.current_token.is_none() && self.is_wrong_case(token.value) {
            let suggestion = Suggestion::from(token, "Attributes must be all lowercase")
                .replace(token.value.to_lowercase());
            let message = format!("Unknown attribute `{}`", token.value);
            return Some(token.error(message).suggest(suggestion));
        }
        None
    }
}
