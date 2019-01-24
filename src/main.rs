//! A syntax checker for Age of Empires 2 random map scripts.
//!
//! ```bash
//! rms-check "/path/to/aoc/Random/Everything_Random_v4.3.rms"
//! ```

extern crate ansi_term;
extern crate codespan;
extern crate codespan_reporting;
extern crate multisplice;
#[macro_use] extern crate quicli;
extern crate rms_check;
extern crate termcolor;

mod cli_reporter;

use std::fs::{File, remove_file};
use std::io::Read;
use std::path::PathBuf;
use rms_check::{RMSCheck, AutoFixReplacement};
use quicli::prelude::*;
use cli_reporter::report as cli_report;
use multisplice::Multisplice;

#[derive(Debug, StructOpt)]
struct Cli {
    /// Auto-fix some problems.
    #[structopt(long = "fix")]
    fix: bool,
    /// Auto-fix some problems, but don't actually write.
    #[structopt(long = "fix-dry-run")]
    fix_dry_run: bool,
    /// Run unsafe autofixes. These may break your map!
    #[structopt(long = "fix-unsafe")]
    fix_unsafe: bool,
    /// The file to check.
    file: String,
}

fn cli_check(args: Cli) -> Result<()> {
    let checker = RMSCheck::default()
        .add_file(args.file.into())?;
    let result = checker.check();
    let has_warnings = result.has_warnings();

    cli_report(result);

    if has_warnings {
        bail!("There were warnings");
    }
    Ok(())
}

fn cli_fix(args: Cli, dry: bool) -> Result<()> {
    let mut input_file = File::open(&args.file)?;
    let mut bytes = vec![];
    input_file.read_to_end(&mut bytes)?;
    let source = String::from_utf8_lossy(&bytes);

    let checker = RMSCheck::default()
        .add_file(PathBuf::from(&args.file))?;
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
                    eprintln!("autofix {}:{} → {}:{} to {}", start.0.number(), start.1.number(), end.0.number(), end.1.number(), new_value);
                    let start = result.resolve_offset(suggestion.start()).unwrap();
                    let end = result.resolve_offset(suggestion.end()).unwrap();
                    splicer.splice(start.to_usize(), end.to_usize(), new_value);
                },
                AutoFixReplacement::Unsafe(ref new_value) if args.fix_unsafe => {
                    let start = result.resolve_position(suggestion.start()).unwrap();
                    let end = result.resolve_position(suggestion.end()).unwrap();
                    eprintln!("UNSAFE autofix {}:{} → {}:{} to {}", start.0.number(), start.1.number(), end.0.number(), end.1.number(), new_value);
                    let start = result.resolve_offset(suggestion.start()).unwrap();
                    let end = result.resolve_offset(suggestion.end()).unwrap();
                    splicer.splice(start.to_usize(), end.to_usize(), new_value);
                },
                _ => (),
            }
        }
    }

    if dry {
        let temp = format!("{}.tmp", args.file);
        write_to_file(&temp, &splicer.to_string())?;
        let result = cli_check(Cli { file: temp.clone(), ..args });
        remove_file(&temp)?;
        result
    } else {
        let backup = format!("{}.bak", args.file);
        write_to_file(&backup, &source)?;
        write_to_file(&args.file, &splicer.to_string())?;
        remove_file(&backup)?;
        cli_check(args)
    }
}

main!(|args: Cli| {
    if args.fix {
        return cli_fix(args, false);
    }
    if args.fix_dry_run {
        return cli_fix(args, true);
    }

    cli_check(args)?
});
