use crate::diagnostic::{Diagnostic, Fix};
use crate::{Atom, AtomKind, Compatibility, Lint, ParseState};

#[derive(Default)]
pub struct CompatibilityLint {
    conditions: Vec<String>,
}

impl CompatibilityLint {
    pub fn new() -> Self {
        Self::default()
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
}

impl Lint for CompatibilityLint {
    fn name(&self) -> &'static str {
        "compatibility"
    }

    fn lint_atom(&mut self, state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Diagnostic> {
        let mut warnings = vec![];

        if let AtomKind::Command { name, .. } = &atom.kind {
            match name.value {
                "effect_amount" | "effect_percent" => {
                    if !self.has_up_extension(state) {
                        warnings.push(Diagnostic::warning(atom.location, "RMS Effects require UserPatch 1.5")
                                      .suggest(Fix::new(atom.location, "Wrap this command in an `if UP_EXTENSION` statement or add a /* Compatibility: UserPatch 1.5 */ comment at the top of the file")));
                    }
                }
                "direct_placement" => {
                    if !self.has_up_extension(state) {
                        warnings.push(Diagnostic::warning(atom.location, "Direct placement requires UserPatch 1.5 or Definitive Edition")
                                .suggest(Fix::new(atom.location,
                                    "Wrap this command in an `if UP_EXTENSION` statement or add a /* Compatibility: UserPatch 1.5 */ comment at the top of the file",
                                ))
                        )
                    }
                }
                "nomad_resources" => {
                    if !self.has_up_available(state) && state.compatibility() != Compatibility::HDEdition {
                        warnings.push(
                            Diagnostic::warning(atom.location, "Nomad resources requires UserPatch 1.4 or HD Edition")
                                .suggest(Fix::new(atom.location,
                                    "Wrap this command in an `if UP_AVAILABLE` statement or add a /* Compatibility: UserPatch 1.4 */ comment at the top of the file",
                                ))
                        )
                    }
                }
                "actor_area"
                | "actor_area_to_place_in"
                | "avoid_actor_area"
                | "avoid_all_actor_areas"
                | "actor_area_radius" if state.compatibility() != Compatibility::DefinitiveEdition => {
                    warnings.push(
                        Diagnostic::warning(atom.location, "Actor areas are only supported in the Definitive Edition")
                        .suggest(Fix::new(atom.location,
                                          "Add a /* Compatibility: Definitive Edition */ comment at the top of the file",)
                                )
                        )
                }
                "avoid_forest_zone"
                | "place_on_forest_zone"
                | "avoid_cliff_zone" if state.compatibility() != Compatibility::DefinitiveEdition => {
                    warnings.push(
                        Diagnostic::warning(atom.location, "Forest and cliff zones are only supported in the Definitive Edition")
                            .suggest(Fix::new(atom.location,
                                "Add a /* Compatibility: Definitive Edition */ comment at the top of the file",)
                            )
                    )
                }
                "second_object" if state.compatibility() != Compatibility::DefinitiveEdition => {
                    warnings.push(
                        Diagnostic::warning(atom.location, "second_object is only supported in the Definitive Edition")
                            .suggest(Fix::new(atom.location,
                                "Add a /* Compatibility: Definitive Edition */ comment at the top of the file",)
                            )
                    )
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
    use crate::{Compatibility, RMSCheck, RMSFile, Severity};

    #[test]
    fn compatibility() -> anyhow::Result<()> {
        let file = RMSFile::from_path("./tests/rms/compatibility.rms")?;
        let result = RMSCheck::new()
            .with_lint(Box::new(CompatibilityLint::new()))
            .check(&file);

        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.severity(), Severity::Warning);
        assert_eq!(first.code(), Some("compatibility"));
        assert_eq!(first.message(), "RMS Effects require UserPatch 1.5");
        Ok(())
    }

    #[test]
    fn nomad_resources() -> anyhow::Result<()> {
        let file = RMSFile::from_string("nomad.rms", "<PLAYER_SETUP> nomad_resources");

        let checks = [
            (Compatibility::All, false),
            (Compatibility::HDEdition, true),
            (Compatibility::UserPatch14, true),
            (Compatibility::DefinitiveEdition, true),
        ];

        for (compatibility, supports_nomad_resources) in &checks {
            let result = RMSCheck::new()
                .compatibility(*compatibility)
                .with_lint(Box::new(CompatibilityLint::new()))
                .check(&file);

            let mut warnings = result.iter();
            if *supports_nomad_resources {
                assert!(warnings.next().is_none());
            } else {
                // No compatibility setting: nomad_resources may be unsupported
                let first = warnings.next().unwrap();
                assert!(warnings.next().is_none());
                assert_eq!(first.severity(), Severity::Warning);
                assert_eq!(first.code(), Some("compatibility"));
            }
        }

        Ok(())
    }
}
