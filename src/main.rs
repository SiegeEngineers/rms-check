extern crate ansi_term;
extern crate rms_check;
#[macro_use] extern crate quicli;

use std::fs::File;
use std::io::Read;
use ansi_term::Colour::{Blue, Red, Yellow, Cyan};
use rms_check::{check, Severity, Suggestion, Warning};
use quicli::prelude::*;

#[derive(Debug, StructOpt)]
struct Cli {
    /// The file to check.
    file: String,
}

fn indent(source: &str, indent: &str) -> String {
    source.lines()
        .map(|line| format!("{}{}\n", indent, line))
        .collect::<String>()
}

fn slice_lines<'a>(source: &'a str, warn: &Warning) -> impl Iterator<Item = (u32, &'a str)> {
    let start = warn.start().line();
    source.lines()
        .take(warn.end().line() as usize + 2)
        .skip(start.saturating_sub(1) as usize)
        .enumerate()
        .map(move |(offs, line)| (if start > 0 { 0 } else { 1 } + start + offs as u32, line))
}

fn format_message(warn: &Warning) -> String {
    format!("{} {}", match warn.severity() {
        Severity::Warning => Yellow.bold().paint("WARN"),
        Severity::Error => Red.bold().paint("ERROR"),
    }, warn.message())
}

fn format_suggestion(suggestion: &Suggestion) -> String {
    let mut string = format!("{} {}", Cyan.paint("SUGGESTION"), suggestion.message());
    match suggestion.replacement() {
        Some(ref new_text) => {
            string.push_str("\n");
            string.push_str(new_text);
        },
        None => (),
    }
    string
}

main!(|args: Cli| {
    let mut file = File::open(args.file)?;
    let mut bytes = vec![];
    file.read_to_end(&mut bytes)?;
    let source = String::from_utf8_lossy(&bytes);
    let warnings = check(&source);
    let has_warnings = !warnings.is_empty();

    for warn in warnings {
        let offending_line = warn.start().line();

        println!("\n{}", format_message(&warn));
        for (n, line) in slice_lines(&source, &warn) {
            println!("{} | {}", n, line);
            if n - 1 == offending_line {
                let cstart = warn.start().column();
                let cend = warn.end().column();
                let mut ptrs = String::new();
                // Replace all characters with whitespace, except tabs, for alignment
                for ch in line[0usize..cstart as usize].chars() {
                    ptrs.push(if ch == '\t' { '\t' } else { ' ' });
                }
                for _ in cstart..cend {
                    ptrs.push('^');
                }
                println!("{}", Blue.bold().paint(format!("{}-->{}", n.to_string().replace(|_| true, " "), ptrs)));
            }
        }

        for suggestion in warn.suggestions() {
            println!("\n{}", indent(&format_suggestion(&suggestion), "    "));
        }
    }

    if has_warnings {
        bail!("There were warnings");
    }
});
