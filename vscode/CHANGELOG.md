# rms-check-vscode change log

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](http://semver.org/).

## 0.0.4

### Language server
* Fix a crash when handling the `initialized` message.
* Implement `textDocument/definition`, providing "Go To Definition" support for consts.

### Command additions and fixes
* Add `set_gaia_unconvertible` and `set_gaia_civilization`.

### Known issues
* The optional "variance" argument to `circle_radius` is not yet supported.

## 0.0.3
Thanks to Chrazini and Zetnus for the feedback on the initial beta release!

### Command additions and fixes
* Add `enable_balanced_elevation`.
* Add layer attributes `base_layer` and `layer_to_place_on`.
* Add `avoid_cliff_zone`, `avoid_all_actor_areas`.
* Add `create_connect_to_nonplayer_land` and `default_terrain_replacement`.
* Remove extraneous argument type from `place_on_forest_zone`.

### Lint additions and fixes
* Update compatibility lint for DE update 36906.
* Allow using `nomad_resources` on HD Edition.

### Known issues
* The optional "variance" argument to `circle_radius` is not yet supported.

## 0.0.2
* Initial beta release.
