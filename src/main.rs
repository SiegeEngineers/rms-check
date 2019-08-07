//! A syntax checker for Age of Empires 2 random map scripts.
//!
//! ```bash
//! rms-check "/path/to/aoc/Random/Everything_Random_v4.3.rms"
//! ```

mod check;
mod cli_reporter;
mod language_server;

use check::{cli_check, cli_fix, CheckArgs};
use failure::Fallible;
use language_server::cli_server;
use quicli::prelude::*;
use rms_check::Compatibility;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Cli {
    /// Start the language server.
    #[structopt(long = "server")]
    server: bool,

    /// Auto-fix some problems.
    #[structopt(long = "fix")]
    fix: bool,
    /// Auto-fix some problems, but don't actually write.
    #[structopt(long = "dry-run")]
    dry_run: bool,
    /// Run unsafe autofixes. These may break your map!
    #[structopt(long = "fix-unsafe")]
    fix_unsafe: bool,
    /// The file to check.
    file: Option<String>,

    #[structopt(long = "aoc")]
    aoc: bool,
    #[structopt(long = "up14")]
    userpatch14: bool,
    #[structopt(long = "up15")]
    userpatch15: bool,
    #[structopt(long = "hd")]
    hd_edition: bool,
    #[structopt(long = "wk")]
    wololo_kingdoms: bool,
}

impl Cli {
    pub fn compat(&self) -> Compatibility {
        if self.wololo_kingdoms {
            Compatibility::WololoKingdoms
        } else if self.hd_edition {
            Compatibility::HDEdition
        } else if self.userpatch14 {
            Compatibility::UserPatch14
        } else if self.userpatch15 {
            Compatibility::UserPatch15
        } else if self.aoc {
            Compatibility::Conquerors
        } else {
            Compatibility::All
        }
    }
}

fn main() -> Fallible<()> {
    let args = Cli::from_args();

    if args.server {
        cli_server();
        unreachable!();
    }

    if args.fix {
        if args.file.is_none() {
            bail!("must specify a file to fix");
        }

        return cli_fix(CheckArgs {
            compatibility: args.compat(),
            file: args.file.unwrap().into(),
            dry_run: args.dry_run,
            fix_unsafe: args.fix_unsafe,
        });
    }

    if args.file.is_none() {
        bail!("must specify a file to check");
    }

    cli_check(CheckArgs {
        compatibility: args.compat(),
        file: args.file.unwrap().into(),
        dry_run: args.dry_run,
        fix_unsafe: args.fix_unsafe,
    })?;

    Ok(())
}
