use crate::cli_reporter::report as cli_report;
use anyhow::{bail, Result};
use multisplice::Multisplice;
use rms_check::{Compatibility, RMSCheck, RMSFile};
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
}

pub fn cli_check(args: CheckArgs) -> Result<()> {
    let file = RMSFile::from_path(args.file)?;
    let checker = RMSCheck::default().compatibility(args.compatibility);
    let result = checker.check(&file);
    let has_warnings = result.has_warnings();

    cli_report(&file, result);

    if has_warnings {
        bail!("There were warnings");
    }
    Ok(())
}

pub fn cli_fix(args: CheckArgs) -> Result<()> {
    let file = RMSFile::from_path(&args.file)?;

    let checker = RMSCheck::default().compatibility(args.compatibility);
    let result = checker.check(&file);

    let mut splicer = Multisplice::new(file.main_source());

    if !result.has_warnings() {
        // All good!
        return Ok(());
    }

    for diagnostic in result.iter() {
        for fix in diagnostic.fixes() {
            let replacement = match fix.replacement() {
                Some(replacement) => replacement,
                None => continue,
            };

            let location = fix.location();
            let start = file
                .get_location(location.file(), location.start())
                .unwrap();
            let end = file.get_location(location.file(), location.end()).unwrap();
            eprintln!(
                "autofix {}:{} → {}:{} to {}",
                start.0 + 1,
                start.1,
                end.0 + 1,
                end.1,
                replacement
            );
            splicer.splice(
                usize::from(location.start()),
                usize::from(location.end()),
                replacement,
            );
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
        write(&backup, file.main_source())?;
        write(&args.file, &splicer.to_string())?;
        remove_file(&backup)?;
        cli_check(args)
    }
}
