extern crate ansi_term;
extern crate codespan;
extern crate codespan_reporting;
#[macro_use] extern crate lazy_static;
extern crate strsim;

mod tokens;
mod wordize;
mod checker;
mod lints;

use std::io::Result;
use std::sync::Arc;
use std::path::PathBuf;
use checker::Checker;
use codespan::{CodeMap, FileMap, FileName, ByteIndex, LineIndex, ColumnIndex, ByteOffset};
use wordize::Wordize;

pub use wordize::Word;
pub use checker::{
    Compatibility,
    Severity,
    AutoFixReplacement,
    Suggestion,
    Warning,
    Lint,
    ParseState,
    Nesting,
    Expect,
};
pub use tokens::{
    ArgType,
    TokenType,
    TokenContext,
    TOKENS,
};

pub struct RMSCheck {
    #[allow(unused)]
    compatibility: Compatibility,
    codemap: CodeMap,
    filemaps: Vec<Arc<FileMap>>,
}

impl Default for RMSCheck {
    fn default() -> RMSCheck {
        RMSCheck {
            compatibility: Compatibility::Conquerors,
            codemap: CodeMap::new(),
            filemaps: vec![],
        }
    }
}

impl RMSCheck {
    pub fn new() -> Self {
        let check = RMSCheck::default();
        check.add_source(
            "random_map.def",
            include_str!("random_map.def")
        )
    }

    pub fn compatibility(self, compatibility: Compatibility) -> Self {
        Self { compatibility, ..self }
    }

    pub fn add_source(mut self, name: &str, source: &str) -> Self {
        let map = self.codemap.add_filemap(FileName::Virtual(name.to_string().into()), source.to_string());
        self.filemaps.push(map);
        self
    }

    pub fn add_file(mut self, path: PathBuf) -> Result<Self> {
        let map = self.codemap.add_filemap_from_disk(path)?;
        self.filemaps.push(map);
        Ok(self)
    }

    pub fn codemap(&self) -> &CodeMap {
        &self.codemap
    }

    pub fn resolve_position(&self, index: ByteIndex) -> Option<(LineIndex, ColumnIndex)> {
        let file = self.codemap.find_file(index);
        file.and_then(|f| f.location(index).ok())
    }

    pub fn resolve_offset(&self, index: ByteIndex) -> Option<ByteOffset> {
        let file = self.codemap.find_file(index);
        file.and_then(|f| {
            f.location(index).ok().and_then(|(l, c)|
                f.offset(l, c).ok()
            )
        })
    }

    pub fn check(&self) -> Vec<Warning> {
        let words = self.filemaps.iter()
            .map(|map| Wordize::new(&map))
            .flatten();

        let mut checker = Checker::new()
            .compatibility(self.compatibility);
        words.filter_map(|w| checker.write_token(&w)).collect()
    }
}

/// Check a random map script for errors or other issues.
pub fn check(source: &str, compatibility: Compatibility) -> Vec<Warning> {
    let checker = RMSCheck::new()
        .compatibility(compatibility)
        .add_source("source.rms", source);

    checker.check()
}
