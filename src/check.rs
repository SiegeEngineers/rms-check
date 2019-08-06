use crate::cli_reporter::report as cli_report;
use multisplice::Multisplice;
use quicli::prelude::*;
use rms_check::{AutoFixReplacement, Compatibility, RMSCheck};
use std::{
    fs::{remove_file, File},
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

pub fn cli_check(args: CheckArgs) -> Result<()> {
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

pub fn cli_fix(args: CheckArgs) -> Result<()> {
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
                    let start = result.resolve_position(suggestion.start()).unwrap();
                    let end = result.resolve_position(suggestion.end()).unwrap();
                    eprintln!(
                        "autofix {}:{} → {}:{} to {}",
                        start.0.number(),
                        start.1.number(),
                        end.0.number(),
                        end.1.number(),
                        new_value
                    );
                    let start = result.resolve_offset(suggestion.start()).unwrap();
                    let end = result.resolve_offset(suggestion.end()).unwrap();
                    splicer.splice(start.to_usize(), end.to_usize(), new_value);
                }
                AutoFixReplacement::Unsafe(ref new_value) if args.fix_unsafe => {
                    let start = result.resolve_position(suggestion.start()).unwrap();
                    let end = result.resolve_position(suggestion.end()).unwrap();
                    eprintln!(
                        "UNSAFE autofix {}:{} → {}:{} to {}",
                        start.0.number(),
                        start.1.number(),
                        end.0.number(),
                        end.1.number(),
                        new_value
                    );
                    let start = result.resolve_offset(suggestion.start()).unwrap();
                    let end = result.resolve_offset(suggestion.end()).unwrap();
                    splicer.splice(start.to_usize(), end.to_usize(), new_value);
                }
                _ => (),
            }
        }
    }

    if args.dry_run {
        let temp = format!("{}.tmp", args.file.to_string_lossy());
        write_to_file(&temp, &splicer.to_string())?;
        let result = cli_check(CheckArgs {
            file: temp.clone().into(),
            ..args
        });
        remove_file(&temp)?;
        result
    } else {
        let backup = format!("{}.bak", args.file.to_string_lossy());
        write_to_file(&backup, &source)?;
        write_to_file(&args.file, &splicer.to_string())?;
        remove_file(&backup)?;
        cli_check(args)
    }
}
