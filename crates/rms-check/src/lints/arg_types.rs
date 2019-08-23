use crate::{ArgType, Atom, Lint, ParseState, Suggestion, Warning, Word, TOKENS};
use strsim::levenshtein;

#[derive(Default)]
pub struct ArgTypesLint {}

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
        cmd: &Word<'_>,
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
                    cmd.value, arg.value
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
        arg_type: &ArgType,
        arg: Option<&Word<'_>>,
    ) -> Option<Warning> {
        let cmd = if let Atom::Command(cmd, _) = atom {
            cmd
        } else {
            unreachable!()
        };
        let arg = if let Some(arg) = arg {
            arg
        } else {
            return Some(atom.error(format!("Missing arguments to {}", cmd.value)));
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
            ArgType::Number => self.check_number(state, cmd, arg),
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
}

impl Lint for ArgTypesLint {
    fn name(&self) -> &'static str {
        "arg-types"
    }
    fn lint_atom(&mut self, state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Warning> {
        if let Atom::Command(cmd, args) = atom {
            let token_type = &TOKENS[&cmd.value.to_lowercase()];
            let mut warnings = vec![];
            for i in 0..token_type.arg_len() {
                if let Some(warning) = self.check_arg(
                    state,
                    atom,
                    &token_type.arg_type(i).unwrap(),
                    args.get(i as usize),
                ) {
                    warnings.push(warning);
                }
            }
            warnings
        } else {
            Default::default()
        }
    }
}

fn meant<'a>(actual: &str, possible: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let mut lowest = 10000;
    let mut result = None;

    for expected in possible {
        let lev = levenshtein(actual, expected);
        if lev < lowest {
            result = Some(expected);
            lowest = lev;
        }
    }

    result
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
    use crate::{RMSCheck, Severity};
    use codespan::{ColumnIndex, LineIndex, Location};
    use std::path::PathBuf;

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
        let result = RMSCheck::new()
            .with_lint(Box::new(ArgTypesLint::new()))
            .add_file(PathBuf::from(filename))
            .unwrap()
            .check();
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
        let first_span = first.diagnostic().primary_label.span;
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
        let second_span = second.diagnostic().primary_label.span;
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
        let third_span = third.diagnostic().primary_label.span;
        assert_eq!(
            result.resolve_position(file, third_span.start()).unwrap(),
            Location::new(LineIndex(7), ColumnIndex(18))
        );

        assert_eq!(fourth.diagnostic().severity, Severity::Error);
        assert_eq!(fourth.diagnostic().code, Some("arg-types".to_string()));
        assert_eq!(fourth.message(), "Missing arguments to create_object");
        let fourth_span = fourth.diagnostic().primary_label.span;
        assert_eq!(
            result.resolve_position(file, fourth_span.start()).unwrap(),
            Location::new(LineIndex(10), ColumnIndex(0))
        );
    }
}
