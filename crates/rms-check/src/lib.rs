//!
#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]
// #![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::missing_const_for_fn)]

mod checker;
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

#[derive(Debug)]
pub struct RMSFile {
    files: Files,
    main_file: FileId,
    /// File ID of the AoC random_map.def file.
    def_aoc: FileId,
    /// File ID of the HD Edition random_map.def file.
    def_hd: FileId,
    /// File ID of the WololoKingdoms random_map.def file.
    def_wk: FileId,
}

impl RMSFile {
    fn new(mut files: Files, main_file: FileId) -> Self {
        let def_aoc = files.add("random_map.def", include_str!("def_aoc.rms"));
        let def_hd = files.add("random_map.def", include_str!("def_hd.rms"));
        let def_wk = files.add("random_map.def", include_str!("def_wk.rms"));

        Self {
            files,
            main_file,
            def_aoc,
            def_hd,
            def_wk,
        }
    }

    pub fn from_path(name: impl AsRef<Path>) -> Result<Self> {
        let source = std::fs::read(name.as_ref())?;
        let source = std::str::from_utf8(&source).unwrap(); // TODO do not unwrap
        Ok(Self::from_string(name.as_ref().to_string_lossy(), source))
    }

    pub fn from_string(name: impl AsRef<str>, source: impl AsRef<str>) -> Self {
        let mut files = Files::new();
        let main_file = files.add(name.as_ref(), source.as_ref());
        Self::new(files, main_file)
    }

    // pub fn from_unpacked_zr(path: impl AsRef<Path>) -> Result<Self> {}
    // pub fn from_binary(name: impl AsRef<str>, source: &[u8]) -> Result<Self> {}

    pub(crate) fn definitions(&self, compatibility: Compatibility) -> (FileId, &str) {
        match compatibility {
            Compatibility::WololoKingdoms => (self.def_wk, self.files.source(self.def_wk)),
            Compatibility::HDEdition => (self.def_hd, self.files.source(self.def_hd)),
            _ => (self.def_aoc, self.files.source(self.def_aoc)),
        }
    }

    const fn file_id(&self) -> FileId {
        self.main_file
    }

    fn main_source(&self) -> &str {
        self.files.source(self.main_file)
    }

    fn find_file(&self, name: &str) -> Option<FileId> {
        if self.files.name(self.main_file) == name {
            Some(self.main_file)
        } else {
            None
        }
    }

    pub(crate) const fn files(&self) -> &Files {
        &self.files
    }
}

/// The result of a lint run.
pub struct RMSCheckResult {
    warnings: Vec<Warning>,
    rms: RMSFile,
}

impl RMSCheckResult {
    /// The files that were linted, and a list of the file IDs so they can be iterated over.
    #[inline]
    pub const fn files(&self) -> &Files {
        self.rms.files()
    }

    /// Get the codespan file ID for a given file name.
    pub fn file_id(&self, name: &str) -> Option<FileId> {
        self.rms.find_file(name)
    }

    /// Get a file's source code by the file name.
    #[inline]
    pub fn file(&self, name: &str) -> Option<&str> {
        self.rms.find_file(name).map(|id| self.files().source(id))
    }

    pub fn main_source(&self) -> &str {
        self.rms.main_source()
    }

    /// Were there any warnings?
    #[inline]
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Resolve a file ID and byte index to a Line/Column location pair.
    #[inline]
    pub fn resolve_position(&self, file_id: FileId, index: ByteIndex) -> Option<Location> {
        self.rms.files().location(file_id, index).ok()
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
            .with_lint(Box::new(lints::UnknownAttributeLint {}))
    }
}

impl RMSCheck {
    /// Initialize an RMS checker.
    #[inline]
    pub fn new() -> Self {
        RMSCheck {
            checker: Checker::builder(),
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

    /// Run the lints and get the result.
    pub fn check(self, rms: RMSFile) -> RMSCheckResult {
        let mut checker = self.checker.build(&rms);

        let words = Wordize::new(rms.file_id(), rms.main_source());
        let mut warnings: Vec<Warning> = words.filter_map(|w| checker.write_token(&w)).collect();

        let parser = Parser::new(rms.file_id(), rms.main_source());
        for (atom, parse_warning) in parser {
            warnings.extend(checker.write_atom(&atom));
            for w in parse_warning {
                if w.kind == ParseErrorKind::MissingCommandArgs {
                    // Handled by arg-types lint
                    continue;
                }
                warnings.push(
                    Warning::error(atom.file_id(), w.span, format!("{:?}", w.kind)).lint("parse"),
                );
            }
        }

        RMSCheckResult { rms, warnings }
    }
}

/// Check a random map script for errors or other issues.
#[inline]
pub fn check(source: &str, compatibility: Compatibility) -> RMSCheckResult {
    let checker = RMSCheck::default().compatibility(compatibility);
    let file = RMSFile::from_string("random.rms", source);

    checker.check(file)
}
