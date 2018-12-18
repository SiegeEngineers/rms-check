//! A syntax checker for Age of Empires 2 random map scripts.
//!
//! ```bash
//! rms-check "/path/to/aoc/Random/Everything_Random_v4.3.rms"
//! ```

extern crate ansi_term;
extern crate multisplice;
extern crate rms_check;
#[macro_use] extern crate quicli;

mod cli_reporter;

use std::fs::{File, remove_file};
use std::io::Read;
use rms_check::{check, Compatibility, AutoFixReplacement};
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
    let mut file = File::open(args.file)?;
    let mut bytes = vec![];
    file.read_to_end(&mut bytes)?;
    let source = String::from_utf8_lossy(&bytes);
    let warnings = check(&source, Compatibility::UserPatch15);
    let has_warnings = !warnings.is_empty();

    cli_report(&source, warnings);

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
    let warnings = check(&source, Compatibility::UserPatch15);
    let mut splicer = Multisplice::new(&source);

    if warnings.is_empty() {
        // All good!
        return Ok(());
    }

    for warn in &warnings {
        // use warnings[i] instead of an iterator so the lifetime of
        // new_value is long enough.
        for suggestion in warn.suggestions() {
            match suggestion.replacement() {
                AutoFixReplacement::Safe(ref new_value) => {
                    eprintln!("autofix {}:{} → {}:{} to {}", suggestion.start().line(), suggestion.start().column(), suggestion.end().line(), suggestion.end().column(), new_value);
                    splicer.splice(suggestion.start().index(), suggestion.end().index(), new_value);
                },
                AutoFixReplacement::Unsafe(ref new_value) if args.fix_unsafe => {
                    eprintln!("UNSAFE autofix {}:{} → {}:{} to {}", suggestion.start().line(), suggestion.start().column(), suggestion.end().line(), suggestion.end().column(), new_value);
                    splicer.splice(suggestion.start().index(), suggestion.end().index(), new_value);
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
