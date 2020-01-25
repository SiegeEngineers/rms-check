use codespan::{ByteIndex, ByteOffset, FileId, Files, Location};
use lsp_types::{FoldingRange, FoldingRangeKind};
use rms_check::{AtomKind, Parser};
use std::ops::{Bound, RangeBounds};

#[derive(Debug)]
pub struct FoldingRanges<'a> {
    file_id: FileId,
    files: &'a Files,
    inner: Parser<'a>,
    waiting_folds: Vec<AtomKind<'a>>,
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

    fn fold_lines(&mut self, range: impl RangeBounds<ByteIndex>, kind: Option<FoldingRangeKind>) {
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
                start_character: Default::default(),
                end_character: Default::default(),
                kind,
            });
        }
    }

    fn fold(&mut self, range: impl RangeBounds<ByteIndex>, kind: Option<FoldingRangeKind>) {
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
            kind,
        });
    }
}

impl Iterator for FoldingRanges<'_> {
    type Item = FoldingRange;
    fn next(&mut self) -> Option<Self::Item> {
        if !self.queued.is_empty() {
            return Some(self.queued.remove(0));
        }

        let atom = match self.inner.next() {
            Some((atom, _)) => atom,
            _ => return None,
        };
        match atom.kind {
            AtomKind::Comment {
                open,
                close: Some(close),
                ..
            } => {
                self.fold_lines(
                    open.span.start()..=close.span.start(),
                    Some(FoldingRangeKind::Comment),
                );
            }
            AtomKind::OpenBlock { .. } => self.waiting_folds.push(atom.kind),
            AtomKind::CloseBlock { head: end } => {
                if let Some(AtomKind::OpenBlock { head: start }) = self.waiting_folds.pop() {
                    self.fold(start.span.end()..end.span.start(), None);
                }
            }
            AtomKind::If { .. } => self.waiting_folds.push(atom.kind),
            AtomKind::ElseIf { head: end, .. } | AtomKind::Else { head: end } => {
                let start = match self.waiting_folds.pop() {
                    Some(AtomKind::If { head, .. }) | Some(AtomKind::ElseIf { head, .. }) => head,
                    _ => return self.next(),
                };
                self.fold_lines(start.span.start()..end.span.start(), None);
                self.waiting_folds.push(atom.kind);
            }
            AtomKind::EndIf { head: end } => match self.waiting_folds.pop() {
                Some(AtomKind::If { head: start, .. })
                | Some(AtomKind::ElseIf { head: start, .. })
                | Some(AtomKind::Else { head: start }) => {
                    self.fold_lines(start.span.start()..=end.span.start(), None);
                }
                _ => (),
            },
            AtomKind::StartRandom { .. } => self.waiting_folds.push(atom.kind),
            AtomKind::PercentChance { head: end, .. } => {
                if let Some(AtomKind::PercentChance { head: start, .. }) = self.waiting_folds.last()
                {
                    let start = start.span.start();
                    self.fold_lines(start..end.span.start(), None);
                    self.waiting_folds.pop();
                }
                self.waiting_folds.push(atom.kind);
            }
            AtomKind::EndRandom { head: end } => {
                if let Some(AtomKind::PercentChance { head: start, .. }) = self.waiting_folds.last()
                {
                    let start = start.span.start();
                    self.fold_lines(start..end.span.start(), None);
                    self.waiting_folds.pop();
                }
                if let Some(AtomKind::StartRandom { head: start }) = self.waiting_folds.last() {
                    let start = start.span.start();
                    self.fold_lines(start..=end.span.start(), None);
                    self.waiting_folds.pop();
                }
            }
            _ => (),
        }
        self.next()
    }
}
