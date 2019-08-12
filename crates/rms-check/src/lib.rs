#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(unused)]

mod checker;
mod lints;
mod parser;
mod tokens;
mod wordize;

use crate::{checker::Checker, wordize::Wordize};
pub use crate::{
    checker::{
        AutoFixReplacement, Compatibility, Lint, Nesting, ParseState, Severity, Suggestion, Warning,
    },
    parser::{Atom, Parser, WarningKind},
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
    #[inline]
    pub fn codemap(&self) -> &CodeMap {
        &self.codemap
    }

    #[inline]
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    #[inline]
    pub fn resolve_position(&self, index: ByteIndex) -> Option<(LineIndex, ColumnIndex)> {
        let file = self.codemap.find_file(index);
        file.and_then(|f| f.location(index).ok())
    }

    #[inline]
    pub fn resolve_offset(&self, index: ByteIndex) -> Option<ByteOffset> {
        let file = self.codemap.find_file(index);
        file.and_then(|f| {
            f.location(index)
                .ok()
                .and_then(|(l, c)| f.offset(l, c).ok())
        })
    }

    #[inline]
    pub fn into_iter(self) -> impl IntoIterator<Item = Warning> {
        self.warnings.into_iter()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Warning> {
        self.warnings.iter()
    }
}

pub struct RMSCheck<'a> {
    checker: Checker<'a>,
    codemap: CodeMap,
    file_maps: Vec<Arc<FileMap>>,
    binary_files: HashMap<String, Vec<u8>>,
    compatibility: Compatibility,
}

impl<'a> Default for RMSCheck<'a> {
    fn default() -> RMSCheck<'a> {
        RMSCheck::new()
            .with_lint(Box::new(lints::ArgTypesLint::new()))
            .with_lint(Box::new(lints::AttributeCaseLint {}))
            .with_lint(Box::new(lints::CommentContentsLint::new()))
            .with_lint(Box::new(lints::CompatibilityLint::new()))
            .with_lint(Box::new(lints::DeadBranchCommentLint {}))
            .with_lint(Box::new(lints::IncludeLint::new()))
            .with_lint(Box::new(lints::IncorrectSectionLint::new()))
        // .with_lint(Box::new(lints::UnknownAttributeLint {}))
        // buggy
    }
}

impl<'a> RMSCheck<'a> {
    #[inline]
    pub fn new() -> Self {
        RMSCheck {
            checker: Checker::default(),
            codemap: CodeMap::new(),
            file_maps: Default::default(),
            binary_files: Default::default(),
            compatibility: Default::default(),
        }
    }

    #[inline]
    pub fn compatibility(self, compatibility: Compatibility) -> Self {
        Self {
            checker: self.checker.compatibility(compatibility),
            compatibility,
            ..self
        }
    }

    #[inline]
    pub fn with_lint(self, lint: Box<dyn Lint>) -> Self {
        Self {
            checker: self.checker.with_lint(lint),
            ..self
        }
    }

    /// Add a binary file.
    #[inline]
    pub fn add_binary(mut self, name: &str, source: Vec<u8>) -> Self {
        self.binary_files.insert(name.to_string(), source);
        self
    }

    /// Add a source string.
    #[inline]
    pub fn add_source(mut self, name: &str, source: &str) -> Self {
        let map = self.codemap.add_filemap(
            FileName::Virtual(name.to_string().into()),
            source.to_string(),
        );
        self.file_maps.push(map);
        self
    }

    /// Add a definitions file, parsed before any other files.
    fn add_definitions(&mut self, name: &str, source: &str) {
        let map = self.codemap.add_filemap(
            FileName::Virtual(name.to_string().into()),
            source.to_string(),
        );
        self.file_maps.insert(0, map);
    }

    /// Add a source string from disk.
    #[inline]
    pub fn add_file(mut self, path: PathBuf) -> Result<Self> {
        let map = self.codemap.add_filemap_from_disk(path)?;
        self.file_maps.push(map);
        Ok(self)
    }

    /// Get the internal CodeMap, useful for converting byte indices.
    #[inline]
    pub fn codemap(&self) -> &CodeMap {
        &self.codemap
    }

    /// Run the lints and get the result.
    pub fn check(mut self) -> RMSCheckResult {
        match self.compatibility {
            Compatibility::WololoKingdoms => {
                self.add_definitions("random_map.def", include_str!("def_wk.rms"));
            }
            Compatibility::UserPatch15 =>{
                self.add_definitions("random_map.def", include_str!("def_aoc.rms"));
                self.add_definitions("UserPatchConst.rms", include_str!("def_up15.rms"));
            }
            _ => {
                self.add_definitions("random_map.def", include_str!("def_aoc.rms"));
            }
        };

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
                    if w.kind == WarningKind::MissingCommandArgs {
                        // Handled by arg-types lint
                        continue;
                    }
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
#[inline]
pub fn check(source: &str, compatibility: Compatibility) -> RMSCheckResult {
    let checker = RMSCheck::default()
        .compatibility(compatibility)
        .add_source("source.rms", source);

    checker.check()
}
