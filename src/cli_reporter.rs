use ansi_term::Style;
use ansi_term::Colour::Cyan;
use codespan::CodeMap;
use termcolor::{StandardStream, ColorChoice};
use rms_check::{Severity, AutoFixReplacement, Suggestion, Warning};

fn format_suggestion(suggestion: &Suggestion) -> String {
    let mut string = format!("{}: {}", Cyan.paint("suggestion"), suggestion.message());
    match suggestion.replacement() {
        AutoFixReplacement::Unsafe(_) => {
            string.push_str(&format!("{}", Style::new().bold().paint(" (UNSAFE)")));
        },
        _ => (),
    }
    string
}

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
            warn.diagnostic()).unwrap();

        match warn.severity() {
            Severity::Error => num_errors += 1,
            Severity::Warning => num_warnings += 1,
            _ => (),
        }

        if warn.suggestions().iter().any(|s| s.replacement().is_fixable()) {
            match warn.severity() {
                Severity::Error => fixable_errors += 1,
                Severity::Warning => fixable_warnings += 1,
                _ => (),
            }
        }

        for suggestion in warn.suggestions() {
            println!("{}", format_suggestion(&suggestion));
        }
        println!();
    }

    println!();
    println!("{} errors, {} warnings found.", num_errors, num_warnings);
    if fixable_errors > 0 || fixable_warnings > 0 {
        println!("{} errors, {} warnings fixable using --fix", fixable_errors, fixable_warnings);
    }
}
