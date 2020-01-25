use crate::{Atom, AtomKind, Compatibility, Lint, ParseState, Warning};

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
                    "definitive edition" | "de" => Some(Compatibility::DefinitiveEdition),
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
        match &atom.kind {
            AtomKind::Comment { content, .. } if self.is_header_comment => {
                self.parse_comment(state, content);
            }
            _ => {
                self.is_header_comment = false;
            }
        }

        let mut warnings = vec![];

        if let AtomKind::Command { name, .. } = &atom.kind {
            match name.value {
                "effect_amount" | "effect_percent" => {
                    if !self.has_up_extension(state) {
                        warnings.push(atom.warning("RMS Effects require UserPatch 1.5").note_at(
                            atom.file,
                            atom.span,
                            "Wrap this command in an `if UP_EXTENSION` statement or add a /* Compatibility: UserPatch 1.5 */ comment at the top of the file",
                        ))
                    }
                }
                "direct_placement" => {
                    if !self.has_up_extension(state) {
                        warnings.push(
                            atom.warning("Direct placement requires UserPatch 1.5")
                                .note_at(
                                    atom.file,
                                    atom.span,
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
                                    atom.file,
                                    atom.span,
                                    "Wrap this command in an `if UP_AVAILABLE` statement or add a /* Compatibility: UserPatch 1.4 */ comment at the top of the file",
                                )
                        )
                    }
                }
                "actor_area"
                | "actor_area_to_place_in"
                | "avoid_actor_area"
                | "actor_area_radius" => {
                    if state.compatibility() != Compatibility::DefinitiveEdition {
                        warnings.push(
                            atom.warning("Actor areas are only supported in the Definitive Edition")
                                .note_at(
                                    atom.file,
                                    atom.span,
                                    "Add a /* Compatibility: Definitive Edition */ comment at the top of the file",
                                )
                        )
                    }
                }
                _ => (),
            }
        };

        match atom.kind {
            AtomKind::If { condition, .. } => self.add_define_check(condition.value),
            AtomKind::ElseIf { condition, .. } => {
                self.conditions.pop();
                self.add_define_check(condition.value);
            }
            AtomKind::Else { .. } => {
                self.conditions.pop();
                // Dummy argument to make nesting work right
                self.add_define_check(" ");
            }
            AtomKind::EndIf { .. } => {
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
    use crate::{RMSCheck, RMSFile, Severity};

    #[test]
    fn compatibility() {
        let file = RMSFile::from_path("./tests/rms/compatibility.rms").unwrap();
        let result = RMSCheck::new()
            .with_lint(Box::new(CompatibilityLint::new()))
            .check(file);

        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.diagnostic().severity, Severity::Warning);
        assert_eq!(first.diagnostic().code, Some("compatibility".to_string()));
        assert_eq!(first.message(), "RMS Effects require UserPatch 1.5");
    }
}
