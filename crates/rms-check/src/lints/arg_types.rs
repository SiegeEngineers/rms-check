use crate::diagnostic::{Diagnostic, Fix};
use crate::{ArgType, Atom, AtomKind, Lint, ParseState, Word, TOKENS};
use cow_utils::CowUtils;
use strsim::jaro_winkler;

#[derive(Default)]
pub struct ArgTypesLint {}

impl ArgTypesLint {
    pub fn new() -> Self {
        Default::default()
    }

    fn check_ever_defined(&self, state: &ParseState<'_>, token: &Word<'_>) -> Option<Diagnostic> {
        if !state.may_have_define(token.value) {
            let warn = Diagnostic::warning(
                token.location,
                format_args!(
                    "Token `{}` is never defined, this condition will always fail",
                    token.value
                ),
            );
            Some(if let Some(similar) = meant(token.value, state.defines()) {
                warn.suggest(
                    Fix::new(token.location, format_args!("Did you mean `{}`?", similar))
                        .replace(similar),
                )
            } else {
                warn
            })
        } else {
            None
        }
    }

    /// Check if a constant was ever defined with a value (using #const)
    fn check_defined_with_value(
        &self,
        state: &ParseState<'_>,
        token: &Word<'_>,
    ) -> Option<Diagnostic> {
        // 1. Check if this may or may not be defined—else warn
        if !state.has_const(token.value) {
            if state.has_define(token.value) {
                // 2. Check if this has a value (is defined using #const)—else warn
                Some(Diagnostic::warning(token.location, format_args!("Expected a valued token (defined using #const), got a valueless token `{}` (defined using #define)", token.value)))
            } else {
                let warn = Diagnostic::warning(
                    token.location,
                    format_args!("Token `{}` is never defined", token.value),
                );
                Some(if let Some(similar) = meant(token.value, state.consts()) {
                    warn.suggest(
                        Fix::new(token.location, format_args!("Did you mean `{}`?", similar))
                            .replace(similar),
                    )
                } else {
                    warn
                })
            }
        } else {
            None
        }
    }

    fn check_number(
        &self,
        _state: &ParseState<'_>,
        name: &Word<'_>,
        arg: &Word<'_>,
    ) -> Option<Diagnostic> {
        // This may be a valued (#const) constant,
        // or a number (12, -35),
        arg.value
            .parse::<i32>()
            .err()
            .map(|_| {
                let warn = Diagnostic::error(
                    arg.location,
                    format_args!(
                        "Expected a number argument to {}, but got {}",
                        name.value, arg.value
                    ),
                );
                if arg.value.starts_with('(') {
                    let (_, replacement) = is_valid_rnd(&format!("rnd{}", arg.value));
                    warn.suggest(
                        Fix::new(arg.location, "Did you forget the `rnd`?")
                            .replace(replacement.unwrap_or_else(|| format!("rnd{}", arg.value))),
                    )
                } else {
                    warn
                }
            })
            .and_then(|warn| {
                // or rnd(\d+,\d+)
                if let (true, _) = is_valid_rnd(arg.value) {
                    None
                } else {
                    Some(warn)
                }
            })
    }

