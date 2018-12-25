use super::super::{
    Lint,
    ParseState,
    Word,
    Warning,
    Suggestion,
};

pub struct IncludeLint {
}

impl IncludeLint {
    pub fn new() -> Self {
        IncludeLint {}
    }
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use super::super::super::{RMSCheck, Severity};
    use super::IncludeLint;

    #[test]
    fn include() {
        let result = RMSCheck::new()
            .with_lint(Box::new(IncludeLint::new()))
            .add_file(PathBuf::from("./tests/rms/include.rms")).unwrap()
            .check();

        let mut warnings = result.iter();
        let first = warnings.next().unwrap();
        let second = warnings.next().unwrap();
        assert!(warnings.next().is_none());
        assert_eq!(first.diagnostic().severity, Severity::Error);
        assert_eq!(first.diagnostic().code, Some("include".to_string()));
        assert_eq!(first.message(), "#include_drs can only be used by builtin maps");
        // assert_eq!(first.diagnostic().start().0.number(), 1);
        // assert_eq!(first.diagnostic().start().1.number(), 1);
        assert_eq!(second.diagnostic().severity, Severity::Error);
        assert_eq!(second.diagnostic().code, Some("include".to_string()));
        assert_eq!(second.message(), "#include can only be used by builtin maps");
        // assert_eq!(second.diagnostic().start().0.number(), 3);
        // assert_eq!(second.diagnostic().start().1.number(), 1);
    }
}
