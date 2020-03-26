//! Entry point for linting AoE2 random map scripts.

#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]
// #![warn(missing_docs)]
#![warn(unused)]

mod checker;
mod formatter;
mod lints;
mod parser;
mod state;
mod tokens;
mod wordize;

use crate::checker::Checker;
pub use crate::checker::{AutoFixReplacement, CheckerBuilder, Lint, Severity, Suggestion, Warning};
pub use crate::formatter::{format, FormatOptions};
pub use crate::parser::{Atom, AtomKind, ParseErrorKind, Parser};
pub use crate::state::{Compatibility, Nesting, ParseState};
pub use crate::tokens::{ArgType, TokenContext, TokenType, TOKENS};
pub use crate::wordize::Word;
use codespan::{ByteIndex, FileId, Files, Location};
use encoding_rs::Encoding;
use std::{borrow::Cow, fs::File, io, path::Path};
use zip::ZipArchive;

fn to_chardet_string(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap_or_else(|err| {
        let bytes = err.as_bytes();
        let (encoding_name, _, _) = chardet::detect(bytes);
        if let Some(encoding) = Encoding::for_label(encoding_name.as_bytes()) {
            encoding.decode(bytes).0.to_string()
        } else {
            String::from_utf8_lossy(bytes).to_string()
        }
    })
}

/// Container for a random map script, generalising various formats.
#[derive(Debug)]
pub struct RMSFile<'source> {
    files: Files<Cow<'source, str>>,
    file_ids: Vec<FileId>,
    /// File ID of the AoC random_map.def file.
    def_aoc: FileId,
    /// File ID of the HD Edition random_map.def file.
    def_hd: FileId,
    /// File ID of the WololoKingdoms random_map.def file.
    def_wk: FileId,
}

impl<'source> RMSFile<'source> {
    fn new(mut files: Files<Cow<'source, str>>, file_ids: Vec<FileId>) -> Self {
        let def_aoc = files.add("random_map.def", include_str!("def_aoc.rms").into());
        let def_hd = files.add("random_map.def", include_str!("def_hd.rms").into());
        let def_wk = files.add("random_map.def", include_str!("def_wk.rms").into());

        Self {
            files,
            file_ids,
            def_aoc,
            def_hd,
            def_wk,
        }
    }

    /// Create an RMSFile from a file path.
    pub fn from_path(name: impl AsRef<Path>) -> io::Result<Self> {
        let source = std::fs::read(name.as_ref())?;
        let filename = name
            .as_ref()
            .file_name()
            .expect("must pass a file path to `RMSFile::from_path`")
            .to_string_lossy();
        if filename.starts_with("ZR@") {
            Self::from_zip_rms(name.as_ref().to_string_lossy(), &source)
        } else {
            let source = to_chardet_string(source);
            Ok(Self::from_string(name.as_ref().to_string_lossy(), source))
        }
    }

    /// Create an RMSFile from a source string.
    pub fn from_string(name: impl AsRef<str>, source: impl Into<Cow<'source, str>>) -> Self {
        let mut files = Files::new();
        let main_file = files.add(name.as_ref(), source.into());
        Self::new(files, vec![main_file])
    }

    fn from_zip_rms_reader<R>(_name: impl AsRef<str>, reader: R) -> io::Result<Self>
    where
        R: io::Read + io::Seek,
    {
        let mut zip = ZipArchive::new(reader)?;
        let mut files = Files::new();
        let mut file_ids = vec![];
        for index in 0..zip.len() {
            let mut file = zip.by_index(index)?;
            let mut bytes = vec![];
            std::io::copy(&mut file, &mut bytes)?;
            if file.name().ends_with(".rms") || file.name().ends_with(".inc") {
                let source = to_chardet_string(bytes);
                file_ids.push(files.add(file.name(), Cow::Owned(source)));
                // If this is an .rms file, move it to the front so main_file() does the right thing
                if file.name().ends_with(".rms") {
                    file_ids.rotate_right(1);
                }
            }
        }

        Ok(Self::new(files, file_ids))
    }

    /// Create an RMSFile from a string of bytes containing a ZR@ map.
    pub fn from_zip_rms(name: impl AsRef<str>, source: &[u8]) -> io::Result<Self> {
        Self::from_zip_rms_reader(name, io::Cursor::new(source))
    }

    /// Create an RMSFile from a folder containing files intended for a ZR@ map.
    pub fn from_zip_rms_path_unpacked(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut files = Files::new();
        let mut file_ids = vec![];
        for entry in std::fs::read_dir(path)? {
            let path = entry?.path();
            let name = path.to_string_lossy();
            let bytes = std::fs::read(&path)?;
            let source = to_chardet_string(bytes);
            file_ids.push(files.add(name.as_ref(), Cow::Owned(source)));
            // If this is an .rms file, move it to the front so main_file() does the right thing
            if name.ends_with(".rms") {
                file_ids.rotate_right(1);
            }
        }

        Ok(Self::new(files, file_ids))
    }

