extern crate ansi_term;
extern crate either;
#[macro_use] extern crate lazy_static;
extern crate strsim;

mod tokens;
mod wordize;
mod checker;

use wordize::Wordize;
use checker::Checker;
use either::Either;

pub use wordize::{Pos, Range};
pub use checker::{Severity, AutoFixReplacement, Suggestion, Note, Warning};
pub enum Compatibility {
    Conquerors,
    UserPatch15,
}

/// Check a random map script for errors or other issues.
pub fn check(source: &str, compatibility: Compatibility) -> Vec<Warning> {
    let words = Wordize::new(include_str!("random_map.def"));
    let words = match compatibility {
        Compatibility::UserPatch15 => Either::Left(
            words.chain(Wordize::new(include_str!("UserPatchConst.rms")))),
        _ => Either::Right(
            words),
    };
    let words = words.chain(Wordize::new(source));

    let mut checker = Checker::new();
    words.filter_map(|w| checker.write_token(&w)).collect()
}
