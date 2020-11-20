use lsp_types::{FoldingRange, FoldingRangeKind};
use rms_check::{AtomKind, ByteIndex, Parser, RMSFile};
use std::ops::{Bound, RangeBounds};

#[derive(Debug)]
pub struct FoldingRanges<'a> {
    file: &'a RMSFile<'a>,
    parser: Parser<'a>,
    waiting_folds: Vec<AtomKind<'a>>,
    queued: Vec<FoldingRange>,
}

impl<'a> FoldingRanges<'a> {
    pub fn new(file: &'a RMSFile<'a>) -> Self {
        let parser = Parser::new(file.file_id(), file.main_source());
        Self {
            file,
            parser,
            waiting_folds: vec![],
            queued: vec![],
        }
    }

    fn line(&self, index: ByteIndex) -> u32 {
        self.file
            .get_location(self.file.file_id(), index)
            .unwrap()
            .0
    }

    fn push(&mut self, range: FoldingRange) {
        self.queued.push(range);
    }

    fn fold_lines(&mut self, range: impl RangeBounds<ByteIndex>, kind: Option<FoldingRangeKind>) {
        let start_line = match range.start_bound() {
            Bound::Unbounded => 0u32,
            Bound::Included(index) => self.line(*index),
            Bound::Excluded(index) => self.line(*index) + 1,
        };
        let end_line = match range.end_bound() {
            Bound::Unbounded => 0u32,
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
            Bound::Unbounded => (0u32, 0u32),
            Bound::Included(index) => self.file.get_location(self.file.file_id(), *index).unwrap(),
            Bound::Excluded(index) => self
                .file
                .get_location(self.file.file_id(), *index + 1)
                .unwrap(),
        };
        let (end_line, end_character) = match range.end_bound() {
            Bound::Unbounded => (0u32, 0u32),
            Bound::Included(index) => self.file.get_location(self.file.file_id(), *index).unwrap(),
            Bound::Excluded(index) => self
                .file
                .get_location(self.file.file_id(), *index - 1)
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

        let atom = match self.parser.next() {
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
                    open.location.start()..=close.location.start(),
                    Some(FoldingRangeKind::Comment),
                );
            }
            AtomKind::OpenBlock { .. } => self.waiting_folds.push(atom.kind),
            AtomKind::CloseBlock { head: end } => {
                if let Some(AtomKind::OpenBlock { head: start }) = self.waiting_folds.pop() {
                    self.fold(start.location.end()..end.location.start(), None);
                }
            }
            AtomKind::If { .. } => self.waiting_folds.push(atom.kind),
            AtomKind::ElseIf { head: end, .. } | AtomKind::Else { head: end } => {
                let start = match self.waiting_folds.pop() {
                    Some(AtomKind::If { head, .. }) | Some(AtomKind::ElseIf { head, .. }) => head,
                    _ => return self.next(),
                };
                self.fold_lines(start.location.start()..end.location.start(), None);
                self.waiting_folds.push(atom.kind);
            }
            AtomKind::EndIf { head: end } => match self.waiting_folds.pop() {
                Some(AtomKind::If { head: start, .. })
                | Some(AtomKind::ElseIf { head: start, .. })
                | Some(AtomKind::Else { head: start }) => {
                    self.fold_lines(start.location.start()..=end.location.start(), None);
                }
                _ => (),
            },
            AtomKind::StartRandom { .. } => self.waiting_folds.push(atom.kind),
            AtomKind::PercentChance { head: end, .. } => {
                if let Some(AtomKind::PercentChance { head: start, .. }) = self.waiting_folds.last()
                {
                    let start = start.location.start();
                    self.fold_lines(start..end.location.start(), None);
                    self.waiting_folds.pop();
                }
                self.waiting_folds.push(atom.kind);
            }
            AtomKind::EndRandom { head: end } => {
                if let Some(AtomKind::PercentChance { head: start, .. }) = self.waiting_folds.last()
                {
                    let start = start.location.start();
                    self.fold_lines(start..end.location.start(), None);
                    self.waiting_folds.pop();
                }
                if let Some(AtomKind::StartRandom { head: start }) = self.waiting_folds.last() {
                    let start = start.location.start();
                    self.fold_lines(start..=end.location.start(), None);
                    self.waiting_folds.pop();
                }
            }
            _ => (),
        }
        self.next()
    }
}
