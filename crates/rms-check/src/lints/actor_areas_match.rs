use crate::diagnostic::{Diagnostic, SourceLocation};
use crate::{Atom, AtomKind, Lint, ParseState};

#[derive(Default)]
pub struct ActorAreasMatchLint {
    actor_areas: Vec<(i32, SourceLocation)>,
}

impl ActorAreasMatchLint {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Lint for ActorAreasMatchLint {
    fn name(&self) -> &'static str {
        "actor-areas-match"
    }
    fn lint_atom(&mut self, _state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Diagnostic> {
        if let AtomKind::Command { name, arguments } = &atom.kind {
            let mut warnings = vec![];

            match name.value {
                "actor_area" if !arguments.is_empty() => {
                    if let Ok(n) = arguments[0].value.parse::<i32>() {
                        self.actor_areas.push((n, arguments[0].location));
                    }
                }
                "actor_area_to_place_in" | "avoid_actor_area" if !arguments.is_empty() => {
                    if let Ok(to_place_in) = arguments[0].value.parse::<i32>() {
                        if self.actor_areas.iter().all(|(n, _)| *n != to_place_in) {
                            warnings.push(Diagnostic::warning(
                                arguments[0].location,
                                format_args!("Actor area {} is never defined", to_place_in),
                            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RMSCheck, RMSFile, Severity};

    #[test]
    fn actor_areas_match() {
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
            .with_lint(Box::new(ActorAreasMatchLint::new()))
            .check(&file);
        let mut warnings = result.iter();

        let first = warnings.next().unwrap();
        assert_eq!(first.severity(), Severity::Warning);
        assert_eq!(first.code(), Some("actor-areas-match"));
        assert_eq!(first.message(), "Actor area 17 is never defined");
        let second = warnings.next().unwrap();
        assert_eq!(second.severity(), Severity::Warning);
        assert_eq!(second.code(), Some("actor-areas-match"));
        assert_eq!(second.message(), "Actor area 18 is never defined");
        assert!(warnings.next().is_none());
    }
}
