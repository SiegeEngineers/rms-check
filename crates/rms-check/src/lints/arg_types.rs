use crate::{ArgType, Atom, AtomKind, Lint, ParseState, Suggestion, Warning, Word, TOKENS};
use codespan::Span;
use cow_utils::CowUtils;
use strsim::jaro_winkler;

#[derive(Default)]
pub struct ArgTypesLint {
    actor_areas: Vec<(i32, Span)>,
}

impl ArgTypesLint {
    pub fn new() -> Self {
        Default::default()
    }

    fn check_ever_defined(&self, state: &ParseState<'_>, token: &Word<'_>) -> Option<Warning> {
        if !state.may_have_define(token.value) {
            let warn = token.warning(format!(
                "Token `{}` is never defined, this condition will always fail",
                token.value
            ));
            Some(if let Some(similar) = meant(token.value, state.defines()) {
                warn.suggest(
                    Suggestion::from(token, format!("Did you mean `{}`?", similar))
                        .replace_unsafe(similar.to_string()),
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
    ) -> Option<Warning> {
        // 1. Check if this may or may not be defined—else warn
        if !state.has_const(token.value) {
            if state.has_define(token.value) {
                // 2. Check if this has a value (is defined using #const)—else warn
                Some(token.warning(format!("Expected a valued token (defined using #const), got a valueless token `{}` (defined using #define)", token.value)))
            } else {
                let warn = token.warning(format!("Token `{}` is never defined", token.value));
                Some(if let Some(similar) = meant(token.value, state.consts()) {
                    warn.suggest(
                        Suggestion::from(token, format!("Did you mean `{}`?", similar))
                            .replace_unsafe(similar.to_string()),
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
    ) -> Option<Warning> {
        // This may be a valued (#const) constant,
        // or a number (12, -35),
        arg.value
            .parse::<i32>()
            .err()
            .map(|_| {
                let warn = arg.error(format!(
                    "Expected a number argument to {}, but got {}",
                    name.value, arg.value
                ));
                if arg.value.starts_with('(') {
                    let (_, replacement) = is_valid_rnd(&format!("rnd{}", arg.value));
                    warn.suggest(
                        Suggestion::from(arg, "Did you forget the `rnd`?")
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
    ) -> Option<Warning> {
        let name = if let AtomKind::Command { name, .. } = atom.kind {
            name
        } else {
            unreachable!("Expected AtomKind::Command, got {:?}", atom.kind)
        };
        let arg = if let Some(arg) = arg {
            arg
        } else {
            return Some(atom.error(format!("Missing arguments to {}", name.value)));
        };

        fn unexpected_number_warning(arg: &Word<'_>) -> Option<Warning> {
            arg.value.parse::<i32>().ok().map(|_| {
                arg.error(format!(
                    "Expected a const name, but got a number {}",
                    arg.value
                ))
            })
        }

        match arg_type {
            ArgType::Number => self.check_number(state, &name, arg),
            ArgType::Word => {
                unexpected_number_warning(arg)
                    .or_else(|| if arg.value.chars().any(char::is_lowercase) {
                        Some(arg.warning("Using lowercase for constant names may cause confusion with attribute or command names")
                             .suggest(Suggestion::from(arg, "Use uppercase for constants")
                                      .replace(arg.value.to_uppercase())))
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
    fn check_assign_to(&self, args: &[Word<'_>], warnings: &mut Vec<Warning>) {
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
                    warnings.push(
                        arg.warning("`assign_to` Target must be AT_COLOR, AT_PLAYER, AT_TEAM"),
                    );
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
                        warnings.push(args[1].warning(
                            "`assign_to` Number must be 1-8 when targeting AT_COLOR or AT_PLAYER",
                        ));
                    }
                }
                Some(AssignTarget::Team) => {
                    if (number < -4 || number > 4) && number != -10 {
                        warnings.push(
                            args[1]
                                .warning("`assign_to` Number must be 1-4 when targeting AT_TEAM"),
                        );
                    }
                }
                _ => (),
            }
        }

        if let Some(Ok(mode)) = args.get(2).map(|f| f.value.parse::<i32>()) {
            match target {
                Some(AssignTarget::Team) => {
                    if mode != -1 && mode != 0 {
                        warnings.push(args[2].warning("`assign_to` Mode must be 0 (random selection) or -1 (ordered selection) when targeting AT_TEAM"));
                    }
                }
                Some(_) => {
                    if mode != 0 {
                        warnings.push(args[2].warning(
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
                warnings.push(args[3].warning("`assign_to` Flags must only combine flags 1 and 2"));
            }
        }
    }
}

impl Lint for ArgTypesLint {
    fn name(&self) -> &'static str {
        "arg-types"
    }
    fn lint_atom(&mut self, state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Warning> {
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
                            warnings.push(arg.warning("Elevation value out of range (0 or 1-7)"));
                        }
                    }
                }
                "land_position" => {
                    if let Some(Ok(first)) = arguments.get(0).map(|f| f.value.parse::<i32>()) {
                        if first < 0 || first > 100 {
                            warnings
                                .push(arguments[0].warning("Land position out of range (0-100)"));
                        }
                    }
                    if let Some(Ok(second)) = arguments.get(1).map(|f| f.value.parse::<i32>()) {
                        if second < 0 || second > 99 {
                            warnings
                                .push(arguments[1].warning("Land position out of range (0-99)"));
                        }
                    }
                }
                "zone" if !arguments.is_empty() => {
                    if arguments[0].value == "99" {
                        warnings.push(arguments[0].warning("`zone 99` crashes the game"));
                    }
                }
                "assign_to" => self.check_assign_to(&arguments, &mut warnings),
                "actor_area" if !arguments.is_empty() => {
                    if let Ok(n) = arguments[0].value.parse::<i32>() {
                        self.actor_areas.push((n, arguments[0].span));
                    }
                }
                "actor_area_to_place_in" | "avoid_actor_area" if !arguments.is_empty() => {
                    if let Ok(to_place_in) = arguments[0].value.parse::<i32>() {
                        if self.actor_areas.iter().all(|(n, _)| *n != to_place_in) {
                            warnings.push(
                                arguments[0].warning(format!(
                                    "Actor area {} is never defined",
                                    to_place_in
                                )),
                            );
                        }
                    }
                }
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
    use crate::{Compatibility, RMSCheck, RMSFile, Severity};
    use codespan::{ColumnIndex, LineIndex, Location};
    use std::ops::Range;

    fn to_span(range: Range<usize>) -> Span {
        Span::new(range.start as u32, range.end as u32)
    }

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
            .check(file);
        let file = result.file_id(filename).unwrap();

        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        let second = warnings.next().unwrap();
        let third = warnings.next().unwrap();
        let fourth = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.diagnostic().severity, Severity::Error);
        assert_eq!(first.diagnostic().code, Some("arg-types".to_string()));
        assert_eq!(first.message(), "Expected a const name, but got a number 0");
        let first_span = to_span(first.diagnostic().labels[0].range.clone());
        assert_eq!(
            result.resolve_position(file, first_span.start()).unwrap(),
            Location::new(LineIndex(1), ColumnIndex(17))
        );

        assert_eq!(second.diagnostic().severity, Severity::Error);
        assert_eq!(second.diagnostic().code, Some("arg-types".to_string()));
        assert_eq!(
            second.message(),
            "Expected a const name, but got a number 10"
        );
        let second_span = to_span(second.diagnostic().labels[0].range.clone());
        assert_eq!(
            result.resolve_position(file, second_span.start()).unwrap(),
            Location::new(LineIndex(3), ColumnIndex(13))
        );

        assert_eq!(third.diagnostic().severity, Severity::Error);
        assert_eq!(third.diagnostic().code, Some("arg-types".to_string()));
        assert_eq!(
            third.message(),
            "Expected a number argument to number_of_objects, but got SOMEVAL"
        );
        let third_span = to_span(third.diagnostic().labels[0].range.clone());
        assert_eq!(
            result.resolve_position(file, third_span.start()).unwrap(),
            Location::new(LineIndex(7), ColumnIndex(18))
        );

        assert_eq!(fourth.diagnostic().severity, Severity::Error);
        assert_eq!(fourth.diagnostic().code, Some("arg-types".to_string()));
        assert_eq!(fourth.message(), "Missing arguments to create_object");
        let fourth_span = to_span(fourth.diagnostic().labels[0].range.clone());
        assert_eq!(
            result.resolve_position(file, fourth_span.start()).unwrap(),
            Location::new(LineIndex(10), ColumnIndex(0))
        );
    }

    #[test]
    fn base_elevation() {
        let filename = "base_elevation.rms";
        let file = RMSFile::from_string(filename, "create_land { base_elevation 8 }");
        let result = RMSCheck::new()
            .with_lint(Box::new(ArgTypesLint::new()))
            .check(file);
        let file = result.file_id(filename).unwrap();

        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.diagnostic().severity, Severity::Warning);
        assert_eq!(first.diagnostic().code, Some("arg-types".to_string()));
        assert_eq!(first.message(), "Elevation value out of range (0 or 1-7)");
        let first_span = to_span(first.diagnostic().labels[0].range.clone());
        assert_eq!(
            result.resolve_position(file, first_span.start()).unwrap(),
            Location::new(LineIndex(0), ColumnIndex(29))
        );
    }

    #[test]
    fn assign_to() {
        let filename = "assign_to.rms";
        let file = RMSFile::from_string(filename, "create_land { assign_to X 0 0 0 }");
        let result = RMSCheck::new()
            .with_lint(Box::new(ArgTypesLint::new()))
            .check(file);
        let file = result.file_id(filename).unwrap();
        let mut warnings = result.iter();
        assert_eq!(
            warnings.next().unwrap().message(),
            "Token `X` is never defined"
        );
        let first = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.diagnostic().severity, Severity::Warning);
        assert_eq!(first.diagnostic().code, Some("arg-types".to_string()));
        assert_eq!(
            first.message(),
            "`assign_to` Target must be AT_COLOR, AT_PLAYER, AT_TEAM"
        );
        let first_span = to_span(first.diagnostic().labels[0].range.clone());
        assert_eq!(
            result.resolve_position(file, first_span.start()).unwrap(),
            Location::new(LineIndex(0), ColumnIndex(24))
        );

        let file = RMSFile::from_string(filename, "create_land { assign_to AT_TEAM 0 0 0 }");
        let result = RMSCheck::new()
            .compatibility(Compatibility::WololoKingdoms)
            .with_lint(Box::new(ArgTypesLint::new()))
            .check(file);
        assert_eq!(result.iter().count(), 0);

        let file = RMSFile::from_string(filename, "create_land { assign_to AT_TEAM 7 -2 4 }");
        let result = RMSCheck::new()
            .compatibility(Compatibility::WololoKingdoms)
            .with_lint(Box::new(ArgTypesLint::new()))
            .check(file);
        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        assert_eq!(first.diagnostic().severity, Severity::Warning);
        assert_eq!(first.diagnostic().code, Some("arg-types".to_string()));
        assert_eq!(
            first.message(),
            "`assign_to` Number must be 1-4 when targeting AT_TEAM"
        );
        let second = warnings.next().unwrap();
        assert_eq!(second.diagnostic().severity, Severity::Warning);
        assert_eq!(second.diagnostic().code, Some("arg-types".to_string()));
        assert_eq!(second.message(), "`assign_to` Mode must be 0 (random selection) or -1 (ordered selection) when targeting AT_TEAM");
        let third = warnings.next().unwrap();
        assert_eq!(third.diagnostic().severity, Severity::Warning);
        assert_eq!(third.diagnostic().code, Some("arg-types".to_string()));
        assert_eq!(
            third.message(),
            "`assign_to` Flags must only combine flags 1 and 2"
        );
        assert!(warnings.next().is_none());
    }

    #[test]
    fn actor_area() {
        let filename = "actor_area.rms";
        let file = RMSFile::from_string(
            filename,
            "
            create_object VILLAGER {
                actor_area 1
            }
            create_object VILLAGER {
                actor_area_to_place_in 1
            }
            create_object VILLAGER {
                avoid_actor_area 1
            }
            create_object VILLAGER {
                actor_area_to_place_in 17
            }
            create_object VILLAGER {
                avoid_actor_area 18
            }
        ",
        );
        let result = RMSCheck::new()
            .with_lint(Box::new(ArgTypesLint::new()))
            .check(file);
        let mut warnings = result.iter();

        let first = warnings.next().unwrap();
        assert_eq!(first.diagnostic().severity, Severity::Warning);
        assert_eq!(first.diagnostic().code, Some("arg-types".to_string()));
        assert_eq!(first.message(), "Actor area 17 is never defined");
        let second = warnings.next().unwrap();
        assert_eq!(second.diagnostic().severity, Severity::Warning);
        assert_eq!(second.diagnostic().code, Some("arg-types".to_string()));
        assert_eq!(second.message(), "Actor area 18 is never defined");
        assert!(warnings.next().is_none());
    }
}
