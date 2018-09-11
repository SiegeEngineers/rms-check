extern crate ansi_term;
#[macro_use]
extern crate lazy_static;

mod tokens;
mod wordize;
mod checker;

use ansi_term::Colour::{Blue, Red, Yellow, Cyan};
use wordize::Wordize;
use checker::{Severity, Checker};

fn main() {
    let source = include_str!("../CM_Houseboat_v2.rms");
    let words = Wordize::new(source);
    let mut checker = Checker::new();
    let warnings = words.filter_map(|w| checker.write_token(&w));

    for warn in warnings {
        let start = warn.start().line() - 1;
        let lines = source.lines()
            .take(warn.end().line() as usize + 2)
            .skip(start as usize)
            .enumerate()
            .map(|(offs, line)| (start + offs as u32, line));

        let message = format!("{} {}", match warn.severity() {
            Severity::Warning => Yellow.bold().paint("WARN"),
            Severity::Error => Red.bold().paint("ERROR"),
        }, warn.message());

        println!("\n{}", message);
        lines.for_each(|(n, line)| {
            println!("{} | {}", n, line);
            if n == start + 1 {
                let cstart = warn.start().column();
                let cend = warn.end().column();
                let mut ptrs = String::new();
                for _ in 0..cstart { ptrs.push(' '); }
                for _ in cstart..cend { ptrs.push('^'); }
                println!("{}", Blue.bold().paint(format!("{}-->{}", n.to_string().replace(|_| true, " "), ptrs)));
            }
        });
        match warn.suggestion() {
            Some(ref new_text) => println!("\n    {} Replace with:\n    {}", Cyan.paint("SUGGESTION"), new_text),
            None => (),
        }
    }
}
