use super::SignatureBuilder;
use lazy_static::lazy_static;
use lsp_types::SignatureInformation;
use std::collections::HashMap;

#[derive(Default)]
struct SignatureCollector {
    list: Vec<SignatureBuilder>,
}
impl SignatureCollector {
    fn new(&mut self, name: &'static str) -> &mut SignatureBuilder {
        self.list.push(SignatureBuilder::new(name));
        self.list.last_mut().unwrap()
    }

    fn collect(self) -> HashMap<&'static str, SignatureInformation> {
        self.list
            .into_iter()
            .map(|sig| (sig.name, sig.build()))
            .collect()
    }
}

lazy_static! {
    pub static ref SIGNATURES: HashMap<&'static str, SignatureInformation> = {
        let mut s = SignatureCollector::default();

        s.new("random_placement")
            .description("Players are positioned in a circle/oval around the map.");

        // <LAND_GENERATION>
        s.new("base_terrain")
            .description("Initially, the map is filled with this terrain type.")
            .arg("TerrainType", "The terrain to place.");
        s.new("create_player_lands")
            .description("Creates starting lands for all players.");
        s.new("create_land")
            .description("Creates a generic land.");
        s.new("terrain_type")
            .description("Set the type of terrain to place.")
            .arg("TerrainType", "The type of terrain to place.");
        s.new("land_percent")
            .description("The size of the land, as a percentage of the total map size. For player lands, this is the combined size of all player lands. For generic lands, this is the size of only that land.")
            .arg("Percent", "Percentage of the map to fill with this land.");
        s.new("number_of_tiles")
            .description("The size of the land, in tiles. For player lands, this is the combined size of all player lands. For generic lands, this is the size of only that land.")
            .arg("Tiles", "The number of tiles to fill with this land.");
        s.new("base_size")
            .description("Set the minimum square radius of the land. Default is 3 (7x7 square). Placed sequentially, so if land bases are large and overlap, the ones placed later will be visible. This command can force land size to be bigger than that specified with `land_percent` / `number_of_tiles`. If base_size is high in comparison with land size, the land becomes square-like (or even a perfect square!). Land origins will be placed at least this far from the edge of the map.  If base_size for non-player lands  is too large, the land will fail to find a valid position and will be placed at the center of the map.")
            .arg("Radius", "The minimum square radius of the land.");
        s.new("left_border")
            .arg("Percent", "Percentage to avoid this border by.");
        s.new("top_border")
            .arg("Percent", "Percentage to avoid this border by.");
        s.new("right_border")
            .arg("Percent", "Percentage to avoid this border by.");
        s.new("bottom_border")
            .arg("Percent", "Percentage to avoid this border by.");
        s.new("land_position")
            .arg("X", "X coordinate of the land.")
            .arg("Y", "Y coordinate of the land.");
        s.new("border_fuzziness")
            .arg("Percent", "");
        s.new("clumping_factor")
            .arg("Clumping", "");
        s.new("zone")
            .arg("ZoneIndex", "");
        s.new("set_zone_randomly");
        s.new("set_zone_by_team");
        s.new("other_zone_avoidance_distance")
            .arg("Distance", "");
        s.new("min_placement_distance")
            .arg("Distance", "");
        s.new("assign_to_player")
            .arg("PlayerId", "");

        // <ELEVATION_GENERATION>
        s.new("create_elevation")
            .arg("ElevationLevel", "");
        s.new("set_scale_by_size");
        s.new("set_scale_by_group");
        s.new("spacing")
            .arg("ElevationSpacing", "");

        // <CLIFF_GENERATION>
        s.new("min_number_of_cliffs")
            .arg("Number", "");
        s.new("max_number_of_cliffs")
            .arg("Number", "");
        s.new("min_length_of_cliffs")
            .arg("Length", "");
        s.new("max_length_of_cliffs")
            .arg("Length", "");
        s.new("cliff_curliness")
            .arg("Curliness", "The percent chance of the cliff direction changing at any given tile.");
        s.new("min_distance_cliffs")
            .arg("Distance", "");
        s.new("min_terrain_distance")
            .arg("Distance", "");

        // UserPatch commands
        s.new("ai_info_map_type")
            .description("Set the map type and various characteristics for AIs.")
            .arg("MapName", "The name of the map.")
            .arg("IsNomad", "Set to 1 to indicate a Nomad-style map.")
            .arg("IsMichi", "Set to 1 to indicate a Michi-style map.")
            .arg("IsStandard", "Set to 1 to show the builtin map name from the MapName parameter in the Objectives window, instead of the name of this custom map script.");
        s.new("assign_to")
            .arg("AssignTarget", "The targeting mode. AT_PLAYER to assign to a specific player, AT_COLOR to assign to a colour, or AT_TEAM to assign to a team.")
            .arg("Number", "The player number (1-8), colour number (1-8), or team (1-4) to assign this land to. For AT_TEAM, use 0 for unteamed players, or negate to target any player outside the team.")
            .arg("Mode", "For AT_TEAM, 0 indicates random selection, -1 indicates ordered selection.")
            .arg("Flags", "1: reset players who have been assigned before starting, 2: do not remember assigning this player.");
        s.new("base_elevation")
            .description("Modify the base elevation for player and standard lands.")
            .arg("Elevation", "The elevation level to place this land on. 0 for any elevation.");
        s.new("direct_placement")
            .description("Position players directly using assign_to_player and land_position. If this is used, !P will be appended to the map name in the Objectives window.");
        s.new("effect_amount")
            .description("Apply a research-style effect with an integer value for all players.")
            .arg("Effect", "")
            .arg("ItemName", "")
            .arg("Type", "")
            .arg("Value", "");
        s.new("effect_percent")
            .description("Apply a research-style effect with a percentage for all players. This command is identical to `effect_amount`, except the value is divided by 100 to provide decimal precision.")
            .arg("Effect", "")
            .arg("ItemName", "")
            .arg("Type", "")
            .arg("Percent", "");
        s.new("grouped_by_team")
            .description("Position team members in close proximity on the map. The `base_size` specified in `create_player_lands` determines the distance between players on a team. When enabled, the UP-GROUPED-BY-TEAM #load symbol will be defined for AIs.");
        s.new("guard_state")
            .description("Set the guard state properties for the game. If this is used, !G will be appended to the map name in the Objectives window, along with the guard state details.")
            .arg("TypeId", "TypeId will follow base unit upgrades. If you wish to enable the guard state for villagers, please use VILLAGER_CLASS instead of VILLAGER.")
            .arg("ResourceAmount", "")
            .arg("ResourceDelta", "")
            .arg("GuardFlags", "Add the following flags together to create the value: 1 for guard-flag-victory, 2 for guard-flag-resource, 4 for guard-flag-inverse. For example, to set guard-flag-victory and guard-flag-resource, the GuardFlags value would be 3 (1 + 2). If guard-flag-resource is set in GuardFlags, then ResourceDelta/100 will slowly be added to ResourceAmount as long as TypeId objects remain. If both guard-flag-resource and guard-flag-inverse are set, then the resources will be added only when there are no TypeId objects left. If the guard-flag-victory condition is set, the player will be defeated if no TypeId objects remain.");
        s.new("nomad_resources")
            .description("Modify starting resources to match the built-in nomad map. This means that the cost of a town center (275W, 100S) is added to the stockpile. When enabled, the UP-NOMAD-RESOURCES #load symbol will be defined for AIs.");
        s.new("terrain_state")
            .description("Set various terrain properties for the game.")
            .arg("Mode", "")
            .arg("Param1", "")
            .arg("Param2", "")
            .arg("Value", "");
        s.new("weather_type")
            .description("Change the weather and lighting for a map.")
            .arg("Style", "")
            .arg("LiveColor", "")
            .arg("FogColor", "")
            .arg("WaterDirection", "");

        s.collect()
    };
}
