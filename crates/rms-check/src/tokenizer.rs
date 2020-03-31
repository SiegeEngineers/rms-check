//! A word splitter for `codespan` files, with location tracking.

use crate::diagnostic::{ByteIndex, FileId, SourceLocation};
use std::iter::Iterator;
use std::str::CharIndices;

/// Represents a word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Word<'a> {
    /// The characters in this word.
    pub value: &'a str,
    /// Source code location for this word.
    pub location: SourceLocation,
}

impl<'a> Word<'a> {
    /// Get the position of the first character in this word.
    pub fn start(&self) -> ByteIndex {
        self.location.start()
    }
    /// Get the position of the character just past this word.
    pub fn end(&self) -> ByteIndex {
        self.location.end()
    }
}

/// Iterator over words in a string, with their start and end positions.
#[derive(Debug)]
pub struct Tokenizer<'a> {
    file: FileId,
    source: &'a str,
    chars: CharIndices<'a>,
}

impl<'a> Tokenizer<'a> {
    /// Create an iterator over the `source` string's words.
    pub fn new(file_id: FileId, source: &'a str) -> Self {
        Tokenizer {
            file: file_id,
            source,
            chars: source.char_indices(),
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Word<'a>;

    /// Get the next word.
    fn next(&mut self) -> Option<Self::Item> {
        let mut start = ByteIndex::from(0);
        let mut end = ByteIndex::from(self.source.len());
        let mut saw_word = false;
        while let Some((index, c)) = self.chars.next() {
            let index = ByteIndex::from(index);
            if !saw_word {
                if !c.is_ascii_whitespace() {
                    saw_word = true;
                    start = index;
                }
                continue;
            }

            if c.is_ascii_whitespace() {
                end = index;
                break;
            }
        }

        if saw_word {
            Some(Word {
                location: SourceLocation::new(self.file, start..end),
                value: &self.source[start.into()..end.into()],
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(source: &str) -> (FileId, &str) {
        (FileId::new(0), source)
    }

    #[test]
    fn split_words() {
        let (file_id, source) = file("simple test words");
        let mut tokenizer = Tokenizer::new(file_id, source);
        let word = tokenizer.next().unwrap();
        assert_eq!(word.value, "simple");
        assert_eq!(word.start(), ByteIndex::from(0));
        assert_eq!(word.end(), ByteIndex::from(6));
        let word = tokenizer.next().unwrap();
        assert_eq!(word.value, "test");
        assert_eq!(word.start(), ByteIndex::from(7));
        assert_eq!(word.end(), ByteIndex::from(11));
        let word = tokenizer.next().unwrap();
        assert_eq!(word.value, "words");
        assert_eq!(word.start(), ByteIndex::from(12));
        assert_eq!(word.end(), ByteIndex::from(17));
    }

    #[test]
    fn split_words_with_chars() {
        let (file_id, source) = file("n/*ot \n \t  a*/comment");
        let mut tokenizer = Tokenizer::new(file_id, source);
        let word = tokenizer.next().unwrap();
        assert_eq!(word.value, "n/*ot");
        assert_eq!(word.start(), ByteIndex::from(0));
        assert_eq!(word.end(), ByteIndex::from(5));
        let word = tokenizer.next().unwrap();
        assert_eq!(word.value, "a*/comment");
        assert_eq!(word.start(), ByteIndex::from(11));
        assert_eq!(word.end(), ByteIndex::from(21));
    }
}
