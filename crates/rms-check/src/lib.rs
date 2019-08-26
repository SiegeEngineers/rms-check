//!
#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]
// #![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::missing_const_for_fn)]

mod checker;
mod formatter;
mod lints;
mod parser;
mod tokens;
mod wordize;

use crate::{checker::Checker, wordize::Wordize};
pub use crate::{
    checker::{
        AutoFixReplacement, CheckerBuilder, Compatibility, Lint, Nesting, ParseState, Severity,
        Suggestion, Warning,
    },
    formatter::format,
    parser::{Atom, ParseErrorKind, Parser},
    tokens::{ArgType, TokenContext, TokenType, TOKENS},
    wordize::Word,
};
use codespan::{ByteIndex, FileId, Files, Location};
use std::{
    collections::HashMap,
    io::{self, Result},
    path::Path,
};

/// The result of a lint run.
pub struct RMSCheckResult {
    warnings: Vec<Warning>,
    files: Files,
    file_ids: Vec<FileId>,
}

impl RMSCheckResult {
    /// The files that were linted, and a list of the file IDs so they can be iterated over.
    #[inline]
    pub fn files(&self) -> (&[FileId], &Files) {
        (&self.file_ids, &self.files)
    }

    /// Get the codespan file ID for a given file name.
    pub fn file_id(&self, name: &str) -> Option<FileId> {
        self.file_ids
            .iter()
            .cloned()
            .find(|&id| self.files.name(id) == name)
    }

    /// Get a file's source code by the file name.
    pub fn file(&self, name: &str) -> Option<&str> {
        self.file_id(name).map(|id| self.files.source(id))
    }

    /// Were there any warnings?
    #[inline]
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Resolve a file ID and byte index to a Line/Column location pair.
    #[inline]
    pub fn resolve_position(&self, file_id: FileId, index: ByteIndex) -> Option<Location> {
        self.files.location(file_id, index).ok()
    }

    /// Iterate over the warnings.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Warning> {
        self.warnings.iter()
    }
}

impl IntoIterator for RMSCheckResult {
    type Item = Warning;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    /// Iterate over the warnings.
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.warnings.into_iter()
    }
}

///
pub struct RMSCheck {
    checker: CheckerBuilder,
    files: Files,
    file_ids: Vec<FileId>,
    binary_files: HashMap<String, Vec<u8>>,
}

impl Default for RMSCheck {
    fn default() -> RMSCheck {
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

impl RMSCheck {
    /// Initialize an RMS checker.
    #[inline]
    pub fn new() -> Self {
        RMSCheck {
            checker: Checker::builder(),
            files: Files::new(),
            file_ids: Default::default(),
            binary_files: Default::default(),
        }
    }

    /// Configure the default compatibility for the script.
    ///
    /// The compatibility setting can be overridden by scripts using `Compatibility: ` comments.
    #[inline]
    pub fn compatibility(self, compatibility: Compatibility) -> Self {
        Self {
            checker: self.checker.compatibility(compatibility),
            ..self
        }
    }

    /// Add a lint rule.
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
        let file_id = self.files.add(name, source);
        self.file_ids.push(file_id);
        self
    }

    /// Add a source string from disk.
    #[inline]
    pub fn add_file(mut self, path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path.as_ref())?;
        let source =
            std::str::from_utf8(&bytes).map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let file_id = self.files.add(path.as_ref().to_string_lossy(), source);
        self.file_ids.push(file_id);
        Ok(self)
    }

    /// Get the internal Files, useful for converting byte indices.
    #[inline]
    pub const fn files(&self) -> &Files {
        &self.files
    }

    /// Run the lints and get the result.
    pub fn check(self) -> RMSCheckResult {
        let Self {
            mut files,
            file_ids,
            ..
        } = self;
        let def_aoc = files.add("random_map.def", include_str!("def_aoc.rms"));
        let def_hd = files.add("random_map.def", include_str!("def_hd.rms"));
        let def_wk = files.add("random_map.def", include_str!("def_wk.rms"));
        let mut checker = self.checker.build(&files, (def_aoc, def_hd, def_wk));
        let words = file_ids
            .iter()
            .cloned()
            .map(|file_id| Wordize::new(file_id, files.source(file_id)))
            .flatten();

        let mut warnings: Vec<Warning> = words.filter_map(|w| checker.write_token(&w)).collect();
        let parsers = file_ids
            .iter()
            .map(|&file_id| Parser::new(file_id, files.source(file_id)));
        for parser in parsers {
            for (atom, parse_warning) in parser {
                warnings.extend(checker.write_atom(&atom));
                for w in parse_warning {
                    if w.kind == ParseErrorKind::MissingCommandArgs {
                        // Handled by arg-types lint
                        continue;
                    }
                    warnings.push(
                        Warning::error(atom.file_id(), w.span, format!("{:?}", w.kind))
                            .lint("parse"),
                    );
                }
            }
        }

        RMSCheckResult {
            files,
            file_ids,
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
