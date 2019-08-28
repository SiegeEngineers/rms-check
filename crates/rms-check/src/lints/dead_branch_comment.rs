use super::super::{Lint, Nesting, ParseState, Suggestion, Warning, Atom};

pub struct DeadBranchCommentLint {}
impl Lint for DeadBranchCommentLint {
    fn name(&self) -> &'static str {
        "dead-comment"
    }

    fn lint_atom(&mut self, state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Warning> {
        let mut warnings = vec![];

        if let Atom::Comment(_, _, _) = atom {
            for nest in &state.nesting {
                if let Nesting::StartRandom(start) = nest {
                    let suggestion = Suggestion::new(atom.file_id(), atom.span(), "Only #define constants in the `start_random` group, and then use `if` branches for the actual code.");
                    warnings.push(atom.warning("Using comments inside `start_random` groups is potentially dangerous.")
                        .note_at(start.file_id(), start.span(), "`start_random` opened here")
                        .suggest(suggestion));
                }
            }
        }

        warnings
    }
}
