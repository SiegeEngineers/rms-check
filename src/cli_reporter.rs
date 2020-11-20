use ansi_term::Colour::Cyan;
use ansi_term::Style;
use codespan_reporting::diagnostic::{Diagnostic, Label, LabelStyle, Severity};
use codespan_reporting::term::{emit, Config};
use rms_check::{ByteIndex, FileId, RMSCheckResult, RMSFile};
use std::ops::Range;
use termcolor::{ColorChoice, StandardStream};

struct Adapter<'a>(&'a RMSFile<'a>);
impl<'a> codespan_reporting::files::Files<'a> for Adapter<'a> {
    type FileId = FileId;
    type Name = &'a str;
    type Source = &'a str;

    fn name(&'a self, id: Self::FileId) -> Option<Self::Name> {
        Some(self.0.name(id))
    }

    fn source(&'a self, id: Self::FileId) -> Option<Self::Source> {
        Some(self.0.source(id))
    }

    fn line_range(&'a self, id: Self::FileId, line: usize) -> Option<Range<usize>> {
        let start_of_line = self.0.get_byte_index(id, line as u32, 0)?;
        let end_of_line = self
            .0
            .get_byte_index(id, line as u32 + 1, 0)
            .unwrap_or_else(|| ByteIndex::from(self.0.source(id).len()));
        Some(usize::from(start_of_line)..usize::from(end_of_line))
    }

    fn line_index(&'a self, id: Self::FileId, byte_index: usize) -> Option<usize> {
        let (line, _) = self.0.get_location(id, ByteIndex::from(byte_index))?;
        // let start_of_line = self.0.get_byte_index(id, line, 0)?;
        Some(line as usize)
    }
}

/// Print rms-check results to standard output.
pub fn report(file: &RMSFile<'_>, result: RMSCheckResult) {
    let mut num_warnings = 0;
    let mut num_errors = 0;
    let mut fixable_warnings = 0;
    let mut fixable_errors = 0;

    let to_codespan_diagnostic = |diag: &rms_check::Diagnostic| {
        let severity = match diag.severity() {
            rms_check::Severity::ParseError => Severity::Bug,
            rms_check::Severity::Error => Severity::Error,
            rms_check::Severity::Warning => Severity::Warning,
            rms_check::Severity::Hint => Severity::Note,
        };
        let main_label = Label {
            style: LabelStyle::Primary,
            file_id: diag.location().file(),
            range: usize::from(diag.location().start())..usize::from(diag.location().end()),
            message: diag.message().to_string(),
        };
        let labels = std::iter::once(main_label).chain(diag.labels().map(|label| Label {
            style: LabelStyle::Secondary,
            file_id: label.location().file(),
            range: usize::from(label.location().start())..usize::from(label.location().end()),
            message: label.message().to_string(),
        }));

        let diagnostic = Diagnostic::new(severity)
            .with_message(diag.message())
            .with_labels(labels.collect());

        match diag.code() {
            Some(code) => diagnostic.with_code(code),
            None => diagnostic,
        }
    };

    let config = Config::default();
    let mut stream = StandardStream::stdout(ColorChoice::Auto);
    for diagnostic in result {
        emit(
            &mut stream,
            &config,
            &Adapter(file),
            &to_codespan_diagnostic(&diagnostic),
        )
        .unwrap();

        match diagnostic.severity() {
            rms_check::Severity::ParseError | rms_check::Severity::Error => num_errors += 1,
            rms_check::Severity::Warning => num_warnings += 1,
            _ => (),
        }

        if diagnostic.fixes().any(|s| s.replacement().is_some()) {
            match diagnostic.severity() {
                rms_check::Severity::ParseError | rms_check::Severity::Error => fixable_errors += 1,
                rms_check::Severity::Warning => fixable_warnings += 1,
                _ => (),
            }
        }

        for fix in diagnostic.fixes() {
            println!("{}: {}", Cyan.paint("suggestion"), fix.message());
        }
        for suggestion in diagnostic.suggestions() {
            println!(
                "{}: {} {}",
                Cyan.paint("suggestion"),
                suggestion.message(),
                Style::new().bold().paint("(UNSAFE)")
            );
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
