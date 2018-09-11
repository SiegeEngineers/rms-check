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
    let mut source = String::new();
    file.read_to_string(&mut source)?;
    let warnings = check(&source);

    for warn in warnings {
        let start = warn.start().line().saturating_sub(1);
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
});
