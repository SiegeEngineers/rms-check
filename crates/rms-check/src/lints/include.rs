use crate::{Atom, AtomKind, Lint, ParseState, Suggestion, Warning};

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
        match atom.kind {
            AtomKind::Command { name, .. } if name.value == "#include_drs" => {
                vec![atom.error("#include_drs can only be used by builtin maps")]
            }
            AtomKind::Command { name, .. } if name.value == "#include" => {
                let suggestion = Suggestion::new(
                    atom.file,
                    atom.span,
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
    use crate::{RMSCheck, RMSFile, Severity};
    use codespan::{Location, Span};
    use std::ops::Range;

    fn to_span(range: Range<usize>) -> Span {
        Span::new(range.start as u32, range.end as u32)
    }

    #[test]
    fn include() {
        let filename = "./tests/rms/include.rms";
        let file = RMSFile::from_path(filename).unwrap();
        let result = RMSCheck::new()
            .with_lint(Box::new(IncludeLint::new()))
            .check(file);
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
        let first_span = to_span(first.diagnostic().labels[0].range.clone());
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
        let second_span = to_span(second.diagnostic().labels[0].range.clone());
        assert_eq!(
            result.resolve_position(file, second_span.start()).unwrap(),
            Location::new(2, 0)
        );
    }
}
