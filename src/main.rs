mod wordize;
mod checker;

use wordize::Wordize;
use checker::Checker;

fn main() {
    let words = Wordize::new(include_str!("../CM_Houseboat_v2.rms"));
    let mut checker = Checker::new();
    words.for_each(|w| checker.write_token(&w));
}