    /// Create an RMSFile from a file path containing a ZR@ map.
    pub fn from_zip_rms_path(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::from_zip_rms_reader(path.as_ref().to_string_lossy(), File::open(path.as_ref())?)
    }

    // pub fn from_bytes(name: impl AsRef<str>, source: &[u8]) -> io::Result<Self> {}

    /// Get the definitions file for this map.
    pub(crate) fn definitions(&self, compatibility: Compatibility) -> (FileId, &str) {
        match compatibility {
            Compatibility::WololoKingdoms => (self.def_wk, self.files.source(self.def_wk)),
            Compatibility::HDEdition => (self.def_hd, self.files.source(self.def_hd)),
            _ => (self.def_aoc, self.files.source(self.def_aoc)),
        }
    }

    /// Get the codespan FileIds in this map.
    pub fn file_ids(&self) -> &[FileId] {
        &self.file_ids
    }

    /// Get the codespan FileId of the main script in this map.
    fn file_id(&self) -> FileId {
        self.file_ids[0]
    }

    /// Get the source code of the main script in this map.
    fn main_source(&self) -> &str {
        self.files.source(self.file_ids[0])
    }

    /// Get the codespan FileId for a file with the given name in this map (mostly for ZR@ maps).
    fn find_file(&self, name: &str) -> Option<FileId> {
        self.file_ids
            .iter()
            .cloned()
            .find(|&id| self.files.name(id) == name)
    }

    /// Get the codespan Files instance.
    pub(crate) const fn files(&self) -> &Files<Cow<'source, str>> {
        &self.files
    }

    fn is_zip_rms(&self) -> bool {
        let name = self.files.name(self.file_id()).to_string_lossy();
        name.starts_with("ZR@")
    }
}

/// The result of a lint run.
pub struct RMSCheckResult<'source> {
    warnings: Vec<Warning>,
    rms: RMSFile<'source>,
}

impl<'source> RMSCheckResult<'source> {
    /// The files that were linted, and a list of the file IDs so they can be iterated over.
    pub const fn files(&self) -> &Files<Cow<'source, str>> {
        self.rms.files()
    }

    /// Get the codespan file ID for a given file name.
    pub fn file_id(&self, name: &str) -> Option<FileId> {
        self.rms.find_file(name)
    }

    /// Get a file's source code by the file name.
    pub fn file(&self, name: &str) -> Option<&str> {
        self.rms
            .find_file(name)
            .map(|id| self.files().source(id).as_ref())
    }

    pub fn main_source(&self) -> &str {
        self.rms.main_source()
    }

    /// Were there any warnings?
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Resolve a file ID and byte index to a Line/Column location pair.
    pub fn resolve_position(&self, file_id: FileId, index: ByteIndex) -> Option<Location> {
        self.rms.files().location(file_id, index).ok()
    }

    /// Iterate over the warnings.
    pub fn iter(&self) -> impl Iterator<Item = &Warning> {
        self.warnings.iter()
    }
}

impl IntoIterator for RMSCheckResult<'_> {
    type Item = Warning;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    /// Iterate over the warnings.
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
            .with_lint(Box::new(lints::IncludeLint::new()))
            .with_lint(Box::new(lints::IncorrectSectionLint::new()))
            .with_lint(Box::new(lints::UnknownAttributeLint {}))
    }
}

impl RMSCheck {
    /// Initialize an RMS checker.
    pub fn new() -> Self {
        RMSCheck {
            checker: Checker::builder(),
        }
    }

    /// Configure the default compatibility for the script.
    ///
    /// The compatibility setting can be overridden by scripts using `Compatibility: ` comments.
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn compatibility(self, compatibility: Compatibility) -> Self {
        Self {
            checker: self.checker.compatibility(compatibility),
        }
    }

    /// Add a lint rule.
    pub fn with_lint(self, lint: Box<dyn Lint>) -> Self {
        Self {
            checker: self.checker.with_lint(lint),
        }
    }

    /// Run the lints and get the result.
    pub fn check(self, rms: RMSFile<'_>) -> RMSCheckResult<'_> {
        let mut checker = self.checker.build(&rms);

        let mut warnings = vec![];

        let parser = Parser::new(rms.file_id(), rms.main_source());
        for (atom, parse_warning) in parser {
            warnings.extend(checker.write_atom(&atom));
            for w in parse_warning {
                if w.kind == ParseErrorKind::MissingCommandArgs {
                    // Handled by arg-types lint
                    continue;
                }
                warnings
                    .push(Warning::error(atom.file, w.span, format!("{:?}", w.kind)).lint("parse"));
            }
        }

        RMSCheckResult { rms, warnings }
    }
}
