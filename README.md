# rms-check

A linter and language server for Age of Empires 2 random map scripts.

[Usage](#usage) - [Status](#status) - [Install](#install) - [License: GPL-3.0](#license)

## Usage

```
Detect common problems with AoE Random Map Scripts

USAGE:
    rms-check [FLAGS] [file]

FLAGS:
        --aoc
        --dry-run       Auto-fix some problems, but don't actually write.
        --fix           Auto-fix some problems.
        --fix-unsafe    Run unsafe autofixes. These may break your map!
    -h, --help          Prints help information
        --hd
        --server        Start the language server.
        --up14
        --up15
    -V, --version       Prints version information

ARGS:
    <file>    The file to check.
```

```bash
rms-check "/path/to/aoc/Random/Everything_Random_v4.3.rms"
```

## Status

There is a simple parser and some lints for highlighting common problems. There is also a barebones language server implementation that provides diagnostics and folding ranges.

In the future, I'd like to support more language server-y things, like automatic formatting, completions, and hover help.

## Install

Currently no binaries are provided. Installation must be done by compiling this repository from source.

First, get rustup: https://rustup.rs/

Then do something like:

```bash
# download rms-check:
git clone https://github.com/goto-bus-stop/rms-check.git
cd rms-check
# build it:
cargo build --release
# run it!
./target/release/rms-check FILENAME
```

## License

[GPL-3.0](./LICENSE.md)
