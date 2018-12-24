use super::super::{
    Lint,
    ParseState,
    Word,
    Warning,
    Suggestion,
    Nesting,
};

pub struct DeadBranchCommentLint {
}
impl Lint for DeadBranchCommentLint {
    fn name(&self) -> &'static str { "dead-comment" }
    fn run_inside_comments(&self) -> bool { true }
    fn lint_token(&mut self, state: &mut ParseState, token: &Word) -> Option<Warning> {
        if token.value != "/*" { return None; }

        state.nesting.iter()
            .find_map(|n| if let Nesting::StartRandom(loc) = n {
                Some(token.warning("Using comments inside `start_random` groups is potentially dangerous.".to_string())
                    .note_at(*loc, "`start_random` opened here")
                    .suggest(Suggestion::from(token, "Only #define constants in the `start_random` group, and then use `if` branches for the actual code.".to_string())))
            } else {
                None
            })
    }
}
