use codespan::{ByteIndex, ByteOffset, FileId, Files, Location};
use lsp_types::FoldingRange;
use rms_check::{Atom, Parser};
use std::ops::{Bound, RangeBounds};

/// FoldingRange doesn't derive Default...
fn default_fold() -> FoldingRange {
    FoldingRange {
        start_line: 0,
        end_line: 0,
        start_character: Default::default(),
        end_character: Default::default(),
        kind: Default::default(),
    }
}

#[derive(Debug)]
pub struct FoldingRanges<'a> {
    file_id: FileId,
    files: &'a Files,
    inner: Parser<'a>,
    waiting_folds: Vec<Atom<'a>>,
    queued: Vec<FoldingRange>,
}

impl<'a> FoldingRanges<'a> {
    pub fn new(files: &'a Files, file_id: FileId) -> Self {
        Self {
            file_id,
            files,
            inner: Parser::new(file_id, files.source(file_id)),
            waiting_folds: vec![],
            queued: vec![],
        }
    }

    fn line(&self, index: ByteIndex) -> u64 {
        self.files
            .location(self.file_id, index)
            .unwrap()
            .line
            .to_usize() as u64
    }

    fn push(&mut self, range: FoldingRange) {
        self.queued.push(range);
    }

    fn fold_lines(&mut self, range: impl RangeBounds<ByteIndex>) {
        let start_line = match range.start_bound() {
            Bound::Unbounded => 0u64,
            Bound::Included(index) => self.line(*index),
            Bound::Excluded(index) => self.line(*index) + 1,
        };
        let end_line = match range.end_bound() {
            Bound::Unbounded => 0u64,
            Bound::Included(index) => self.line(*index),
            Bound::Excluded(index) => self.line(*index) - 1,
        };
        if end_line > start_line {
            self.push(FoldingRange {
                start_line,
                end_line,
                ..default_fold()
            });
        }
    }

    fn fold(&mut self, range: impl RangeBounds<ByteIndex>) {
        let (start_line, start_character) = match range.start_bound() {
            Bound::Unbounded => (0u64, 0u64),
            Bound::Included(index) => {
                let Location { line, column } = self.files.location(self.file_id, *index).unwrap();
                (line.to_usize() as u64, column.to_usize() as u64)
            }
            Bound::Excluded(index) => {
                let Location { line, column } = self
                    .files
                    .location(self.file_id, *index + ByteOffset(1))
                    .unwrap();
                (line.to_usize() as u64, column.to_usize() as u64)
            }
        };
        let (end_line, end_character) = match range.end_bound() {
            Bound::Unbounded => (0u64, 0u64),
            Bound::Included(index) => {
                let Location { line, column } = self.files.location(self.file_id, *index).unwrap();
                (line.to_usize() as u64, column.to_usize() as u64)
            }
            Bound::Excluded(index) => self
                .files
                .location(self.file_id, *index - ByteOffset(1))
                .map(|Location { line, column }| (line.to_usize() as u64, column.to_usize() as u64))
                .unwrap_or((0, 0)),
        };
        self.push(FoldingRange {
            start_line,
            end_line,
            start_character: Some(start_character),
            end_character: Some(end_character),
            kind: Default::default(),
        });
    }
}

impl Iterator for FoldingRanges<'_> {
    type Item = FoldingRange;
    fn next(&mut self) -> Option<Self::Item> {
        if !self.queued.is_empty() {
            return Some(self.queued.remove(0));
        }

        use Atom::*;
        let atom = match self.inner.next() {
            Some((atom, _)) => atom,
            _ => return None,
        };
        match atom {
            Comment(start, _, Some(end)) => {
                self.fold_lines(start.span.start()..=end.span.start());
            }
            OpenBlock(_) => self.waiting_folds.push(atom),
            CloseBlock(end) => {
                if let Some(OpenBlock(start)) = self.waiting_folds.pop() {
                    self.fold(start.span.end()..end.span.start());
                }
            }
            If(_, _) => self.waiting_folds.push(atom),
            ElseIf(end, _) | Else(end) => {
                let start = match self.waiting_folds.pop() {
                    Some(If(start, _)) | Some(ElseIf(start, _)) => start,
                    _ => return self.next(),
                };
                self.fold_lines(start.span.start()..end.span.start());
                self.waiting_folds.push(atom);
            }
            EndIf(end) => match self.waiting_folds.pop() {
                Some(If(start, _)) | Some(ElseIf(start, _)) | Some(Else(start)) => {
                    self.fold_lines(start.span.start()..=end.span.start());
                }
                _ => (),
            },
            StartRandom(_) => self.waiting_folds.push(atom),
            PercentChance(end, _) => {
                if let Some(PercentChance(start, _)) = self.waiting_folds.last() {
                    let start = start.span.start();
                    self.fold_lines(start..end.span.start());
                    self.waiting_folds.pop();
                }
                self.waiting_folds.push(atom);
            }
            EndRandom(end) => {
                if let Some(PercentChance(start, _)) = self.waiting_folds.last() {
                    let start = start.span.start();
                    self.fold_lines(start..end.span.start());
                    self.waiting_folds.pop();
                }
                if let Some(StartRandom(start)) = self.waiting_folds.last() {
                    let start = start.span.start();
                    self.fold_lines(start..=end.span.start());
                    self.waiting_folds.pop();
                }
            }
            _ => (),
        }
        self.next()
    }
}