    fn check_arg(
        &self,
        state: &ParseState<'_>,
        atom: &Atom<'_>,
        arg_type: ArgType,
        arg: Option<&Word<'_>>,
    ) -> Option<Diagnostic> {
        let name = if let AtomKind::Command { name, .. } = atom.kind {
            name
        } else {
            unreachable!("Expected AtomKind::Command, got {:?}", atom.kind)
        };
        let arg = if let Some(arg) = arg {
            arg
        } else {
            return Some(Diagnostic::error(
                atom.location,
                format_args!("Missing arguments to {}", name.value),
            ));
        };

        fn unexpected_number_warning(arg: &Word<'_>) -> Option<Diagnostic> {
            arg.value.parse::<i32>().ok().map(|_| {
                Diagnostic::error(
                    arg.location,
                    format_args!("Expected a const name, but got a number {}", arg.value),
                )
            })
        }

        match arg_type {
            ArgType::Number => self.check_number(state, &name, arg),
            ArgType::Word => {
                unexpected_number_warning(arg)
                    .or_else(|| if arg.value.chars().any(char::is_lowercase) {
                        Some(Diagnostic::warning(arg.location, "Using lowercase for constant names may cause confusion with attribute or command names")
                             .suggest(Fix::new(arg.location, "Use uppercase for constants")
                                      .replace(arg.value.cow_to_uppercase())))
                    } else {
                        None
                    })
            },
            ArgType::OptionalToken => {
                unexpected_number_warning(arg).or_else(|| self.check_ever_defined(state, arg))
            }
            ArgType::Token => {
                unexpected_number_warning(arg).or_else(|| self.check_defined_with_value(state, arg))
            }
            _ => None,
        }
    }

