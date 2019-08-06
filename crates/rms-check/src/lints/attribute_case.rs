use super::super::{Lint, ParseState, Suggestion, Warning, Atom, TOKENS};

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
    fn lint_atom(&mut self, state: &mut ParseState, atom: &Atom) -> Vec<Warning> {
        match atom {
            Atom::Command(cmd, _) if self.is_wrong_case(cmd.value) => {
                let suggestion = Suggestion::from(cmd, "Attributes must be all lowercase")
                    .replace(cmd.value.to_lowercase());
                let message = format!("Unknown attribute `{}`", cmd.value);
                vec![atom.error(message).suggest(suggestion)]
            },
            _ => Default::default(),
        }
    }
}
