use crate::diagnostic::{Diagnostic, Fix};
use crate::{Atom, AtomKind, Lint, ParseState};

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
    fn lint_atom(&mut self, _state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Diagnostic> {
        match atom.kind {
            AtomKind::Command { name, .. } if name.value == "#include_drs" => {
                vec![Diagnostic::error(
                    atom.location,
                    "#include_drs can only be used by builtin maps",
                )]
            }
            AtomKind::Command { name, .. } if name.value == "#include" => {
                vec![
                    Diagnostic::error(atom.location, "#include can only be used by builtin maps")
                        .suggest(Fix::new(
                        atom.location,
                        "If you're trying to make a map pack, use a map pack generator instead.",
                    )),
                ]
            }
            _ => Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IncludeLint;
    use crate::diagnostic::{ByteIndex, SourceLocation};
    use crate::{RMSCheck, RMSFile, Severity};

    #[test]
    fn include() {
        let filename = "./tests/rms/include.rms";
        let file = RMSFile::from_path(filename).unwrap();
        let result = RMSCheck::new()
            .with_lint(Box::new(IncludeLint::new()))
            .check(&file);
        let file = file.file_id();

        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        let second = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.severity(), Severity::Error);
        assert_eq!(first.code(), Some("include"));
        assert_eq!(
            first.message(),
            "#include_drs can only be used by builtin maps"
        );
        assert_eq!(
            first.location(),
            SourceLocation::new(file, ByteIndex::from(0)..ByteIndex::from(33))
        );
        assert_eq!(second.severity(), Severity::Error);
        assert_eq!(second.code(), Some("include"));
        assert_eq!(
            second.message(),
            "#include can only be used by builtin maps"
        );
        assert_eq!(
            second.location(),
            SourceLocation::new(file, ByteIndex::from(35)..ByteIndex::from(51))
        );
    }
}
