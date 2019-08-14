use super::super::{Lint, ParseState, Warning, Atom};

#[allow(unused)]
pub struct UnknownAttributeLint {}
impl Lint for UnknownAttributeLint {
    fn name(&self) -> &'static str {
        "unknown-attribute"
    }
    fn lint_atom(&mut self, _state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Warning> {
        match atom {
            // Treat unrecognised tokens as attributes, if they are not numbers
            Atom::Other(word) => if !word.value.chars().all(|c| c.is_ascii_digit()) {
                vec![word.error(format!("Unknown attribute `{}`", word.value))]
            } else {
                Default::default()
            },
            _ => Default::default()
        }
    }
}
