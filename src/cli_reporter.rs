use ansi_term::Style;
use ansi_term::Colour::{Blue, Red, Yellow, Cyan, White};
use codespan::CodeMap;
use termcolor::{StandardStream, ColorChoice, WriteColor};
use rms_check::{Severity, AutoFixReplacement, Suggestion, Note, Warning};

fn indent(source: &str, indent: &str) -> String {
    source.lines()
        .map(|line| format!("{}{}\n", indent, line))
        .collect::<String>()
}

fn format_message(warn: &Warning) -> String {
    format!("{} {}", match warn.severity() {
        Severity::Warning => Yellow.bold().paint("WARN"),
        Severity::Error => Red.bold().paint("ERROR"),
        Severity::Bug => Red.bold().paint("BUG"),
        Severity::Help => White.bold().paint("NOTE"),
        Severity::Note => Blue.bold().paint("NOTE"),
    }, Style::new().bold().paint(warn.message()))
}

fn format_suggestion(suggestion: &Suggestion) -> String {
    let mut string = format!("{} {}", Cyan.paint("SUGGESTION"), suggestion.message());
    match suggestion.replacement() {
        AutoFixReplacement::Safe(ref new_text) => {
            string.push_str("\n");
            string.push_str(new_text);
        },
        AutoFixReplacement::Unsafe(ref new_text) => {
            string.push_str(&format!("{}\n{}", Style::new().bold().paint(" (UNSAFE)"), new_text));
        },
        _ => (),
    }
    string
}

/*
fn format_note(source: &str, note: &Note) -> String {
    let mut string = format!("  {} {}", Style::new().bold().paint("note:"), note.message());

    if let Some(ref range) = note.range() {
        for (n, line) in slice_lines(source, range, 0) {
            string.push_str(&format!("\n  {} | {}", n, line));
        }
    }

    string
}
*/

pub fn report(codemap: &CodeMap, warnings: Vec<Warning>) -> () {
    let mut num_warnings = 0;
    let mut num_errors = 0;
    let mut fixable_warnings = 0;
    let mut fixable_errors = 0;

    let mut stream = StandardStream::stdout(ColorChoice::Auto);
    for warn in warnings {
        codespan_reporting::emit(
            &mut stream,
            codemap,
            warn.diagnostic());

        match warn.severity() {
            Severity::Error => num_errors += 1,
            Severity::Warning => num_warnings += 1,
            _ => (),
        }

        /*
        let offending_line = warn.start().line();

        if warn.suggestions().iter().any(|s| s.replacement().is_fixable()) {
            match warn.severity() {
                Severity::Error => fixable_errors += 1,
                Severity::Warning => fixable_warnings += 1,
                _ => (),
            }
        }

        println!("\n{}", format_message(&warn));
        for (n, line) in slice_lines(&source, warn.range(), 1) {
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
        */
    }

    println!();
    println!("{} errors, {} warnings found.", num_errors, num_warnings);
    if fixable_errors > 0 || fixable_warnings > 0 {
        println!("{} errors, {} warnings fixable using --fix", fixable_errors, fixable_warnings);
    }
}
