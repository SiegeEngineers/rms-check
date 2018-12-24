use super::super::{
    Lint,
    ParseState,
    Word,
    Warning,
    Suggestion,
};

pub struct IncludeLint {
}

impl Lint for IncludeLint {
    fn name(&self) -> &'static str { "include" }
    fn lint_token(&mut self, _state: &mut ParseState, token: &Word) -> Option<Warning> {
        match token.value {
            "#include_drs" => {
                Some(token.error("#include_drs can only be used by builtin maps"))
            },
            "#include" => {
                let suggestion = Suggestion::from(token, "If you're trying to make a map pack, use a map pack generator instead.");
                Some(token.error("#include can only be used by builtin maps").suggest(suggestion))
            },
            _ => None,
        }
    }
}
