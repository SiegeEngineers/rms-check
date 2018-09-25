# rms-check

A syntax checker for Age of Empires 2 rando map scripts.

[Usage](#usage) [Goals](#goals) - [Install](#install) - [License: GPL-3.0](#license)

## Usage

```
Detect common problems with AoE Random Map Scripts

USAGE:
    rms-check [FLAGS] <file>

FLAGS:
        --fix            Auto-fix some problems.
        --fix-dry-run    Auto-fix some problems, but don't actually write.
        --fix-unsafe     Run unsafe autofixes. These may break your map!
    -h, --help           Prints help information
    -V, --version        Prints version information

ARGS:
    <file>    The file to check.
```

```bash
rms-check "/path/to/aoc/Random/Everything_Random_v4.3.rms"
```

<!--output start-->
<pre>
<b><span style="color:#A00">ERROR</span></b> <b>Incorrect comment: there must be a space after the opening /*</b>
8401 | 
8402 | /**** BASIC STARTING TOWN ******/
<b><span style="color:#00A">    -->^^^^^</span></b>
8403 | 

    <span style="color:#0AA">SUGGESTION</span> Add a space after the /*
    /* ***


<b><span style="color:#A50">WARN</span></b> <b>Possibly unclosed comment, */ must be preceded by whitespace</b>
8401 | 
8402 | /**** BASIC STARTING TOWN ******/
<b><span style="color:#00A">    -->                          ^^^^^^^</span></b>
8403 | 

    <span style="color:#0AA">SUGGESTION</span> Add a space before the */
    *** */


<b><span style="color:#A50">WARN</span></b> <b>Token `SIEGE_WORSHOP` is never defined</b>
8687 | 	percent_chance 1
8688 | create_object SIEGE_WORSHOP
<b><span style="color:#00A">    -->              ^^^^^^^^^^^^^</span></b>
8689 | {

    <span style="color:#0AA">SUGGESTION</span> Did you mean `SIEGE_WORKSHOP`?<b> (UNSAFE)</b>
    SIEGE_WORKSHOP


<b><span style="color:#A00">ERROR</span></b> <b>Incorrect rnd() call</b>
9002 | {
9003 | 	number_of_objects rnd (1,2)
<b><span style="color:#00A">    -->	                  ^^^^^^^^^</span></b>
9004 |    	number_of_groups rnd(5,20)

    <span style="color:#0AA">SUGGESTION</span> rnd() must not contain spaces
    rnd(1,2)


<b><span style="color:#A00">ERROR</span></b> <b>Incorrect rnd() call</b>
9095 | 	min_distance_to_players rnd(14,19)
9096 | 	max_distance_to_players rnd 25
<b><span style="color:#00A">    -->	                        ^^^^^^</span></b>
9097 | 	min_distance_group_placement rnd(2,7)

    <span style="color:#0AA">SUGGESTION</span> rnd() must not contain spaces


<b><span style="color:#A50">WARN</span></b> <b>Expected a number argument to number_of_groups, but got (0,5)</b>
9151 | 	number_of_objects rnd(3,5)
9152 | 	number_of_groups (0,5)
<b><span style="color:#00A">    -->	                 ^^^^^</span></b>
9153 | 	group_varience 1

    <span style="color:#0AA">SUGGESTION</span> Did you forget the `rnd`?
    rnd(0,5)


<b><span style="color:#A50">WARN</span></b> <b>Token `TRADECART` is never defined</b>
9304 | 	percent_chance 8
9305 | create_object TRADECART
<b><span style="color:#00A">    -->              ^^^^^^^^^</span></b>
9306 | {

    <span style="color:#0AA">SUGGESTION</span> Did you mean `TRADE_CART`?<b> (UNSAFE)</b>
    TRADE_CART


3 errors, 4 warnings found.
2 errors, 2 warnings fixable using --fix
</pre>
<!--output end-->

Using --fix, the problems with "SUGGESTION" lines can be automatically fixed. Lines marked (UNSAFE) will not be autofixed, unless `--fix --fix-unsafe` is used, but be aware that this can break things like constant names.

## Goals

First, the goal is to syntax check and lint RMS files based on a token stream (essentially each whitespace-separated word). That gets us pretty far without needing a proper parser. Parsing RMS properly is difficult because evaluation of branches and parsing are intertwined in the game.

Second, the goal is to provide useful suggestions for fixing any issues that the linter finds.

Later, the goal is to provide a [language server](https://microsoft.github.io/language-server-protocol/) for integration with editors like VS Code and vim. Ideally, fixing many issues would be as simple as pressing a "fix" button when hovering them, or something. Idk what's possible in all editors, but it would be nice!

At that point, this project might grow to also provide autocompletion and other common language server features.

Currently, the focus is on the first two points. rms-check already provides suggestions for many common issues, particularly around comment syntax.

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
