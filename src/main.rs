//! A syntax checker for Age of Empires 2 random map scripts.
//!
//! ```bash
//! rms-check "/path/to/aoc/Random/Everything_Random_v4.3.rms"
//! ```

mod check;
mod cli_reporter;
mod language_server;
mod zip_rms;

use check::{cli_check, cli_fix, CheckArgs};
use failure::{bail, Fallible};
use language_server::cli_server;
use rms_check::Compatibility;
use std::path::PathBuf;
use structopt::StructOpt;
use zip_rms::{cli_pack, cli_unpack};

#[derive(Debug, StructOpt)]
struct CliCheck {
    /// The file to check.
    file: PathBuf,
}

#[derive(Debug, StructOpt)]
enum CliCommand {
    /// Pack a folder into an Zip-RMS map.
    #[structopt(name = "pack")]
    Pack {
        output: PathBuf,
        #[structopt(long, short = "d")]
        indir: PathBuf,
    },
    /// Unpack a Zip-RMS map into a folder.
    #[structopt(name = "unpack")]
    Unpack {
        #[structopt(long, short = "o")]
        outdir: PathBuf,
        input: PathBuf,
    },
    /// Auto-fix problems with a random map script.
    #[structopt(name = "fix")]
    Fix {
        /// Don't write the results.
        #[structopt(long = "dry-run")]
        dry_run: bool,
        /// Run unsafe autofixes. These may break your map!
        #[structopt(long = "unsafe")]
        fix_unsafe: bool,
        /// The file to check.
        file: PathBuf,
    },
    /// Syntax check and lint a random map script.
    #[structopt(name = "check")]
    Check(CliCheck),
    /// Start the language server.
    #[structopt(name = "server")]
    Server,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "rms-check")]
pub struct Cli {
    /// Set the default compatibility to Age of Conquerors. Scripts can override this using
    /// `/* Compatibility: */` comments.
    #[structopt(long = "aoc")]
    aoc: bool,
    /// Set the default compatibility to UserPatch 1.4. Scripts can override this using
    /// `/* Compatibility: */` comments.
    #[structopt(long = "up14")]
    userpatch14: bool,
    /// Set the default compatibility to UserPatch 1.5. Scripts can override this using
    /// `/* Compatibility: */` comments.
    #[structopt(long = "up15")]
    userpatch15: bool,
    /// Set the default compatibility to HD Edition. Scripts can override this using
    /// `/* Compatibility: */` comments.
    #[structopt(long = "hd")]
    hd_edition: bool,
    /// Set the default compatibility to WololoKingdoms. Scripts can override this using
    /// `/* Compatibility: */` comments.
    #[structopt(long = "wk")]
    wololo_kingdoms: bool,

    #[structopt(subcommand)]
    command: Option<CliCommand>,

    /// The file to check, when not using any subcommand.
    file: Option<String>,
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
    let compatibility = args.compat();

    match args.command {
        Some(CliCommand::Unpack { outdir, input }) => cli_unpack(input, outdir),
        Some(CliCommand::Pack { indir, output }) => cli_pack(indir, output),
        Some(CliCommand::Fix {
            dry_run,
            fix_unsafe,
            file,
        }) => cli_fix(CheckArgs {
            compatibility,
            file,
            dry_run,
            fix_unsafe,
        }),
        Some(CliCommand::Server) => {
            cli_server();
            unreachable!();
        }
        Some(CliCommand::Check(args)) => cli_check(CheckArgs {
            compatibility,
            file: args.file,
            ..Default::default()
        }),
        None => {
            let args = CliCheck::from_args();
            cli_check(CheckArgs {
                compatibility,
                file: args.file,
                ..Default::default()
            })
        }
    }
}
