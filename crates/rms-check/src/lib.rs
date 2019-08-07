mod checker;
mod lints;
mod parser;
mod tokens;
mod wordize;

use crate::{checker::Checker, wordize::Wordize};
pub use crate::{
    checker::{
        AutoFixReplacement, Compatibility, Expect, Lint, Nesting, ParseState, Severity, Suggestion,
        Warning,
    },
    parser::{Atom, Parser},
    tokens::{ArgType, TokenContext, TokenType, TOKENS},
    wordize::Word,
};
use codespan::{ByteIndex, ByteOffset, CodeMap, ColumnIndex, FileMap, FileName, LineIndex};
use std::{collections::HashMap, io::Result, path::PathBuf, sync::Arc};

pub struct RMSCheckResult {
    warnings: Vec<Warning>,
    codemap: CodeMap,
}

impl RMSCheckResult {
    pub fn codemap(&self) -> &CodeMap {
        &self.codemap
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn resolve_position(&self, index: ByteIndex) -> Option<(LineIndex, ColumnIndex)> {
        let file = self.codemap.find_file(index);
        file.and_then(|f| f.location(index).ok())
    }

    pub fn resolve_offset(&self, index: ByteIndex) -> Option<ByteOffset> {
        let file = self.codemap.find_file(index);
        file.and_then(|f| {
            f.location(index)
                .ok()
                .and_then(|(l, c)| f.offset(l, c).ok())
        })
    }

    pub fn into_iter(self) -> impl IntoIterator<Item = Warning> {
        self.warnings.into_iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Warning> {
        self.warnings.iter()
    }
}

pub struct RMSCheck<'a> {
    checker: Checker<'a>,
    codemap: CodeMap,
    file_maps: Vec<Arc<FileMap>>,
    binary_files: HashMap<String, Vec<u8>>,
}

impl<'a> Default for RMSCheck<'a> {
    fn default() -> RMSCheck<'a> {
        RMSCheck::new()
            .with_lint(Box::new(lints::AttributeCaseLint {}))
            .with_lint(Box::new(lints::CommentContentsLint::new()))
            .with_lint(Box::new(lints::CompatibilityLint::new()))
            .with_lint(Box::new(lints::DeadBranchCommentLint {}))
            .with_lint(Box::new(lints::IncludeLint::new()))
            .with_lint(Box::new(lints::IncorrectSectionLint {}))
        // .with_lint(Box::new(lints::UnknownAttributeLint {}))
        // buggy
    }
}

impl<'a> RMSCheck<'a> {
    pub fn new() -> Self {
        let check = RMSCheck {
            checker: Checker::default(),
            codemap: CodeMap::new(),
            file_maps: Default::default(),
            binary_files: Default::default(),
        };
        check.add_source("random_map.def", include_str!("random_map.def"))
    }

    pub fn compatibility(self, compatibility: Compatibility) -> Self {
        Self {
            checker: self.checker.compatibility(compatibility),
            ..self
        }
    }

    pub fn with_lint(self, lint: Box<Lint>) -> Self {
        Self {
            checker: self.checker.with_lint(lint),
            ..self
        }
    }

    pub fn add_binary(mut self, name: &str, source: Vec<u8>) -> Self {
        self.binary_files.insert(name.to_string(), source);
        self
    }

    pub fn add_source(mut self, name: &str, source: &str) -> Self {
        let map = self.codemap.add_filemap(
            FileName::Virtual(name.to_string().into()),
            source.to_string(),
        );
        self.file_maps.push(map);
        self
    }

    pub fn add_file(mut self, path: PathBuf) -> Result<Self> {
        let map = self.codemap.add_filemap_from_disk(path)?;
        self.file_maps.push(map);
        Ok(self)
    }

    pub fn codemap(&self) -> &CodeMap {
        &self.codemap
    }

    pub fn check(self) -> RMSCheckResult {
        let mut checker = self.checker.build();
        let words = self
            .file_maps
            .iter()
            .map(|map| Wordize::new(&map))
            .flatten();

        let mut warnings: Vec<Warning> = words.filter_map(|w| checker.write_token(&w)).collect();
        let parsers = self.file_maps.iter().map(|map| Parser::new(&map));
        for parser in parsers {
            for (atom, parse_warning) in parser {
                warnings.extend(checker.write_atom(&atom));
                for w in parse_warning {
                    warnings.push(Warning::error(w.span, format!("{:?}", w.kind)).lint("parse"));
                }
            }
        }

        RMSCheckResult {
            codemap: self.codemap,
            warnings,
        }
    }
}

/// Check a random map script for errors or other issues.
pub fn check(source: &str, compatibility: Compatibility) -> RMSCheckResult {
    let checker = RMSCheck::default()
        .compatibility(compatibility)
        .add_source("source.rms", source);

    checker.check()
}
