extern crate ansi_term;
extern crate rms_check;
#[macro_use] extern crate quicli;

use std::fs::File;
use std::io::Read;
use ansi_term::Colour::{Blue, Red, Yellow, Cyan};
use rms_check::{check, Severity};
use quicli::prelude::*;

#[derive(Debug, StructOpt)]
struct Cli {
    /// The file to check.
    file: String,
}

main!(|args: Cli| {
    let mut file = File::open(args.file)?;
    let mut bytes = vec![];
    file.read_to_end(&mut bytes)?;
    let source = String::from_utf8_lossy(&bytes);
    let warnings = check(&source);

    for warn in warnings {
        let start = warn.start().line();
        let lines = source.lines()
            .take(warn.end().line() as usize + 2)
            .skip(start.saturating_sub(1) as usize)
            .enumerate()
            .map(|(offs, line)| (if start > 0 { 0 } else { 1 } + start + offs as u32, line));

        let message = format!("{} {}", match warn.severity() {
            Severity::Warning => Yellow.bold().paint("WARN"),
            Severity::Error => Red.bold().paint("ERROR"),
        }, warn.message());

        println!("\n{}", message);
        lines.for_each(|(n, line)| {
            println!("{} | {}", n, line);
            if n - 1 == start {
                let cstart = warn.start().column();
                let cend = warn.end().column();
                let mut ptrs = String::new();
                for _ in 0..cstart { ptrs.push(' '); }
                for _ in cstart..cend { ptrs.push('^'); }
                println!("{}", Blue.bold().paint(format!("{}-->{}", n.to_string().replace(|_| true, " "), ptrs)));
            }
        });

        for suggestion in warn.suggestions() {
            println!("\n    {} {}", Cyan.paint("SUGGESTION"), suggestion.message());
            match suggestion.replacement() {
                Some(ref new_text) => println!("    {}", new_text),
                None => (),
            }
        }
    }
});
