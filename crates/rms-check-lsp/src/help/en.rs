use super::Signature;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    pub static ref SIGNATURES: HashMap<&'static str, Signature> = {
        let mut s = HashMap::new();
        let mut insert = |help: Signature| s.insert(help.name, help);
        // UserPatch commands
        insert(Signature {
            name: "ai_info_map_type",
            args: &["map name", "is nomad", "is michi", "is standard"],
            short: "Set the map type and various characteristics for AIs.",
            long: None,
        });
        insert(Signature {
            name: "assign_to",
            args: &["assign target", "number", "mode", "flags"],
            short: "Set a land to a player number, color, or team.",
            long: Some("This command is intended to assist with direct_placement. You can set Number to 1-8 for AT_PLAYER or AT_COLOR. For AT_TEAM, Number can be set to 0-4 to target the land to any on the specified team (0 is unteamed), negate it to target any outside the team, or -10 for anyone. For AT_TEAM, set Mode to 0 for random selection or -1 for ordered selection. For the Flags parameter, please combine the following: 1: reset players who have already been assigned before starting, 2: do not remember assigning this player."),
        });
        insert(Signature {
            name: "base_elevation",
            args: &["elevation"],
            short: "Modify the base elevation for player and standard lands.",
            long: None,
        });
        insert(Signature {
            name: "direct_placement",
            args: &[],
            short: "Position players directly using assign_to_player and land_position.",
            long: Some("If you set this flag, you can use assign_to_player and land_position inside the create_land command to directly position players on the map. If this is used, !P will be appended to the map name in the Objectives window."),
        });
        insert(Signature {
            name: "effect_amount",
            args: &["effect", "item name", "type", "value"],
            short: "Apply a research-style effect with an integer value for all players",
            long: Some("You may need to use #const to define additional item names. When modifying objects, you may need to target ALL hidden variations, one-by-one, as well. Please consider in-game object upgrades, so that an upgrade will not push a unit's max hitpoints over 32768 or the object will be destroyed. If you disable an object with this command, in-game techs/ages (unless disabled) may re-enable them. The civ tech tree may also override changes. If this is used, !C will be appended to the map name in the Objectives window."),
        });
        insert(Signature {
            name: "effect_percent",
            args: &["effect", "item name", "type", "percent"],
            short: "Apply a research-style effect with a percentage for all players.",
            long: Some("This command is identical to effect_amount, except the value is divided by 100 to provide decimal precision. You may need to use #const to define additional item names. When modifying objects, you may need to target ALL hidden variations, one-by-one, as well. Please consider in-game object upgrades, so that an upgrade will not push a unit's max hitpoints over 32768 or the object will be destroyed. If you disable an object with this command, in-game techs/ages (unless disabled) may re-enable them. The civ tech tree may also override changes. If this is used, !C will be appended to the map name in the Objectives window."),
        });
        insert(Signature {
            name: "grouped_by_team",
            args: &[],
            short: "Position team members in close proximity on the map.",
            long: Some("This command and `random_placement` are mutually exclusive. The `base_size` specified in `create_player_lands` determines the distance between players on a team. When enabled, the UP-GROUPED-BY-TEAM #load symbol will be defined for AIs."),
        });
        insert(Signature {
            name: "guard_state",
            args:  &["type", "resource amount", "resource delta", "guard flags"],
            short: "Set the guard state properties for the game.",
            long: Some("Add the following flags together to create the GuardFlags value: 1 for guard-flag-victory, 2 for guard-flag-resource, 4 for guard-flag-inverse. For example, to set guard-flag-victory and guard-flag-resource, the GuardFlags value would be 3 (1 + 2). If guard-flag-resource is set in GuardFlags, then ResourceDelta/100 will slowly be added to ResourceAmount as long as TypeId objects remain. If both guard-flag-resource and guard-flag-inverse are set, then the resources will be added only when there are no TypeId objects left. If the guard-flag-victory condition is set, the player will be defeated if no TypeId objects remain. TypeId will follow base unit upgrades. If you wish to enable the guard state for villagers, please use VILLAGER_CLASS instead of VILLAGER. If this is used, !G will be appended to the map name in the Objectives window, along with the guard state details."),
        });
        insert(Signature {
            name: "nomad_resources",
            args: &[],
            short: "Modify starting resources to match the built-in nomad map.",
            long: Some("This means that the cost of a town center (275W, 100S) is added to the stockpile. When enabled, the UP-NOMAD-RESOURCES #load symbol will be defined for AIs."),
        });
        insert(Signature {
            name: "terrain_state",
            args: &["mode", "param1", "param2", "value"],
            short: "Set various terrain properties for the game.",
            long: Some("You can enable shallow terrain construction by adding flag 1 to Value with ModeId 0. When enabled, resources like trees, gold, stone, and forage can exist on shallow terrain, as well. Internally, this changes the accessibility of terrain id 4 (shallows) from 0.0 to 1.0 for terrain restrictions 4, 8, 10, and 11. Add flag 2 for thinner shallow/beach blending, which changes the blend priority for shallows (4) to 111. Add flag 4 for alternate ice blending, which changes the blend type for ice (26) to 4."),
        });
        insert(Signature {
            name: "weather_type",
            args: &["style", "live color", "fog color", "water direction"],
            short: "Change the weather and lighting for a map.",
            long: None,
        });
        s
    };
}