    /// Check the arguments to an `assign_to` attribute.
    fn check_assign_to(&self, args: &[Word<'_>], warnings: &mut Vec<Diagnostic>) {
        enum AssignTarget {
            Color,
            Player,
            Team,
        }
        let target = if let Some(arg) = args.get(0) {
            match arg.value {
                "AT_COLOR" => Some(AssignTarget::Color),
                "AT_PLAYER" => Some(AssignTarget::Player),
                "AT_TEAM" => Some(AssignTarget::Team),
                _ => {
                    warnings.push(Diagnostic::warning(
                        arg.location,
                        "`assign_to` Target must be AT_COLOR, AT_PLAYER, AT_TEAM",
                    ));
                    None
                }
            }
        } else {
            None
        };

        if let Some(Ok(number)) = args.get(1).map(|f| f.value.parse::<i32>()) {
            match target {
                Some(AssignTarget::Color) | Some(AssignTarget::Player) => {
                    if number < 0 || number > 8 {
                        warnings.push(Diagnostic::warning(
                            args[1].location,
                            "`assign_to` Number must be 1-8 when targeting AT_COLOR or AT_PLAYER",
                        ));
                    }
                }
                Some(AssignTarget::Team) => {
                    if (number < -4 || number > 4) && number != -10 {
                        warnings.push(Diagnostic::warning(
                            args[1].location,
                            "`assign_to` Number must be 1-4 when targeting AT_TEAM",
                        ));
                    }
                }
                _ => (),
            }
        }

        if let Some(Ok(mode)) = args.get(2).map(|f| f.value.parse::<i32>()) {
            match target {
                Some(AssignTarget::Team) => {
                    if mode != -1 && mode != 0 {
                        warnings.push(Diagnostic::warning(args[2].location,"`assign_to` Mode must be 0 (random selection) or -1 (ordered selection) when targeting AT_TEAM"));
                    }
                }
                Some(_) => {
                    if mode != 0 {
                        warnings.push(Diagnostic::warning(
                            args[2].location,
                            "`assign_to` Mode should be 0 when targeting AT_COLOR or AT_PLAYER",
                        ));
                    }
                }
                _ => (),
            }
        }

        if let Some(Ok(flags)) = args.get(3).map(|f| f.value.parse::<i32>()) {
            let mask = 1 | 2;
            if (flags & mask) != flags {
                warnings.push(Diagnostic::warning(
                    args[3].location,
                    "`assign_to` Flags must only combine flags 1 and 2",
                ));
            }
        }
    }
}

impl Lint for ArgTypesLint {
    fn name(&self) -> &'static str {
        "arg-types"
    }
    fn lint_atom(&mut self, state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Diagnostic> {
        if let AtomKind::Command { name, arguments } = &atom.kind {
            let token_type = &TOKENS[name.value.cow_to_ascii_lowercase().as_ref()];
            let mut warnings = vec![];
            for i in 0..token_type.arg_len() {
                if let Some(warning) = self.check_arg(
                    state,
                    atom,
                    token_type.arg_type(i).unwrap(),
                    arguments.get(i as usize),
                ) {
                    warnings.push(warning);
                }
            }

            match name.value {
                "base_elevation" if !arguments.is_empty() => {
                    let arg = arguments[0];
                    if let Ok(n) = arg.value.parse::<i32>() {
                        if n < 0 || n > 7 {
                            warnings.push(Diagnostic::warning(
                                arg.location,
                                "Elevation value out of range (0 or 1-7)",
                            ));
                        }
                    }
                }
                "land_position" => {
                    if let Some(Ok(first)) = arguments.get(0).map(|f| f.value.parse::<i32>()) {
                        if first < 0 || first > 100 {
                            warnings.push(Diagnostic::warning(
                                arguments[0].location,
                                "Land position out of range (0-100)",
                            ));
                        }
                    }
                    if let Some(Ok(second)) = arguments.get(1).map(|f| f.value.parse::<i32>()) {
                        if second < 0 || second > 99 {
                            warnings.push(Diagnostic::warning(
                                arguments[1].location,
                                "Land position out of range (0-99)",
                            ));
                        }
                    }
                }
                "zone" if !arguments.is_empty() => {
                    if arguments[0].value == "99" {
                        warnings.push(Diagnostic::warning(
                            arguments[0].location,
                            "`zone 99` crashes the game",
                        ));
                    }
                }
                "assign_to" => self.check_assign_to(&arguments, &mut warnings),
                _ => (),
            }

            warnings
        } else {
            Default::default()
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

/// Check if a string is numeric.
fn is_numeric(s: &str) -> bool {
    s.parse::<i32>().is_ok()
}

/// Check if a string contains a valid rnd(1,10) call.
///
/// Returns a tuple with values:
///
///   0. whether the string was valid
///   1. an optional valid replacement value
fn is_valid_rnd(s: &str) -> (bool, Option<String>) {
    if s.starts_with("rnd(") && s.ends_with(')') && s[4..s.len() - 1].split(',').all(is_numeric) {
        return (true, None);
    } else if s.chars().any(char::is_whitespace) {
        let no_ws = s
            .chars()
            .filter(|c| !char::is_whitespace(*c))
            .collect::<String>();
        if let (true, _) = is_valid_rnd(&no_ws) {
            return (false, Some(no_ws));
        }
    }
    (false, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{ByteIndex, SourceLocation};
    use crate::{Compatibility, RMSCheck, RMSFile, Severity};

    #[test]
    fn is_numeric_test() {
        assert!(is_numeric("10"));
        assert!(is_numeric("432543"));
        assert!(!is_numeric("rnd(1,3)"));
        assert!(!is_numeric("SOMEVAL"));
    }

    #[test]
    fn is_valid_rnd_test() {
        assert_eq!(is_valid_rnd("rnd(1,2)"), (true, None));
        assert_eq!(is_valid_rnd("rnd(4,2)"), (true, None)); // TODO this should probably not be true?
        assert_eq!(
            is_valid_rnd("rnd(4, 2)"),
            (false, Some("rnd(4,2)".to_string()))
        );
        assert_eq!(is_valid_rnd("SOMEVAL"), (false, None));
        assert_eq!(is_valid_rnd("42"), (false, None));
    }

    #[test]
    fn arg_types() {
        let filename = "./tests/rms/arg-types.rms";
        let file = RMSFile::from_path(filename).unwrap();
        let result = RMSCheck::new()
            .with_lint(Box::new(ArgTypesLint::new()))
            .check(&file);
        let file = file.file_id();

        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        let second = warnings.next().unwrap();
        let third = warnings.next().unwrap();
        let fourth = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.severity(), Severity::Error);
        assert_eq!(first.code(), Some("arg-types"));
        assert_eq!(first.message(), "Expected a const name, but got a number 0");
        assert_eq!(
            first.location(),
            SourceLocation::new(file, ByteIndex::from(64)..ByteIndex::from(65))
        );

        assert_eq!(second.severity(), Severity::Error);
        assert_eq!(second.code(), Some("arg-types"));
        assert_eq!(
            second.message(),
            "Expected a const name, but got a number 10"
        );
        assert_eq!(
            second.location(),
            SourceLocation::new(file, ByteIndex::from(109)..ByteIndex::from(111))
        );

        assert_eq!(third.severity(), Severity::Error);
        assert_eq!(third.code(), Some("arg-types"));
        assert_eq!(
            third.message(),
            "Expected a number argument to number_of_objects, but got SOMEVAL"
        );
        assert_eq!(
            third.location(),
            SourceLocation::new(file, ByteIndex::from(176)..ByteIndex::from(183))
        );

        assert_eq!(fourth.severity(), Severity::Error);
        assert_eq!(fourth.code(), Some("arg-types"));
        assert_eq!(fourth.message(), "Missing arguments to create_object");
        assert_eq!(
            fourth.location(),
            SourceLocation::new(file, ByteIndex::from(215)..ByteIndex::from(228))
        );
    }

    #[test]
    fn base_elevation() {
        let filename = "base_elevation.rms";
        let file = RMSFile::from_string(filename, "create_land { base_elevation 8 }");
        let result = RMSCheck::new()
            .with_lint(Box::new(ArgTypesLint::new()))
            .check(&file);
        let file = file.file_id();

        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.severity(), Severity::Warning);
        assert_eq!(first.code(), Some("arg-types"));
        assert_eq!(first.message(), "Elevation value out of range (0 or 1-7)");
        assert_eq!(
            first.location(),
            SourceLocation::new(file, ByteIndex::from(29)..ByteIndex::from(30))
        );
    }

    #[test]
    fn assign_to() {
        let filename = "assign_to.rms";
        let file = RMSFile::from_string(filename, "create_land { assign_to X 0 0 0 }");
        let result = RMSCheck::new()
            .with_lint(Box::new(ArgTypesLint::new()))
            .check(&file);
        let file = file.file_id();
        let mut warnings = result.iter();
        assert_eq!(
            warnings.next().unwrap().message(),
            "Token `X` is never defined"
        );
        let first = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.severity(), Severity::Warning);
        assert_eq!(first.code(), Some("arg-types"));
        assert_eq!(
            first.message(),
            "`assign_to` Target must be AT_COLOR, AT_PLAYER, AT_TEAM"
        );
        assert_eq!(
            first.location(),
            SourceLocation::new(file, ByteIndex::from(24)..ByteIndex::from(25))
        );

        let file = RMSFile::from_string(filename, "create_land { assign_to AT_TEAM 0 0 0 }");
        let result = RMSCheck::new()
            .compatibility(Compatibility::WololoKingdoms)
            .with_lint(Box::new(ArgTypesLint::new()))
            .check(&file);
        assert_eq!(result.iter().count(), 0);

        let file = RMSFile::from_string(filename, "create_land { assign_to AT_TEAM 7 -2 4 }");
        let result = RMSCheck::new()
            .compatibility(Compatibility::WololoKingdoms)
            .with_lint(Box::new(ArgTypesLint::new()))
            .check(&file);
        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        assert_eq!(first.severity(), Severity::Warning);
        assert_eq!(first.code(), Some("arg-types"));
        assert_eq!(
            first.message(),
            "`assign_to` Number must be 1-4 when targeting AT_TEAM"
        );
        let second = warnings.next().unwrap();
        assert_eq!(second.severity(), Severity::Warning);
        assert_eq!(second.code(), Some("arg-types"));
        assert_eq!(second.message(), "`assign_to` Mode must be 0 (random selection) or -1 (ordered selection) when targeting AT_TEAM");
        let third = warnings.next().unwrap();
        assert_eq!(third.severity(), Severity::Warning);
        assert_eq!(third.code(), Some("arg-types"));
        assert_eq!(
            third.message(),
            "`assign_to` Flags must only combine flags 1 and 2"
        );
        assert!(warnings.next().is_none());
    }
}
