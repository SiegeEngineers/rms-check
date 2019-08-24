use super::super::{Lint, Nesting, ParseState, Suggestion, Warning, Word};

pub struct DeadBranchCommentLint {}
impl Lint for DeadBranchCommentLint {
    fn name(&self) -> &'static str {
        "dead-comment"
    }
    fn run_inside_comments(&self) -> bool {
        true
    }
    fn lint_token(&mut self, state: &mut ParseState<'_>, token: &Word<'_>) -> Option<Warning> {
        if token.value != "/*" {
            return None;
        }

        state.nesting.iter()
            .find_map(|n| if let Nesting::StartRandom(loc) = n {
                let suggestion = Suggestion::from(token, "Only #define constants in the `start_random` group, and then use `if` branches for the actual code.");
                Some(token.warning("Using comments inside `start_random` groups is potentially dangerous.")
                    .note_at(token.file, *loc, "`start_random` opened here")
                    .suggest(suggestion))
            } else {
                None
            })
    }
}
