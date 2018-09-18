# rms-check

A syntax checker for Age of Empires 2 rando map scripts.

```bash
rms-check "/path/to/aoc/Random/Everything_Random_v4.3.rms"
```

<!--output start-->
<pre>
<b><span style="color:#A00">ERROR</span></b> Incorrect comment: there must be a space after the opening /*
8402 | 
8403 | /**** BASIC STARTING TOWN ******/
<b><span style="color:#00A">    -->^^^^^</span></b>
8404 | 

    <span style="color:#0AA">SUGGESTION</span> Add a space after the /*
    /* ***


<b><span style="color:#A50">WARN</span></b> Possibly unclosed comment, */ must be preceded by whitespace
8402 | 
8403 | /**** BASIC STARTING TOWN ******/
<b><span style="color:#00A">    -->                          ^^^^^^^</span></b>
8404 | 

    <span style="color:#0AA">SUGGESTION</span> Add a space before the */
    *** */


<b><span style="color:#A50">WARN</span></b> Token `SIEGE_WORSHOP` is never defined
8688 | 	percent_chance 1
8689 | create_object SIEGE_WORSHOP
<b><span style="color:#00A">    -->              ^^^^^^^^^^^^^</span></b>
8690 | {

<b><span style="color:#A50">WARN</span></b> Expected a number, but got rnd
9003 | {
9004 | 	number_of_objects rnd (1,2)
<b><span style="color:#00A">    -->                   ^^^</span></b>
9005 |    	number_of_groups rnd(5,20)

<b><span style="color:#A50">WARN</span></b> Expected a number, but got rnd
9096 | 	min_distance_to_players rnd(14,19)
9097 | 	max_distance_to_players rnd 25
<b><span style="color:#00A">    -->                         ^^^</span></b>
9098 | 	min_distance_group_placement rnd(2,7)

<b><span style="color:#A50">WARN</span></b> Expected a number, but got (0,5)
9152 | 	number_of_objects rnd(3,5)
9153 | 	number_of_groups (0,5)
<b><span style="color:#00A">    -->                  ^^^^^</span></b>
9154 | 	group_varience 1

<b><span style="color:#A50">WARN</span></b> Token `TRADECART` is never defined
9305 | 	percent_chance 8
9306 | create_object TRADECART
<b><span style="color:#00A">    -->              ^^^^^^^^^</span></b>
9307 | {
</pre>
<!--output end-->

## Goals

First, the goal is to syntax check and lint RMS files based on a token stream (essentially each whitespace-separated word). That gets us pretty far without needing a proper parser. Parsing RMS properly is difficult because evaluation of branches and parsing are intertwined in the game.

Second, the goal is to provide useful suggestions for fixing any issues that the linter finds.

Later, the goal is to provide a language server of some kind for integration with editors like VS Code and vim. Ideally, fixing many issues would be as simple as pressing a "fix" button when hovering them, or something. Idk what's possible in all editors, but it would be nice!

Currently, the focus is on the first two points. rms-check already provides suggestions for many common issues, particularly around comment syntax.

## License

[GPL-3.0](./LICENSE.md)
