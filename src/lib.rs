extern crate ansi_term;
#[macro_use]
extern crate lazy_static;

mod tokens;
mod wordize;
mod checker;

use wordize::Wordize;
use checker::Checker;

pub use wordize::Pos;
pub use checker::{Severity, Suggestion, Note, Range, Warning};

pub fn check(source: &str) -> Vec<Warning> {
    let words = Wordize::new(include_str!("random_map.def"))
        .chain(Wordize::new(source));
    let mut checker = Checker::new();
    words.filter_map(|w| checker.write_token(&w)).collect()
}
