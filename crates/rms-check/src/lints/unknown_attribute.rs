use crate::{Atom, AtomKind, Lint, ParseState, Suggestion, Warning, TOKENS};
use strsim::jaro_winkler;

#[allow(unused)]
pub struct UnknownAttributeLint {}
impl Lint for UnknownAttributeLint {
    fn name(&self) -> &'static str {
        "unknown-attribute"
    }
    fn lint_atom(&mut self, _state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Warning> {
        match atom.kind {
            // Treat unrecognised tokens as attributes, if they are not numbers
            AtomKind::Other { value } => {
                if !value.value.chars().all(|c| c.is_ascii_digit()) {
                    let warning = value.error(format!("Unknown attribute `{}`", value.value));
                    let warning = if let Some(similar) =
                        meant(value.value, TOKENS.keys().map(|s| s.as_ref()))
                    {
                        warning.suggest(
                            Suggestion::from(&value, format!("Did you mean `{}`?", similar))
                                .replace_unsafe(similar.to_string()),
                        )
                    } else {
                        warning
                    };
                    vec![warning]
                } else {
                    Default::default()
                }
            }
            _ => Default::default(),
        }
    }
}

fn meant<'a>(actual: &str, possible: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    possible
        .map(|expected| (expected, jaro_winkler(actual, expected)))
        .filter(|(_, similarity)| *similarity >= 0.8)
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(string, _)| string)
}
