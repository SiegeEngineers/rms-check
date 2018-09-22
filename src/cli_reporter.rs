use ansi_term::Style;
use ansi_term::Colour::{Blue, Red, Yellow, Cyan};
use rms_check::{Severity, Suggestion, Note, Range, Warning};

fn indent(source: &str, indent: &str) -> String {
    source.lines()
        .map(|line| format!("{}{}\n", indent, line))
        .collect::<String>()
}

fn slice_lines<'a>(source: &'a str, start: u32, end: u32) -> impl Iterator<Item = (u32, &'a str)> {
    source.lines()
        .take(end as usize + 1)
        .skip(start as usize)
        .enumerate()
        .map(move |(offs, line)| (start + offs as u32, line))
}

fn slice_lines_range<'a>(source: &'a str, range: &Range, context: u32) -> impl Iterator<Item = (u32, &'a str)> {
    slice_lines(source, range.0.line().saturating_sub(context), range.1.line() + context)
}

fn format_message(warn: &Warning) -> String {
    format!("{} {}", match warn.severity() {
        Severity::Warning => Yellow.bold().paint("WARN"),
        Severity::Error => Red.bold().paint("ERROR"),
    }, warn.message())
}

fn format_suggestion(suggestion: &Suggestion) -> String {
    let mut string = format!("{} {}", Cyan.paint("SUGGESTION"), suggestion.message());
    if let Some(ref new_text) = suggestion.replacement() {
        string.push_str("\n");
        string.push_str(new_text);
    }
    string
}

fn format_note(source: &str, note: &Note) -> String {
    let mut string = format!("  {} {}", Style::new().bold().paint("note:"), note.message());

    if let Some(ref range) = note.range() {
        for (n, line) in slice_lines_range(source, range, 0) {
            string.push_str(&format!("\n  {} | {}", n, line));
        }
    }

    string
}

pub fn report(source: &str, warnings: Vec<Warning>) -> () {
    for warn in warnings {
        let offending_line = warn.start().line();

        println!("\n{}", format_message(&warn));
        for (n, line) in slice_lines_range(&source, &Range(*warn.start(), *warn.end()), 1) {
            println!("{} | {}", n, line);
            if n == offending_line {
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

        for note in warn.notes() {
            println!("{}", format_note(&source, &note));
        }

        for suggestion in warn.suggestions() {
            println!("\n{}", indent(&format_suggestion(&suggestion), "    "));
        }
    }
}
