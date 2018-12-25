use codespan::ByteSpan;
use super::super::{
    Lint,
    ParseState,
    Word,
    Warning,
    Suggestion,
    TokenType,
    TOKENS,
};

#[derive(Default)]
pub struct CommentContentsLint {
    current_command: Option<(ByteSpan, &'static TokenType, u8)>,
}

impl CommentContentsLint {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Lint for CommentContentsLint {
    fn name(&self) -> &'static str { "comment-contents" }
    fn run_inside_comments(&self) -> bool { true }
    fn lint_token(&mut self, state: &mut ParseState, token: &Word) -> Option<Warning> {
        if !state.is_comment { return None; }

        self.current_command = self.current_command.and_then(|(s, t, args)|
            if args > 0 {
                Some((s, t, args - 1))
            } else {
                None
            });
        if let Some((span, tt, remaining)) = self.current_command {
            if token.value == "*/" {
                let suggestion = Suggestion::new(span.start(), span.end(), "Add `backticks` around the command to make the parser ignore it")
                    .replace(format!("`{}`", tt.name));
                return Some(token.warning(format!("This close comment may be ignored because a previous command is expecting {} more argument(s)", remaining))
                            .note_at(span, "Command started here")
                            .suggest(suggestion));
            }
        }

        if self.current_command.is_none() && (state.has_define(token.value) || state.has_const(token.value)) {
            let suggestion = Suggestion::from(token, "Add `backticks` around the name to make the parser ignore it")
                .replace(format!("`{}`", token.value));
            return Some(token.warning("Using constant names in comments can be dangerous, because the game may interpret them as other tokens instead.")
                        .suggest(suggestion));
        }

        if let Some(command) = TOKENS.get(token.value) {
            if command.arg_len() > 0 {
                self.current_command = Some((token.span, &command, command.arg_len()));
            }
        }

        None
    }
}
