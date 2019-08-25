use crate::{Atom, Compatibility, Lint, ParseState, Warning};

pub struct CompatibilityLint {
    is_header_comment: bool,
    conditions: Vec<String>,
}

impl CompatibilityLint {
    pub fn new() -> Self {
        Self {
            is_header_comment: true,
            conditions: vec![],
        }
    }

    fn has_up_extension(&self, state: &ParseState<'_>) -> bool {
        if state.compatibility() >= Compatibility::UserPatch15 {
            return true;
        }
        self.conditions.iter().any(|item| item == "UP_EXTENSION")
    }
    fn has_up_available(&self, state: &ParseState<'_>) -> bool {
        if state.compatibility() >= Compatibility::UserPatch14 {
            return true;
        }
        self.conditions.iter().any(|item| item == "UP_AVAILABLE")
    }

    fn add_define_check(&mut self, name: &str) {
        self.conditions.push(name.to_string());
    }

    fn set_header(&mut self, state: &mut ParseState<'_>, name: &str, val: &str) {
        match name {
            "Compatibility" => {
                let compat = match val.to_lowercase().trim() {
                    "hd edition" | "hd" => Some(Compatibility::HDEdition),
                    "conquerors" | "aoc" => Some(Compatibility::Conquerors),
                    "userpatch 1.5" | "up 1.5" => Some(Compatibility::UserPatch15),
                    "userpatch 1.4" | "up 1.4" | "userpatch" | "up" => {
                        Some(Compatibility::UserPatch15)
                    }
                    "wololokingdoms" | "wk" => Some(Compatibility::WololoKingdoms),
                    _ => None,
                };
                if let Some(compat) = compat {
                    state.set_compatibility(compat);
                }
            }
            _ => (),
        }
    }

    fn parse_comment(&mut self, state: &mut ParseState<'_>, content: &str) {
        for mut line in content.lines() {
            line = line.trim();
            if line.starts_with("* ") {
                line = &line[2..];
            }

            let mut parts = line.splitn(2, ": ");
            if let (Some(name), Some(val)) = (parts.next(), parts.next()) {
                self.set_header(state, name, val);
            }
        }
    }
}

impl Lint for CompatibilityLint {
    fn name(&self) -> &'static str {
        "compatibility"
    }

    fn lint_atom(&mut self, state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Warning> {
        use Atom::*;

        match atom {
            Comment(_, content, _) if self.is_header_comment => {
                self.parse_comment(state, content);
            }
            _ => {
                self.is_header_comment = false;
            }
        }

        let mut warnings = vec![];

        if let Command(name, _) = atom {
            match name.value {
                "effect_amount" | "effect_percent" => {
                    if !self.has_up_extension(state) {
                        warnings.push(atom.warning("RMS Effects require UserPatch 1.5").note_at(
                            atom.file_id(),
                            atom.span(),
                            "Wrap this command in an `if UP_EXTENSION` statement or add a /* Compatibility: UserPatch 1.5 */ comment at the top of the file",
                        ))
                    }
                }
                "direct_placement" => {
                    if !self.has_up_extension(state) {
                        warnings.push(
                            atom.warning("Direct placement requires UserPatch 1.5")
                                .note_at(
                                    atom.file_id(),
                                    atom.span(),
                                    "Wrap this command in an `if UP_EXTENSION` statement or add a /* Compatibility: UserPatch 1.5 */ comment at the top of the file",
                                )
                        )
                    }
                }
                "nomad_resources" => {
                    if !self.has_up_available(state) {
                        warnings.push(
                            atom.warning("Nomad resources requires UserPatch 1.4")
                                .note_at(
                                    atom.file_id(),
                                    atom.span(),
                                    "Wrap this command in an `if UP_AVAILABLE` statement or add a /* Compatibility: UserPatch 1.4 */ comment at the top of the file",
                                )
                        )
                    }
                }
                _ => (),
            }
        };

        match atom {
            If(_, arg) => self.add_define_check(arg.value),
            ElseIf(_, arg) => {
                self.conditions.pop();
                self.add_define_check(arg.value);
            }
            Else(_) => {
                self.conditions.pop();
                // Dummy argument to make nesting work right
                self.add_define_check(" ");
            }
            EndIf(_) => {
                self.conditions.pop();
            }
            _ => (),
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::CompatibilityLint;
    use crate::{RMSCheck, Severity};
    use std::path::PathBuf;

    #[test]
    fn compatibility() {
        let result = RMSCheck::new()
            .with_lint(Box::new(CompatibilityLint::new()))
            .add_file(PathBuf::from("./tests/rms/compatibility.rms"))
            .unwrap()
            .check();

        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.diagnostic().severity, Severity::Warning);
        assert_eq!(first.diagnostic().code, Some("compatibility".to_string()));
        assert_eq!(first.message(), "RMS Effects require UserPatch 1.5");
    }
}
