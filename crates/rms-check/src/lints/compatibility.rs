use super::super::{Compatibility, Lint, ParseState, TokenType, Warning, Word};

#[derive(Default)]
pub struct CompatibilityLint {
    conditions: Vec<String>,
}

impl CompatibilityLint {
    pub fn new() -> Self {
        Self::default()
    }

    fn has_up_extension(&self, state: &ParseState) -> bool {
        if state.compatibility >= Compatibility::UserPatch15 {
            return true;
        }
        self.conditions.iter().any(|item| item == "UP_EXTENSION")
    }
    fn has_up_available(&self, state: &ParseState) -> bool {
        if state.compatibility >= Compatibility::UserPatch14 {
            return true;
        }
        self.conditions.iter().any(|item| item == "UP_AVAILABLE")
    }

    fn add_define_check(&mut self, name: &str) {
        self.conditions.push(name.to_string());
    }

    fn check_token(&mut self, state: &mut ParseState, token: &Word) -> Option<Warning> {
        match token.value {
            "effect_amount" | "effect_percent" => {
                if self.has_up_extension(state) {
                    return None;
                }
                Some(token.warning("RMS Effects require UserPatch 1.5").note_at(
                    token.span,
                    "Wrap this command in an `if UP_EXTENSION` statement",
                ))
            }
            "direct_placement" => {
                if self.has_up_extension(state) {
                    return None;
                }
                Some(
                    token
                        .warning("Direct placement requires UserPatch 1.5")
                        .note_at(
                            token.span,
                            "Wrap this command in an `if UP_EXTENSION` statement",
                        ),
                )
            }
            "nomad_resources" => {
                if self.has_up_available(state) {
                    return None;
                }
                Some(
                    token
                        .warning("Nomad resources requires UserPatch 1.4")
                        .note_at(
                            token.span,
                            "Wrap this command in an `if UP_AVAILABLE` statement",
                        ),
                )
            }
            _ => None,
        }
    }
}

impl Lint for CompatibilityLint {
    fn name(&self) -> &'static str {
        "compatibility"
    }

    fn lint_token(&mut self, state: &mut ParseState, token: &Word) -> Option<Warning> {
        match state.current_token {
            Some(TokenType { name, .. }) if *name == "if" => {
                // `token` will be the arg to `if`
                self.add_define_check(token.value);
                None
            }
            _ => self.check_token(state, token),
        }
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
            .with_lint(Box::new(CompatibilityLint::default()))
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
