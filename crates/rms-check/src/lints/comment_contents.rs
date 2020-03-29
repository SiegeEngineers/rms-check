use crate::diagnostic::{ByteIndex, Diagnostic, Fix, Label, SourceLocation};
use crate::{Atom, AtomKind, Compatibility, Lint, Nesting, ParseErrorKind, ParseState, Parser};
use std::ops::Range;

fn offset_range(span: Range<ByteIndex>, offset: isize) -> Range<ByteIndex> {
    Range {
        start: span.start + offset,
        end: span.end + offset,
    }
}

#[derive(Default)]
pub struct CommentContentsLint {}

impl CommentContentsLint {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Lint for CommentContentsLint {
    fn name(&self) -> &'static str {
        "comment-contents"
    }

    fn run_inside_comments(&self) -> bool {
        true
    }

    fn lint_atom(&mut self, state: &mut ParseState<'_>, atom: &Atom<'_>) -> Vec<Diagnostic> {
        if let AtomKind::Comment {
            open,
            content,
            close,
        } = &atom.kind
        {
            let offset = usize::from(open.end()) as isize;

            let (has_start_random, has_if) =
                state
                    .nesting
                    .iter()
                    .fold((false, false), |acc, nest| match nest {
                        Nesting::If(_) | Nesting::ElseIf(_) | Nesting::Else(_) => (acc.0, true),
                        Nesting::StartRandom(_) | Nesting::PercentChance(_) => (true, acc.1),
                        _ => acc,
                    });

            let may_trigger_parsing_bug = has_if
                && state.compatibility <= Compatibility::UserPatch14
                || has_start_random && state.compatibility <= Compatibility::UserPatch15;

            let parser = Parser::new(state.rms.file_id(), &content);
            let mut warnings = vec![];

            let mut expecting_more_arguments = None;
            'outer: for (atom, errors) in parser {
                for error in errors {
                    use ParseErrorKind::*;
                    match error.kind {
                        MissingConstName | MissingConstValue | MissingDefineName
                        | MissingCommandArgs | MissingIfCondition | MissingPercentChance => {
                            expecting_more_arguments = Some(atom);
                            continue 'outer;
                        }
                        _ => (),
                    }
                }

                if let AtomKind::Other { value } = atom.kind {
                    if may_trigger_parsing_bug
                        && (state.has_define(value.value) || state.has_const(value.value))
                    {
                        let suggestion = Fix::new(
                            value.location,
                            "Add `backticks` around the name to make the parser ignore it",
                        )
                        .replace(format_args!("`{}`", value.value));
                        warnings.push(Diagnostic::warning(SourceLocation::new(value.location.file(), offset_range(value.location.range(), offset)),
                                                      "Using constant names in comments inside `start_random` or `if` statements can be dangerous, because the game may interpret them as other tokens instead.")
                                    .suggest(suggestion));
                    }
                }

                expecting_more_arguments = None;
            }

            if let (Some(atom), Some(close_comment)) = (&expecting_more_arguments, close) {
                warnings.push(Diagnostic::warning(close_comment.location, "This close comment may be ignored because a previous command is expecting more arguments")
                              .add_label(Label::new(SourceLocation::new(atom.location.file(), offset_range(atom.location.range(), offset)), "Command started here")));
            }

            return warnings;
        }

        return vec![];
    }
}

#[cfg(test)]
mod tests {
    use super::CommentContentsLint;
    use crate::{RMSCheck, RMSFile, Severity};

    #[test]
    fn comment_contents() {
        let file = RMSFile::from_path("./tests/rms/comment-contents.rms").unwrap();
        let result = RMSCheck::new()
            .with_lint(Box::new(CommentContentsLint::new()))
            .check(file);

        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        let second = warnings.next().unwrap();
        let third = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.severity(), Severity::Warning);
        assert_eq!(first.code(), Some("comment-contents"));
        assert_eq!(first.message(), "This close comment may be ignored because a previous command is expecting more arguments");
        assert_eq!(second.severity(), Severity::Warning);
        assert_eq!(second.code(), Some("comment-contents"));
        assert_eq!(second.message(), "This close comment may be ignored because a previous command is expecting more arguments");
        assert_eq!(third.severity(), Severity::Warning);
        assert_eq!(third.code(), Some("comment-contents"));
        assert_eq!(third.message(), "Using constant names in comments inside `start_random` or `if` statements can be dangerous, because the game may interpret them as other tokens instead.");
    }
}
