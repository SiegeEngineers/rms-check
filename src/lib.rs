extern crate ansi_term;
#[macro_use]
extern crate lazy_static;

mod tokens;
mod wordize;
mod checker;

use ansi_term::Colour::{Blue, Red, Yellow, Cyan};
use wordize::Wordize;
use checker::Checker;

pub use wordize::Pos;
pub use checker::{Severity, Warning};

pub fn check(source: &str) -> Vec<Warning> {
    let words = Wordize::new(source);
    let mut checker = Checker::new();
    words.filter_map(|w| checker.write_token(&w)).collect()
}
