use std::iter::Iterator;
use std::str::CharIndices;
use codespan::{FileMap, ByteIndex, ByteSpan, ByteOffset};

/// Represents a word.
#[derive(Clone, Copy)]
pub struct Word<'a> {
    /// Position of this word in the source code.
    pub span: ByteSpan,
    /// The characters in this word.
    pub value: &'a str,
}

impl<'a> Word<'a> {
    /// Get the position of the first character in this word.
    pub fn start(&self) -> ByteIndex {
        self.span.start()
    }
    /// Get the position of the character just past this word.
    pub fn end(&self) -> ByteIndex {
        self.span.end()
    }
}

/// Iterator over words in a string, with their start and end positions.
pub struct Wordize<'a> {
    pos: ByteIndex,
    file_map: &'a FileMap,
    chars: CharIndices<'a>,
}

impl<'a> Wordize<'a> {
    /// Create an iterator over the `source` string's words.
    pub fn new(file_map: &'a FileMap) -> Self {
        Wordize {
            pos: file_map.span().start(),
            file_map,
            chars: file_map.src().char_indices(),
        }
    }
}

impl<'a> Iterator for Wordize<'a> {
    type Item = Word<'a>;

    /// Get the next word.
    fn next(&mut self) -> Option<Self::Item> {
        let mut start = ByteIndex::none();
        let mut end = ByteIndex::none();
        let mut saw_word = false;
        while let Some((index, c)) = self.chars.next() {
            let offset = ByteOffset(index as i64);
            if !saw_word {
                if !char::is_whitespace(c) {
                    saw_word = true;
                    start = self.file_map.span().start() + offset;
                }
                continue;
            }

            if char::is_whitespace(c) {
                end = self.file_map.span().start() + offset;
                break;
            }
        }

        if saw_word {
            // HACK to detect the final token
            if end == ByteIndex::none() {
                end = self.file_map.span().end();
            }

            let span = ByteSpan::new(start, end);
            let value = self.file_map.src_slice(span).unwrap();
            Some(Word {
                span,
                value,
            })
        } else {
            None
        }
    }
}
