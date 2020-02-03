# rms-check For VS Code
rms-check is a syntax checker and linter for Age of Empires 2 random map scripts.

![Linting errors screenshot][screenshot]

[screenshot]: https://raw.githubusercontent.com/goto-bus-stop/rms-check/default/vscode/screenshot.png

## Syntax Highlighting
The syntax highlighting was developed by Nikita Litvin (deltaidea) for the AoE2 Random Map Scripting extension, released under the MIT license, available [here](https://github.com/mangudai/vscode).

## Syntax Checking
rms-check will flag potential problems in the map script while you're editing.

By default, the extension will flag compatibility problems, like using the UserPatch 1.4 feature `resource_delta` which does not work in the Conquerors 1.0c. To change the compatibility target, add a comment like this at the top of the file, before executing any RMS commands:
```
/* Compatibility: WololoKingdoms */
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
