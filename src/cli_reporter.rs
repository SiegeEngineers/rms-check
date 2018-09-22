use ansi_term::Colour::{Blue, Red, Yellow, Cyan};
use rms_check::{Severity, Suggestion, Warning};

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

pub fn report(source: &str, warnings: Vec<Warning>) -> () {
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
}
