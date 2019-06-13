use super::super::{Lint, ParseState, Suggestion, Warning, Word};

pub struct IncludeLint {}

impl IncludeLint {
    pub fn new() -> Self {
        IncludeLint {}
    }
}

impl Lint for IncludeLint {
    fn name(&self) -> &'static str {
        "include"
    }
    fn lint_token(&mut self, _state: &mut ParseState, token: &Word) -> Option<Warning> {
        match token.value {
            "#include_drs" => Some(token.error("#include_drs can only be used by builtin maps")),
            "#include" => {
                let suggestion = Suggestion::from(
                    token,
                    "If you're trying to make a map pack, use a map pack generator instead.",
                );
                Some(
                    token
                        .error("#include can only be used by builtin maps")
                        .suggest(suggestion),
                )
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{RMSCheck, Severity};
    use super::IncludeLint;
    use codespan::{ColumnIndex, LineIndex};
    use std::path::PathBuf;

    #[test]
    fn include() {
        let result = RMSCheck::new()
            .with_lint(Box::new(IncludeLint::new()))
            .add_file(PathBuf::from("./tests/rms/include.rms"))
            .unwrap()
            .check();

        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        let second = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.diagnostic().severity, Severity::Error);
        assert_eq!(first.diagnostic().code, Some("include".to_string()));
        assert_eq!(
            first.message(),
            "#include_drs can only be used by builtin maps"
        );
        let first_span = first.diagnostic().labels[0].span;
        assert_eq!(
            result.resolve_position(first_span.start()).unwrap().0,
            LineIndex(0)
        );
        assert_eq!(
            result.resolve_position(first_span.start()).unwrap().1,
            ColumnIndex(0)
        );
        assert_eq!(second.diagnostic().severity, Severity::Error);
        assert_eq!(second.diagnostic().code, Some("include".to_string()));
        assert_eq!(
            second.message(),
            "#include can only be used by builtin maps"
        );
        let second_span = second.diagnostic().labels[0].span;
        assert_eq!(
            result.resolve_position(second_span.start()).unwrap().0,
            LineIndex(2)
        );
        assert_eq!(
            result.resolve_position(second_span.start()).unwrap().1,
            ColumnIndex(0)
        );
    }
}
