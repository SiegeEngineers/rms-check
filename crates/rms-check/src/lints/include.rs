use crate::{Atom, Lint, ParseState, Suggestion, Warning};

#[derive(Default)]
pub struct IncludeLint {}

impl IncludeLint {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Lint for IncludeLint {
    fn name(&self) -> &'static str {
        "include"
    }
    fn lint_atom(&mut self, _state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Warning> {
        match atom {
            Atom::Command(cmd, _) if cmd.value == "#include_drs" => {
                vec![atom.error("#include_drs can only be used by builtin maps")]
            }
            Atom::Command(cmd, _) if cmd.value == "#include" => {
                let suggestion = Suggestion::new(
                    atom.file_id(),
                    atom.span(),
                    "If you're trying to make a map pack, use a map pack generator instead.",
                );
                vec![atom
                    .error("#include can only be used by builtin maps")
                    .suggest(suggestion)]
            }
            _ => Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IncludeLint;
    use crate::{RMSCheck, Severity};
    use codespan::Location;
    use std::path::PathBuf;

    #[test]
    fn include() {
        let filename = "./tests/rms/include.rms";
        let result = RMSCheck::new()
            .with_lint(Box::new(IncludeLint::new()))
            .add_file(PathBuf::from(filename))
            .unwrap()
            .check();
        let file = result.file_id(filename).unwrap();

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
        let first_span = first.diagnostic().primary_label.span;
        assert_eq!(
            result.resolve_position(file, first_span.start()).unwrap(),
            Location::new(0, 0)
        );
        assert_eq!(second.diagnostic().severity, Severity::Error);
        assert_eq!(second.diagnostic().code, Some("include".to_string()));
        assert_eq!(
            second.message(),
            "#include can only be used by builtin maps"
        );
        let second_span = second.diagnostic().primary_label.span;
        assert_eq!(
            result.resolve_position(file, second_span.start()).unwrap(),
            Location::new(2, 0)
        );
    }
}
