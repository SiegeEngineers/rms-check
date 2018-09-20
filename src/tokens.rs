use std::collections::HashMap;

/// Argument type.
#[derive(Clone, Copy)]
pub enum ArgType {
    /// A literal string (no spaces)
    Word = 1,
    /// A number.
    Number = 2,
    /// A token with a value (#const)
    Token = 3,
    /// A possibly-present token (#define)
    OptionalToken = 4,
    /// A file name.
    Filename = 5,
}

#[derive(Clone)]
pub enum TokenContext {
    /// A flow control token.
    Flow,
    /// A <SECTION> token (must be top level)
    Section,
    /// A command with braces at the top level, with an optional <SECTION> restriction.
    Command(Option<&'static str>),
    /// An attribute at the top level, with an optional <SECTION> restriction.
    TopLevelAttribute(Option<&'static str>),
    /// An attribute inside a block, with an optional block type restriction.
    Attribute(Option<&'static str>),
    /// This token can occur in multiple places.
    AnyOf(Vec<TokenContext>),
}

pub type TokenArgTypes = [Option<ArgType>; 4];
pub struct TokenType {
    pub name: &'static str,
    context: TokenContext,
    arg_types: TokenArgTypes,
}
impl TokenType {
    pub fn arg_type(&self, n: u8) -> &Option<ArgType> {
        &self.arg_types[n as usize]
    }
    pub fn arg_len(&self) -> u8 {
        match self.arg_types.iter().position(Option::is_none) {
            Some(index) => index as u8,
            None => 4u8,
        }
    }

    pub fn context(&self) -> &TokenContext {
        &self.context
    }
}

lazy_static! {
    pub static ref TOKENS: HashMap<String, TokenType> = {
        let mut m = HashMap::new();
        m.insert("#define".into(), TokenType {
            name: "#define",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::Word), None, None, None],
        });
        m.insert("#undefine".into(), TokenType {
            name: "#undefine",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::Word), None, None, None],
        });
        m.insert("#const".into(), TokenType {
            name: "#const",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::Word), Some(ArgType::Number), None, None],
        });

        m.insert("if".into(), TokenType {
            name: "if",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::OptionalToken), None, None, None],
        });
        m.insert("elseif".into(), TokenType {
            name: "elseif",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::OptionalToken), None, None, None],
        });
        m.insert("else".into(), TokenType {
            name: "else",
            context: TokenContext::Flow,
            arg_types: [None, None, None, None],
        });
        m.insert("endif".into(), TokenType {
            name: "endif",
            context: TokenContext::Flow,
            arg_types: [None, None, None, None],
        });

        m.insert("start_random".into(), TokenType {
            name: "start_random",
            context: TokenContext::Flow,
            arg_types: [None, None, None, None],
        });
        m.insert("percent_chance".into(), TokenType {
            name: "percent_chance",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("end_random".into(), TokenType {
            name: "end_random",
            context: TokenContext::Flow,
            arg_types: [None, None, None, None],
        });

        m.insert("#include".into(), TokenType {
            name: "#include",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::Filename), None, None, None],
        });
        m.insert("#include_drs".into(), TokenType {
            name: "#include_drs",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::Filename), Some(ArgType::Number), None, None],
        });

        m.insert("<PLAYER_SETUP>".into(), TokenType {
            name: "<PLAYER_SETUP>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert("<LAND_GENERATION>".into(), TokenType {
            name: "<LAND_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert("<ELEVATION_GENERATION>".into(), TokenType {
            name: "<ELEVATION_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert("<TERRAIN_GENERATION>".into(), TokenType {
            name: "<TERRAIN_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert("<CLIFF_GENERATION>".into(), TokenType {
            name: "<CLIFF_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert("<OBJECTS_GENERATION>".into(), TokenType {
            name: "<OBJECTS_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert("<CONNECTION_GENERATION>".into(), TokenType {
            name: "<CONNECTION_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });

        m.insert("random_placement".into(), TokenType {
            name: "random_placement",
            context: TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")),
            arg_types: [None, None, None, None],
        });
        m.insert("grouped_by_team".into(), TokenType {
            name: "grouped_by_team",
            context: TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")),
            arg_types: [None, None, None, None],
        });

        let land_attribute_context = TokenContext::AnyOf(vec![
           TokenContext::Attribute(Some("create_land")),
           TokenContext::Attribute(Some("create_player_lands")),
        ]);

        m.insert("create_land".into(), TokenType {
            name: "create_land",
            context: TokenContext::Command(Some("<LAND_GENERATION>")),
            arg_types: [None, None, None, None],
        });
        m.insert("create_player_lands".into(), TokenType {
            name: "create_player_lands",
            context: TokenContext::Command(Some("<LAND_GENERATION>")),
            arg_types: [None, None, None, None],
        });
        m.insert("land_percent".into(), TokenType {
            name: "land_percent",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("land_position".into(), TokenType {
            name: "land_position",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("land_id".into(), TokenType {
            name: "land_id",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("terrain_type".into(), TokenType {
            name: "terrain_type",
            context: TokenContext::AnyOf(vec![
               TokenContext::Attribute(Some("create_land")),
               TokenContext::Attribute(Some("create_player_lands")),
               TokenContext::Attribute(Some("create_terrain")),
            ]),
            arg_types: [Some(ArgType::Token), None, None, None],
        });
        m.insert("base_size".into(), TokenType {
            name: "base_size",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("left_border".into(), TokenType {
            name: "left_border",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("right_border".into(), TokenType {
            name: "right_border",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("top_border".into(), TokenType {
            name: "top_border",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("bottom_border".into(), TokenType {
            name: "bottom_border",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("border_fuzziness".into(), TokenType {
            name: "border_fuzziness",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("zone".into(), TokenType {
            name: "zone",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("set_zone_by_team".into(), TokenType {
            name: "set_zone_by_team",
            context: land_attribute_context.clone(),
            arg_types: [None, None, None, None],
        });
        m.insert("set_zone_randomly".into(), TokenType {
            name: "set_zone_randomly",
            context: land_attribute_context.clone(),
            arg_types: [None, None, None, None],
        });
        m.insert("other_zone_avoidance_distance".into(), TokenType {
            name: "other_zone_avoidance_distance",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("assign_to_player".into(), TokenType {
            name: "assign_to_player",
            context: TokenContext::Attribute(Some("create_land")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });

        m.insert("base_terrain".into(), TokenType {
            name: "base_terrain",
            context: TokenContext::AnyOf(vec![
                TokenContext::TopLevelAttribute(Some("<LAND_GENERATION>")),
                TokenContext::Attribute(Some("create_land")),
                TokenContext::Attribute(Some("create_player_lands")),
                TokenContext::Attribute(Some("create_elevation")),
                TokenContext::Attribute(Some("create_terrain")),
                TokenContext::Attribute(Some("create_object")),
            ]),
            arg_types: [Some(ArgType::Number), None, None, None],
        });

        m.insert("min_number_of_cliffs".into(), TokenType {
            name: "min_number_of_cliffs",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("max_number_of_cliffs".into(), TokenType {
            name: "max_number_of_cliffs",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("min_length_of_cliff".into(), TokenType {
            name: "min_length_of_cliff",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("max_length_of_cliff".into(), TokenType {
            name: "max_length_of_cliff",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("cliff_curliness".into(), TokenType {
            name: "cliff_curliness",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("min_distance_cliffs".into(), TokenType {
            name: "min_distance_cliffs",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("min_terrain_distance".into(), TokenType {
            name: "min_terrain_distance",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });


        m.insert("create_terrain".into(), TokenType {
            name: "create_terrain",
            context: TokenContext::Command(Some("<TERRAIN_GENERATION>")),
            arg_types: [Some(ArgType::Token), None, None, None],
        });
        m.insert("percent_of_land".into(), TokenType {
            name: "percent_of_land",
            context: TokenContext::Attribute(Some("create_terrain")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("number_of_tiles".into(), TokenType {
            name: "number_of_tiles",
            context: TokenContext::AnyOf(vec![
                 TokenContext::Attribute(Some("create_terrain")),
                 TokenContext::Attribute(Some("create_elevation")),
            ]),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("number_of_clumps".into(), TokenType {
            name: "number_of_clumps",
            context: TokenContext::AnyOf(vec![
                 TokenContext::Attribute(Some("create_terrain")),
                 TokenContext::Attribute(Some("create_elevation")),
            ]),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("set_scale_by_groups".into(), TokenType {
            name: "set_scale_by_groups",
            context: TokenContext::AnyOf(vec![
                 TokenContext::Attribute(Some("create_terrain")),
                 TokenContext::Attribute(Some("create_elevation")),
            ]),
            arg_types: [None, None, None, None],
        });
        m.insert("set_scale_by_size".into(), TokenType {
            name: "set_scale_by_size",
            context: TokenContext::AnyOf(vec![
                 TokenContext::Attribute(Some("create_terrain")),
                 TokenContext::Attribute(Some("create_elevation")),
            ]),
            arg_types: [None, None, None, None],
        });
        m.insert("spacing_to_other_terrain_types".into(), TokenType {
            name: "spacing_to_other_terrain_types",
            context: TokenContext::Attribute(Some("create_terrain")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("height_limits".into(), TokenType {
            name: "height_limits",
            context: TokenContext::Attribute(Some("create_terrain")),
            arg_types: [Some(ArgType::Number), Some(ArgType::Number), None, None],
        });
        m.insert("set_flat_terrain_only".into(), TokenType {
            name: "set_flat_terrain_only",
            context: TokenContext::Attribute(Some("create_terrain")),
            arg_types: [None, None, None, None],
        });
        m.insert("clumping_factor".into(), TokenType {
            name: "clumping_factor",
            context: TokenContext::Attribute(Some("create_terrain")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });

        m.insert("create_object".into(), TokenType {
            name: "create_object",
            context: TokenContext::Command(Some("<OBJECTS_GENERATION>")),
            arg_types: [Some(ArgType::Token), None, None, None],
        });
        m.insert("set_scaling_to_map_size".into(), TokenType {
            name: "set_scaling_to_map_size",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [None, None, None, None],
        });
        m.insert("number_of_groups".into(), TokenType {
            name: "number_of_groups",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("number_of_objects".into(), TokenType {
            name: "number_of_objects",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("group_variance".into(), TokenType {
            name: "group_variance",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("group_placement_radius".into(), TokenType {
            name: "group_placement_radius",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("set_loose_grouping".into(), TokenType {
            name: "set_loose_grouping",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [None, None, None, None],
        });
        m.insert("set_tight_grouping".into(), TokenType {
            name: "set_tight_grouping",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [None, None, None, None],
        });
        m.insert("terrain_to_place_on".into(), TokenType {
            name: "terrain_to_place_on",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Token), None, None, None],
        });
        m.insert("set_gaia_object_only".into(), TokenType {
            name: "set_gaia_object_only",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [None, None, None, None],
        });
        m.insert("set_place_for_every_player".into(), TokenType {
            name: "set_place_for_every_player",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [None, None, None, None],
        });
        m.insert("place_on_specific_land_id".into(), TokenType {
            name: "place_on_specific_land_id",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("min_distance_to_players".into(), TokenType {
            name: "min_distance_to_players",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("max_distance_to_players".into(), TokenType {
            name: "max_distance_to_players",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });

        let connect_attribute_context = TokenContext::AnyOf(vec![
            TokenContext::Attribute(Some("create_connect_all_players_land")),
            TokenContext::Attribute(Some("create_connect_teams_land")),
            TokenContext::Attribute(Some("create_connect_same_land_zones")),
            TokenContext::Attribute(Some("create_connect_all_lands")),
        ]);

        m.insert("create_connect_all_players_land".into(), TokenType {
            name: "create_connect_all_players_land",
            context: TokenContext::Command(Some("<CONNECTION_GENERATION>")),
            arg_types: [None, None, None, None],
        });
        m.insert("create_connect_teams_land".into(), TokenType {
            name: "create_connect_teams_land",
            context: TokenContext::Command(Some("<CONNECTION_GENERATION>")),
            arg_types: [None, None, None, None],
        });
        m.insert("create_connect_same_land_zones".into(), TokenType {
            name: "create_connect_same_land_zones",
            context: TokenContext::Command(Some("<CONNECTION_GENERATION>")),
            arg_types: [None, None, None, None],
        });
        m.insert("create_connect_all_lands".into(), TokenType {
            name: "create_connect_all_lands",
            context: TokenContext::Command(Some("<CONNECTION_GENERATION>")),
            arg_types: [None, None, None, None],
        });
        m.insert("replace_terrain".into(), TokenType {
            name: "replace_terrain",
            context: connect_attribute_context.clone(),
            arg_types: [Some(ArgType::Token), Some(ArgType::Token), None, None],
        });
        m.insert("terrain_cost".into(), TokenType {
            name: "terrain_cost",
            context: connect_attribute_context.clone(),
            arg_types: [Some(ArgType::Token), Some(ArgType::Number), None, None],
        });
        m.insert("terrain_size".into(), TokenType {
            name: "terrain_size",
            context: connect_attribute_context.clone(),
            arg_types: [Some(ArgType::Token), Some(ArgType::Number), Some(ArgType::Number), None],
        });
        m.insert("default_terrain_placement".into(), TokenType {
            name: "default_terrain_placement",
            context: connect_attribute_context.clone(),
            arg_types: [Some(ArgType::Token), None, None, None],
        });

        m.insert("create_elevation".into(), TokenType {
            name: "create_elevation",
            context: TokenContext::Command(Some("<ELEVATION_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert("spacing".into(), TokenType {
            name: "spacing",
            context: TokenContext::Attribute(Some("create_elevation")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });

        m
    };
}
