use ansi_term::Colour::Cyan;
use ansi_term::Style;
use rms_check::{AutoFixReplacement, RMSCheckResult, Severity, Suggestion};
use termcolor::{ColorChoice, StandardStream};
use codespan_reporting::term::{Config, emit};

fn format_suggestion(suggestion: &Suggestion) -> String {
    let mut string = format!("{}: {}", Cyan.paint("suggestion"), suggestion.message());
    if let AutoFixReplacement::Unsafe(_) = suggestion.replacement() {
        string.push_str(&format!("{}", Style::new().bold().paint(" (UNSAFE)")));
    }
    string
}

pub fn report(result: RMSCheckResult) {
    let mut num_warnings = 0;
    let mut num_errors = 0;
    let mut fixable_warnings = 0;
    let mut fixable_errors = 0;

    let config = Config::default();
    let mut stream = StandardStream::stdout(ColorChoice::Auto);
    for warn in result.iter() {
        emit(&mut stream, &config, result.files().1, warn.diagnostic()).unwrap();

        match warn.severity() {
            Severity::Error => num_errors += 1,
            Severity::Warning => num_warnings += 1,
            _ => (),
        }

        if warn
            .suggestions()
            .iter()
            .any(|s| s.replacement().is_fixable())
        {
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
        println!(
            "{} errors, {} warnings fixable using --fix",
            fixable_errors, fixable_warnings
        );
    }
}
