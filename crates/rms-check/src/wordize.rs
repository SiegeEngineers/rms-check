use codespan::{ByteIndex, FileId, Span};
use std::iter::Iterator;
use std::str::CharIndices;

/// Represents a word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Word<'a> {
    /// The file this word is in.
    pub file: FileId,
    /// Position of this word in the source code.
    pub span: Span,
    /// The characters in this word.
    pub value: &'a str,
}

impl<'a> Word<'a> {
    /// Get the position of the first character in this word.
    #[inline]
    pub fn start(&self) -> ByteIndex {
        self.span.start()
    }
    /// Get the position of the character just past this word.
    #[inline]
    pub fn end(&self) -> ByteIndex {
        self.span.end()
    }
}

/// Iterator over words in a string, with their start and end positions.
#[derive(Debug)]
pub struct Wordize<'a> {
    file: FileId,
    source: &'a str,
    chars: CharIndices<'a>,
}

impl<'a> Wordize<'a> {
    /// Create an iterator over the `source` string's words.
    pub fn new(file_id: FileId, source: &'a str) -> Self {
        Wordize {
            file: file_id,
            source,
            chars: source.char_indices(),
        }
    }
}

impl<'a> Iterator for Wordize<'a> {
    type Item = Word<'a>;

    /// Get the next word.
    fn next(&mut self) -> Option<Self::Item> {
        let mut start = 0u32;
        let mut end = self.source.len() as u32;
        let mut saw_word = false;
        while let Some((index, c)) = self.chars.next() {
            let index = index as u32;
            if !saw_word {
                if !char::is_whitespace(c) {
                    saw_word = true;
                    start = index;
                }
                continue;
            }

            if char::is_whitespace(c) {
                end = index;
                break;
            }
        }

        if saw_word {
            let span = Span::new(start, end);
            let value = &self.source[span.start().to_usize()..span.end().to_usize()];
            Some(Word {
                file: self.file,
                span,
                value,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codespan::{ColumnIndex, Files, LineIndex, Location};

    #[test]
    fn split_words() {
        let mut files = Files::new();
        let file_id = files.add("words.txt", "simple test words");
        let mut wordize = Wordize::new(file_id, files.source(file_id));
        let word = wordize.next().unwrap();
        assert_eq!(word.value, "simple");
        assert_eq!(
            files.location(file_id, word.span.start()).unwrap(),
            Location::new(LineIndex(0), ColumnIndex(0))
        );
        assert_eq!(
            files.location(file_id, word.span.end()).unwrap(),
            Location::new(LineIndex(0), ColumnIndex(6))
        );
        let word = wordize.next().unwrap();
        assert_eq!(word.value, "test");
        assert_eq!(
            files.location(file_id, word.span.start()).unwrap(),
            Location::new(LineIndex(0), ColumnIndex(7))
        );
        assert_eq!(
            files.location(file_id, word.span.end()).unwrap(),
            Location::new(LineIndex(0), ColumnIndex(11))
        );
        let word = wordize.next().unwrap();
        assert_eq!(word.value, "words");
        assert_eq!(
            files.location(file_id, word.span.start()).unwrap(),
            Location::new(LineIndex(0), ColumnIndex(12))
        );
        assert_eq!(
            files.location(file_id, word.span.end()).unwrap(),
            Location::new(LineIndex(0), ColumnIndex(17))
        );
    }

    #[test]
    fn split_words_with_chars() {
        let mut files = Files::new();
        let file_id = files.add("words.txt", "n/*ot \n \t  a*/comment");
        let mut wordize = Wordize::new(file_id, files.source(file_id));
        let word = wordize.next().unwrap();
        assert_eq!(word.value, "n/*ot");
        assert_eq!(
            files.location(file_id, word.span.start()).unwrap(),
            Location::new(LineIndex(0), ColumnIndex(0))
        );
        assert_eq!(
            files.location(file_id, word.span.end()).unwrap(),
            Location::new(LineIndex(0), ColumnIndex(5))
        );
        let word = wordize.next().unwrap();
        assert_eq!(word.value, "a*/comment");
        assert_eq!(
            files.location(file_id, word.span.start()).unwrap(),
            Location::new(LineIndex(1), ColumnIndex(4))
        );
        assert_eq!(
            files.location(file_id, word.span.end()).unwrap(),
            Location::new(LineIndex(1), ColumnIndex(14))
        );
    }
}
