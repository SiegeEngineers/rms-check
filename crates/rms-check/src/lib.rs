//! Entry point for linting AoE2 random map scripts.

#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]
// #![warn(missing_docs)]
#![warn(unused)]

mod checker;
mod diagnostic;
mod formatter;
mod lints;
mod parser;
mod state;
mod tokens;
mod wordize;

use crate::checker::Checker;
pub use crate::checker::{CheckerBuilder, Lint};
pub use crate::diagnostic::{ByteIndex, Diagnostic, FileId, Fix, Severity, SourceLocation};
pub use crate::formatter::{format, FormatOptions};
pub use crate::parser::{Atom, AtomKind, ParseErrorKind, Parser};
pub use crate::state::{Compatibility, Nesting, ParseState};
pub use crate::tokens::{ArgType, TokenContext, TokenType, TOKENS};
pub use crate::wordize::Word;
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

#[derive(Debug, Clone)]
struct FileData<'source> {
    name: String,
    source: Cow<'source, str>,
}

impl<'source> FileData<'source> {
    fn new(name: String, source: Cow<'source, str>) -> Self {
        Self { name, source }
    }
}

/// Container for a random map script, generalising various formats.
#[derive(Debug)]
pub struct RMSFile<'source> {
    files: Vec<FileData<'source>>,
    /// File ID of the AoC random_map.def file.
    def_aoc: FileId,
    /// File ID of the HD Edition random_map.def file.
    def_hd: FileId,
    /// File ID of the WololoKingdoms random_map.def file.
    def_wk: FileId,
}

impl<'source> RMSFile<'source> {
    fn new(mut files: Vec<FileData<'source>>) -> Self {
        let def_aoc = FileId::new(files.len() as u32);
        files.push(FileData::new(
            "random_map.def".into(),
            include_str!("def_aoc.rms").into(),
        ));
        let def_hd = FileId::new(files.len() as u32);
        files.push(FileData::new(
            "random_map.def".into(),
            include_str!("def_hd.rms").into(),
        ));
        let def_wk = FileId::new(files.len() as u32);
        files.push(FileData::new(
            "random_map.def".into(),
            include_str!("def_wk.rms").into(),
        ));

        Self {
            files,
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
    pub fn from_string(name: impl ToString, source: impl Into<Cow<'source, str>>) -> Self {
        Self::new(vec![FileData::new(name.to_string(), source.into())])
    }

    fn from_zip_rms_reader<R>(_name: impl AsRef<str>, reader: R) -> io::Result<Self>
    where
        R: io::Read + io::Seek,
    {
        let mut zip = ZipArchive::new(reader)?;
        let mut files = vec![];
        for index in 0..zip.len() {
            let mut file = zip.by_index(index)?;
            let mut bytes = vec![];
            std::io::copy(&mut file, &mut bytes)?;
            if file.name().ends_with(".rms") || file.name().ends_with(".inc") {
                let source = to_chardet_string(bytes);
                files.push(FileData::new(file.name().to_string(), Cow::Owned(source)));
                // If this is an .rms file, move it to the front so main_file() does the right thing
                if file.name().ends_with(".rms") {
                    files.rotate_right(1);
                }
            }
        }

        Ok(Self::new(files))
    }

    /// Create an RMSFile from a string of bytes containing a ZR@ map.
    pub fn from_zip_rms(name: impl AsRef<str>, source: &[u8]) -> io::Result<Self> {
        Self::from_zip_rms_reader(name, io::Cursor::new(source))
    }

    /// Create an RMSFile from a folder containing files intended for a ZR@ map.
    pub fn from_zip_rms_path_unpacked(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut files = vec![];
        for entry in std::fs::read_dir(path)? {
            let path = entry?.path();
            let name = path.to_string_lossy();
            let bytes = std::fs::read(&path)?;
            let source = to_chardet_string(bytes);
            files.push(FileData::new(name.to_string(), Cow::Owned(source)));
            // If this is an .rms file, move it to the front so main_file() does the right thing
            if name.ends_with(".rms") {
                files.rotate_right(1);
            }
        }

        Ok(Self::new(files))
    }

    /// Create an RMSFile from a file path containing a ZR@ map.
    pub fn from_zip_rms_path(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::from_zip_rms_reader(path.as_ref().to_string_lossy(), File::open(path.as_ref())?)
    }

    // pub fn from_bytes(name: impl AsRef<str>, source: &[u8]) -> io::Result<Self> {}

    /// Get the definitions file for this map.
    pub(crate) fn definitions(&self, compatibility: Compatibility) -> (FileId, &str) {
        match compatibility {
            Compatibility::WololoKingdoms => (self.def_wk, self.source(self.def_wk)),
            Compatibility::HDEdition => (self.def_hd, self.source(self.def_hd)),
            _ => (self.def_aoc, self.source(self.def_aoc)),
        }
    }

    fn source(&self, file: FileId) -> &str {
        self.files[file.to_usize()].source.as_ref()
    }

    /// Get the [`FileId`] of the main script in this map.
    ///
    /// [`FileId`]: TODO
    fn file_id(&self) -> FileId {
        FileId::new(0)
    }

    /// Get the source code of the main script in this map.
    fn main_source(&self) -> &str {
        self.source(FileId::new(0))
    }

    /// Get the codespan FileId for a file with the given name in this map (mostly for ZR@ maps).
    fn find_file_id(&self, name: &str) -> Option<FileId> {
        self.files
            .iter()
            .position(|file| file.name == name)
            .map(|index| FileId::new(index as u32))
    }

    fn find_file_source<'a>(&'a self, name: &str) -> Option<&'a str> {
        self.files.iter().find_map(|file| {
            if file.name == name {
                Some(file.source.as_ref())
            } else {
                None
            }
        })
    }

    fn is_zip_rms(&self) -> bool {
        self.files[0].name.starts_with("ZR@")
    }
}

/// The result of a lint run.
pub struct RMSCheckResult<'source> {
    diagnostics: Vec<Diagnostic>,
    rms: RMSFile<'source>,
}

impl<'source> RMSCheckResult<'source> {
    /// Get the codespan file ID for a given file name.
    pub fn file_id(&self, name: &str) -> Option<FileId> {
        self.rms.find_file_id(name)
    }

    /// Get a file's source code by the file name.
    pub fn source(&self, name: &str) -> Option<&str> {
        self.rms.find_file_source(name)
    }

    pub fn main_source(&self) -> &str {
        self.rms.main_source()
    }

    /// Were there any warnings?
    pub fn has_warnings(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    /// Iterate over the diagnostics.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }
}

impl IntoIterator for RMSCheckResult<'_> {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    /// Iterate over the diagnostics.
    fn into_iter(self) -> Self::IntoIter {
        self.diagnostics.into_iter()
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

        let mut diagnostics = vec![];

        let parser = Parser::new(rms.file_id(), rms.main_source());
        for (atom, parse_warning) in parser {
            diagnostics.extend(checker.write_atom(&atom));
            for w in parse_warning {
                if w.kind == ParseErrorKind::MissingCommandArgs {
                    // Handled by arg-types lint
                    continue;
                }
                diagnostics.push(
                    Diagnostic::parse_error(w.location, format!("{:?}", w.kind)).with_code("parse"),
                );
            }
        }

        RMSCheckResult { rms, diagnostics }
    }
}
