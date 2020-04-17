# rms-check For VS Code
rms-check is a syntax checker and linter for Age of Empires 2 random map scripts.

> Beta: this extension is an early release and can sometimes get in your way. Please read the below sections.

![Linting errors screenshot][screenshot]

[screenshot]: https://raw.githubusercontent.com/goto-bus-stop/rms-check/default/vscode/screenshot.png

## Syntax Highlighting
The syntax highlighting was developed by Nikita Litvin (deltaidea) for the AoE2 Random Map Scripting extension, released under the MIT license, available [here](https://github.com/mangudai/vscode).

## Syntax Checking
rms-check will flag potential problems in the map script while you're editing.

Currently, this cannot be disabled, and it can be a bit too strict at times! In the future, it will be possible to enable/disable individual lints as you like.

By default, the extension will flag compatibility problems, like using the UserPatch 1.4 feature `resource_delta` which does not work in the Conquerors 1.0c. To change the compatibility target, add a comment like this at the top of the file, before executing any RMS commands:
```aoe2-rms
/* Compatibility: WololoKingdoms */
```

For Definitive Edition maps, use:
```aoe2-rms
/* Compatibility: Definitive Edition */
```

If your map supports many different Age of Empires 2 versions, specify the oldest supported version in the `Compatibility` comment and wrap version-specific commands in an `if` statement:
```aoe2-rms
/* Compatibility: Conquerors */
create_object GOLD {
  if UP_EXTENSION
    resource_delta 200
  endif
  if DE_AVAILABLE
    actor_area 123
  endif
}
```

Supported compatibility settings are:
```aoe2-rms
/* For AoC 1.0c: */
/* Compatibility: AoC */
/* Compatibility: Conquerors */

/* For UserPatch 1.4: */
/* Compatibility: UserPatch */
/* Compatibility: UP */
/* Compatibility: UserPatch 1.4 */
/* Compatibility: UP 1.4 */

/* For UserPatch 1.5: */
/* Compatibility: UserPatch 1.5 */
/* Compatibility: UP 1.5 */

/* For HD Edition: */
/* Compatibility: HD */
/* Compatibility: HD Edition */

/* For UP 1.5 + WololoKingdoms: */
/* Compatibility: WK */
/* Compatibility: WololoKingdoms */

/* For the Definitive Edition: */
/* Compatibility: DE */
/* Compatibility: Definitive Edition */
```

## Semantic Folding
Fold command groups, `if`/`else`/`elseif`/`endif` statements, `start_random`/`percent_chance`/`end_random` statements, comments, etc.

## ZR@ (Zip-RMS) Editing (beta)
Right-click a ZR@ map and click "Edit ZR@ (Zip-RMS) map" to open the rms script inside it for editing. When saving the file, it will update the ZR@ file.

This has seen some testing, but not enough. Please make backups every now and then in case a save operation goes awry.

## Formatting (beta)
The extension can format map scripts—it's quite good in some ways, and quite bad in other ways. I don't fully recommend using it yet. It aligns command arguments nicely but removes meaningful whitespace.

## License
rms-check is available under the GPL-3.0 license, [full text](https://github.com/goto-bus-stop/rms-check/blob/default/LICENSE.md).
