use crate::cli_reporter::report as cli_report;
use failure::{bail, Fallible};
use multisplice::Multisplice;
use rms_check::{AutoFixReplacement, Compatibility, RMSCheck};
use std::{
    fs::{remove_file, write, File},
    io::Read,
    path::PathBuf,
};

#[derive(Debug, Default)]
pub struct CheckArgs {
    pub file: PathBuf,
    pub compatibility: Compatibility,
    /// Do not a actually apply fixes.
    pub dry_run: bool,
    /// Also apply unsafe fixes.
    pub fix_unsafe: bool,
}

pub fn cli_check(args: CheckArgs) -> Fallible<()> {
    let checker = RMSCheck::default()
        .compatibility(args.compatibility)
        .add_file(args.file)?;
    let result = checker.check();
    let has_warnings = result.has_warnings();

    cli_report(result);

    if has_warnings {
        bail!("There were warnings");
    }
    Ok(())
}

pub fn cli_fix(args: CheckArgs) -> Fallible<()> {
    let mut input_file = File::open(&args.file)?;
    let mut bytes = vec![];
    input_file.read_to_end(&mut bytes)?;
    let source = String::from_utf8_lossy(&bytes);

    let checker = RMSCheck::default()
        .compatibility(args.compatibility)
        .add_file(args.file.clone())?;
    let result = checker.check();

    let mut splicer = Multisplice::new(&source);

    if !result.has_warnings() {
        // All good!
        return Ok(());
    }

    for warn in result.iter() {
        for suggestion in warn.suggestions() {
            match suggestion.replacement() {
                AutoFixReplacement::Safe(ref new_value) => {
                    let start = result
                        .resolve_position(suggestion.file_id(), suggestion.start())
                        .unwrap();
                    let end = result
                        .resolve_position(suggestion.file_id(), suggestion.end())
                        .unwrap();
                    eprintln!(
                        "autofix {}:{} → {}:{} to {}",
                        start.line.number(),
                        start.column.number(),
                        end.line.number(),
                        end.column.number(),
                        new_value
                    );
                    let start = suggestion.start();
                    let end = suggestion.end();
                    splicer.splice(start.to_usize(), end.to_usize(), new_value);
                }
                AutoFixReplacement::Unsafe(ref new_value) if args.fix_unsafe => {
                    let start = result
                        .resolve_position(suggestion.file_id(), suggestion.start())
                        .unwrap();
                    let end = result
                        .resolve_position(suggestion.file_id(), suggestion.end())
                        .unwrap();
                    eprintln!(
                        "UNSAFE autofix {}:{} → {}:{} to {}",
                        start.line.number(),
                        start.column.number(),
                        end.line.number(),
                        end.column.number(),
                        new_value
                    );
                    let start = suggestion.start();
                    let end = suggestion.end();
                    splicer.splice(start.to_usize(), end.to_usize(), new_value);
                }
                _ => (),
            }
        }
    }

    if args.dry_run {
        let temp = format!("{}.tmp", args.file.to_string_lossy());
        write(&temp, &splicer.to_string())?;
        let result = cli_check(CheckArgs {
            file: temp.clone().into(),
            ..args
        });
        remove_file(&temp)?;
        result
    } else {
        let backup = format!("{}.bak", args.file.to_string_lossy());
        write(&backup, source.as_ref())?;
        write(&args.file, &splicer.to_string())?;
        remove_file(&backup)?;
        cli_check(args)
    }
}
