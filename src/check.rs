use crate::cli_reporter::report as cli_report;
use anyhow::{bail, Result};
use multisplice::Multisplice;
use rms_check::{AutoFixReplacement, Compatibility, RMSCheck, RMSFile};
use std::fs::{remove_file, write};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct CheckArgs {
    /// Path to the RMS file.
    pub file: PathBuf,
    /// Compatibility level to use when checking the script.
    pub compatibility: Compatibility,
    /// Do not a actually apply fixes.
    pub dry_run: bool,
    /// Also apply unsafe fixes.
    pub fix_unsafe: bool,
}

pub fn cli_check(args: CheckArgs) -> Result<()> {
    let file = RMSFile::from_path(args.file)?;
    let checker = RMSCheck::default().compatibility(args.compatibility);
    let result = checker.check(file);
    let has_warnings = result.has_warnings();

    cli_report(result);

    if has_warnings {
        bail!("There were warnings");
    }
    Ok(())
}

pub fn cli_fix(args: CheckArgs) -> Result<()> {
    let file = RMSFile::from_path(&args.file)?;

    let checker = RMSCheck::default().compatibility(args.compatibility);
    let result = checker.check(file);

    let mut splicer = Multisplice::new(result.main_source());

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
        let check_result = cli_check(CheckArgs {
            file: temp.clone().into(),
            ..args
        });
        remove_file(&temp)?;
        check_result
    } else {
        let backup = format!("{}.bak", args.file.to_string_lossy());
        write(&backup, result.main_source())?;
        write(&args.file, &splicer.to_string())?;
        remove_file(&backup)?;
        cli_check(args)
    }
}
