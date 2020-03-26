use crate::{Atom, AtomKind, Lint, ParseState, Suggestion, Warning, TOKENS};
use cow_utils::CowUtils;
use std::borrow::Cow;

pub struct AttributeCaseLint {}

impl AttributeCaseLint {
    fn fix_case(&self, value: &str) -> Option<String> {
        match value.cow_to_ascii_lowercase() {
            // If the value was already lowercase, nothing can change, and we don't need to check.
            Cow::Borrowed(_) => None,
            Cow::Owned(lower_value)
                if !TOKENS.contains_key(value) && TOKENS.contains_key(lower_value.as_str()) =>
            {
                Some(lower_value)
            }
            // If the value wasn't lowercase, but the lowercase value _also_ isn't an attribute, we
            // can't fix it.
            Cow::Owned(_) => None,
        }
    }
}

impl Lint for AttributeCaseLint {
    fn name(&self) -> &'static str {
        "attribute-case"
    }
    fn lint_atom(&mut self, _state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Warning> {
        match atom.kind {
            AtomKind::Command { name, .. } => {
                if let Some(fixed_case) = self.fix_case(name.value) {
                    let suggestion =
                        Suggestion::from(&name, "Convert to lowercase").replace(fixed_case);
                    let message = format!("Unknown attribute `{}`", name.value);
                    vec![atom.error(message).suggest(suggestion)]
                } else {
                    vec![]
                }
            }
            _ => Default::default(),
        }
    }
}
