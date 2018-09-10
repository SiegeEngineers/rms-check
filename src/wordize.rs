use std::iter::Iterator;
use std::str::CharIndices;

/// Source code position.
#[derive(Clone, Copy)]
pub struct Pos(usize, u32, u32);
impl Pos {
    /// Get the current character index.
    pub fn index(&self) -> usize { self.0 }
    /// Get the current line.
    pub fn line(&self) -> u32 { self.1 }
    /// Get the current column.
    pub fn column(&self) -> u32 { self.2 }

    /// Advance by a line.
    fn next_line(&mut self) -> () {
        self.1 += 1;
        self.2 = 0;
    }
    /// Advance by a column.
    fn next_column(&mut self) -> () {
        self.2 += 1;
    }
}

/// Iterator over words in a string, with their start and end positions.
pub struct Wordize<'a> {
    pos: Pos,
    source: &'a str,
    chars: CharIndices<'a>,
}

/// Represents a word.
pub struct Word<'a> {
    /// Position of the first character in the source code.
    pub start: Pos,
    /// Position just past the last character.
    pub end: Pos,
    /// The characters in this word.
    pub value: &'a str,
}

impl<'a> Wordize<'a> {
    /// Create an iterator over the `source` string's words.
    pub fn new(source: &'a str) -> Self {
        Wordize {
            pos: Pos(0, 0, 0),
            source,
            chars: source.char_indices(),
        }
    }
}

impl<'a> Iterator for Wordize<'a> {
    type Item = Word<'a>;

    /// Get the next word.
    fn next(&mut self) -> Option<Self::Item> {
        let mut start = Pos(0, 0, 0);
        let mut end = Pos(0, 0, 0);
        let mut saw_word = false;
        while let Some((index, c)) = self.chars.next() {
            if !saw_word {
                if c == '\n' {
                    self.pos.next_line();
                    continue;
                }
                if !char::is_whitespace(c) {
                    saw_word = true;
                    start = Pos(index, self.pos.line(), self.pos.column());
                }
                self.pos.next_column();
                continue;
            }

            if char::is_whitespace(c) {
                end = Pos(index, self.pos.line(), self.pos.column());
                if c == '\n' {
                    self.pos.next_line();
                } else {
                    self.pos.next_column();
                }
                break;
            }

            self.pos.next_column();
        }

        if saw_word {
            // HACK to detect the final token
            if end.index() == 0 {
                end = Pos(self.source.len(), self.pos.line(), self.pos.column());
            }

            let value = &self.source[start.index()..end.index()];
            Some(Word {
                start,
                end,
                value,
            })
        } else {
            None
        }
    }
}
