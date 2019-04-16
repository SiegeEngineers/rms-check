use std::iter::Iterator;
use std::str::CharIndices;
use codespan::{FileMap, ByteIndex, ByteSpan, ByteOffset};

/// Represents a word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    file_map: &'a FileMap,
    chars: CharIndices<'a>,
}

impl<'a> Wordize<'a> {
    /// Create an iterator over the `source` string's words.
    pub fn new(file_map: &'a FileMap) -> Self {
        Wordize {
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

#[cfg(test)]
mod tests {
    use super::*;
    use codespan::{FileMap, FileName, LineIndex, ColumnIndex};

    #[test]
    fn split_words() {
        let filemap = FileMap::new(FileName::Virtual("words.txt".into()), "simple test words".to_string());
        let mut wordize = Wordize::new(&filemap);
        let word = wordize.next().unwrap();
        assert_eq!(word.value, "simple");
        assert_eq!(
            filemap.location(word.span.start()).unwrap(),
            (LineIndex(0), ColumnIndex(0))
        );
        assert_eq!(
            filemap.location(word.span.end()).unwrap(),
            (LineIndex(0), ColumnIndex(6))
        );
        let word = wordize.next().unwrap();
        assert_eq!(word.value, "test");
        assert_eq!(
            filemap.location(word.span.start()).unwrap(),
            (LineIndex(0), ColumnIndex(7))
        );
        assert_eq!(
            filemap.location(word.span.end()).unwrap(),
            (LineIndex(0), ColumnIndex(11))
        );
        let word = wordize.next().unwrap();
        assert_eq!(word.value, "words");
        assert_eq!(
            filemap.location(word.span.start()).unwrap(),
            (LineIndex(0), ColumnIndex(12))
        );
        assert_eq!(
            filemap.location(word.span.end()).unwrap(),
            (LineIndex(0), ColumnIndex(17))
        );
    }

    #[test]
    fn split_words_with_chars() {
        let filemap = FileMap::new(FileName::Virtual("words.txt".into()), "n/*ot \n \t  a*/comment".to_string());
        let mut wordize = Wordize::new(&filemap);
        let word = wordize.next().unwrap();
        assert_eq!(word.value, "n/*ot");
        assert_eq!(
            filemap.location(word.span.start()).unwrap(),
            (LineIndex(0), ColumnIndex(0))
        );
        assert_eq!(
            filemap.location(word.span.end()).unwrap(),
            (LineIndex(0), ColumnIndex(5))
        );
        let word = wordize.next().unwrap();
        assert_eq!(word.value, "a*/comment");
        assert_eq!(
            filemap.location(word.span.start()).unwrap(),
            (LineIndex(1), ColumnIndex(4))
        );
        assert_eq!(
            filemap.location(word.span.end()).unwrap(),
            (LineIndex(1), ColumnIndex(14))
        );
    }
}
