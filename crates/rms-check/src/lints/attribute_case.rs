use crate::{Atom, AtomKind, Lint, ParseState, Suggestion, Warning, TOKENS};

pub struct AttributeCaseLint {}

impl AttributeCaseLint {
    fn is_wrong_case(&self, value: &str) -> bool {
        !TOKENS.contains_key(value) && TOKENS.contains_key(&value.to_ascii_lowercase())
    }
}

impl Lint for AttributeCaseLint {
    fn name(&self) -> &'static str {
        "attribute-case"
    }
    fn lint_atom(&mut self, _state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Warning> {
        match atom.kind {
            AtomKind::Command { name, .. } if self.is_wrong_case(name.value) => {
                let suggestion = Suggestion::from(&name, "Convert to lowercase")
                    .replace(name.value.to_ascii_lowercase());
                let message = format!("Unknown attribute `{}`", name.value);
                vec![atom.error(message).suggest(suggestion)]
            }
            _ => Default::default(),
        }
    }
}
