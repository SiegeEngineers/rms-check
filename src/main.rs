extern crate ansi_term;
extern crate rms_check;
#[macro_use] extern crate quicli;

mod cli_reporter;

use std::fs::File;
use std::io::Read;
use rms_check::check;
use quicli::prelude::*;
use cli_reporter::report as cli_report;

#[derive(Debug, StructOpt)]
struct Cli {
    /// The file to check.
    file: String,
}

main!(|args: Cli| {
    let mut file = File::open(args.file)?;
    let mut bytes = vec![];
    file.read_to_end(&mut bytes)?;
    let source = String::from_utf8_lossy(&bytes);
    let warnings = check(&source);
    let has_warnings = !warnings.is_empty();

    cli_report(&source, warnings);

    if has_warnings {
        bail!("There were warnings");
    }
});
