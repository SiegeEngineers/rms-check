//! Data structures for diagnostics.
use std::fmt::Display;
use std::ops::Range;

///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(u32);

impl FileId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }

    #[allow(unused)]
    pub(crate) const fn to_u32(self) -> u32 {
        self.0
    }

    pub(crate) const fn to_usize(self) -> usize {
        self.0 as usize
    }
}

/// Byte index in a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteIndex(usize);

impl From<usize> for ByteIndex {
    fn from(n: usize) -> Self {
        Self(n)
    }
}

impl From<ByteIndex> for usize {
    fn from(index: ByteIndex) -> Self {
        index.0
    }
}

impl std::ops::Add<isize> for ByteIndex {
    type Output = Self;
    fn add(self, other: isize) -> Self::Output {
        Self(((self.0 as isize) + other) as usize)
    }
}

impl std::ops::Sub<isize> for ByteIndex {
    type Output = Self;
    fn sub(self, other: isize) -> Self::Output {
        Self(((self.0 as isize) - other) as usize)
    }
}

/// Source code location structure, identifying a range of source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    file: FileId,
    start: ByteIndex,
    end: ByteIndex,
}

impl SourceLocation {
    /// Create a source code range.
    pub const fn new(file: FileId, range: Range<ByteIndex>) -> Self {
        Self {
            file,
            start: range.start,
            end: range.end,
        }
    }

    /// Return the file this range refers to.
    pub const fn file(self) -> FileId {
        self.file
    }

    /// Return the byte range this file refers to.
    pub const fn range(self) -> Range<ByteIndex> {
        Range {
            start: self.start,
            end: self.end,
        }
    }

    /// Return the start of the range.
    pub const fn start(self) -> ByteIndex {
        self.start
    }

    /// Return the end of the range.
    pub const fn end(self) -> ByteIndex {
        self.end
    }
}

/// Severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// Something could not be parsed.
    ParseError,
    Error,
    Warning,
    Hint,
}

/// A source code replacement that may fix a problem.
#[derive(Debug, Clone, Hash)]
pub struct Fix {
    message: String,
    location: SourceLocation,
    replacement: Option<String>,
}

impl Fix {
    pub fn new(location: SourceLocation, message: impl Display) -> Self {
        Self {
            location,
            message: message.to_string(),
            replacement: None,
        }
    }

    pub fn replace(self, replacement: impl Display) -> Self {
        Self {
            replacement: Some(replacement.to_string()),
            ..self
        }
    }
}

/// A label describing an underlined region of code associated with a diagnostic.
#[derive(Debug, Clone, Hash)]
pub struct Label {
    location: SourceLocation,
    message: String,
}

impl Label {
    pub fn new(location: SourceLocation, message: impl Display) -> Self {
        Self {
            location,
            message: message.to_string(),
        }
    }
}

/// Represents a diagnostic message that can provide information like errors and warnings to the user.
#[derive(Debug, Clone, Hash)]
pub struct Diagnostic {
    severity: Severity,
    label: Label,
    code: Option<String>,
    fixes: Vec<Fix>,
    suggestions: Vec<Fix>,
    labels: Vec<Label>,
}

impl Diagnostic {
    fn new(severity: Severity, location: SourceLocation, message: impl Display) -> Self {
        Self {
            severity,
            label: Label::new(location, message),
            code: None,
            fixes: vec![],
            suggestions: vec![],
            labels: vec![],
        }
    }

    pub fn parse_error(location: SourceLocation, message: impl Display) -> Self {
        Self::new(Severity::ParseError, location, message)
    }

    pub fn error(location: SourceLocation, message: impl Display) -> Self {
        Self::new(Severity::Error, location, message)
    }

    pub fn warning(location: SourceLocation, message: impl Display) -> Self {
        Self::new(Severity::Warning, location, message)
    }

    pub fn with_code(self, code: impl ToString) -> Self {
        Self {
            code: Some(code.to_string()),
            ..self
        }
    }

    pub fn add_labels(mut self, labels: impl IntoIterator<Item = Label>) -> Self {
        self.labels.extend(labels);
        self
    }

    pub fn add_label(self, label: Label) -> Self {
        self.add_labels(std::iter::once(label))
    }

    pub fn autofix(mut self, fix: Fix) -> Self {
        self.fixes.push(fix);
        self
    }

    pub fn suggest(mut self, fix: Fix) -> Self {
        self.suggestions.push(fix);
        self
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn code(&self) -> Option<&str> {
        self.code.as_ref().map(|s| s.as_str())
    }

    pub fn message(&self) -> &str {
        &self.label.message
    }

    pub fn location(&self) -> SourceLocation {
        self.label.location
    }
}
