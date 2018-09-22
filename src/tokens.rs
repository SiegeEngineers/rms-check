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

type TokenMap = HashMap<String, TokenType>;

struct TokenMapBuilder(TokenMap);
impl TokenMapBuilder {
    fn new() -> Self {
        TokenMapBuilder(TokenMap::new())
    }

    fn insert(&mut self, t: TokenType) -> () {
        self.0.insert(t.name.into(), t);
    }

    fn build(self) -> TokenMap {
        self.0
    }
}

lazy_static! {
    pub static ref TOKENS: HashMap<String, TokenType> = {
        let mut m = TokenMapBuilder::new();
        m.insert(TokenType {
            name: "#define",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::Word), None, None, None],
        });
        m.insert(TokenType {
            name: "#undefine",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::Word), None, None, None],
        });
        m.insert(TokenType {
            name: "#const",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::Word), Some(ArgType::Number), None, None],
        });

        m.insert(TokenType {
            name: "if",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::OptionalToken), None, None, None],
        });
        m.insert(TokenType {
            name: "elseif",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::OptionalToken), None, None, None],
        });
        m.insert(TokenType {
            name: "else",
            context: TokenContext::Flow,
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "endif",
            context: TokenContext::Flow,
            arg_types: [None, None, None, None],
        });

        m.insert(TokenType {
            name: "start_random",
            context: TokenContext::Flow,
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "percent_chance",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "end_random",
            context: TokenContext::Flow,
            arg_types: [None, None, None, None],
        });

        m.insert(TokenType {
            name: "#include",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::Filename), None, None, None],
        });
        m.insert(TokenType {
            name: "#include_drs",
            context: TokenContext::Flow,
            arg_types: [Some(ArgType::Filename), Some(ArgType::Number), None, None],
        });

        m.insert(TokenType {
            name: "<PLAYER_SETUP>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "<LAND_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "<ELEVATION_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "<TERRAIN_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "<CLIFF_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "<OBJECTS_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "<CONNECTION_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });

        m.insert(TokenType {
            name: "random_placement",
            context: TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "grouped_by_team",
            context: TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")),
            arg_types: [None, None, None, None],
        });

        let land_attribute_context = TokenContext::AnyOf(vec![
           TokenContext::Attribute(Some("create_land")),
           TokenContext::Attribute(Some("create_player_lands")),
        ]);

        m.insert(TokenType {
            name: "create_land",
            context: TokenContext::Command(Some("<LAND_GENERATION>")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "create_player_lands",
            context: TokenContext::Command(Some("<LAND_GENERATION>")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "land_percent",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "land_position",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "land_id",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "terrain_type",
            context: TokenContext::AnyOf(vec![
               TokenContext::Attribute(Some("create_land")),
               TokenContext::Attribute(Some("create_player_lands")),
               TokenContext::Attribute(Some("create_terrain")),
            ]),
            arg_types: [Some(ArgType::Token), None, None, None],
        });
        m.insert(TokenType {
            name: "base_size",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "left_border",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "right_border",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "top_border",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "bottom_border",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "border_fuzziness",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "zone",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "set_zone_by_team",
            context: land_attribute_context.clone(),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "set_zone_randomly",
            context: land_attribute_context.clone(),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "other_zone_avoidance_distance",
            context: land_attribute_context.clone(),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "assign_to_player",
            context: TokenContext::Attribute(Some("create_land")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });

        m.insert(TokenType {
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

        m.insert(TokenType {
            name: "min_number_of_cliffs",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "max_number_of_cliffs",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "min_length_of_cliff",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "max_length_of_cliff",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "cliff_curliness",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "min_distance_cliffs",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "min_terrain_distance",
            context: TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });


        m.insert(TokenType {
            name: "create_terrain",
            context: TokenContext::Command(Some("<TERRAIN_GENERATION>")),
            arg_types: [Some(ArgType::Token), None, None, None],
        });
        m.insert(TokenType {
            name: "percent_of_land",
            context: TokenContext::Attribute(Some("create_terrain")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "number_of_tiles",
            context: TokenContext::AnyOf(vec![
                 TokenContext::Attribute(Some("create_terrain")),
                 TokenContext::Attribute(Some("create_elevation")),
            ]),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "number_of_clumps",
            context: TokenContext::AnyOf(vec![
                 TokenContext::Attribute(Some("create_terrain")),
                 TokenContext::Attribute(Some("create_elevation")),
            ]),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "set_scale_by_groups",
            context: TokenContext::AnyOf(vec![
                 TokenContext::Attribute(Some("create_terrain")),
                 TokenContext::Attribute(Some("create_elevation")),
            ]),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "set_scale_by_size",
            context: TokenContext::AnyOf(vec![
                 TokenContext::Attribute(Some("create_terrain")),
                 TokenContext::Attribute(Some("create_elevation")),
            ]),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "spacing_to_other_terrain_types",
            context: TokenContext::Attribute(Some("create_terrain")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "height_limits",
            context: TokenContext::Attribute(Some("create_terrain")),
            arg_types: [Some(ArgType::Number), Some(ArgType::Number), None, None],
        });
        m.insert(TokenType {
            name: "set_flat_terrain_only",
            context: TokenContext::Attribute(Some("create_terrain")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "clumping_factor",
            context: TokenContext::Attribute(Some("create_terrain")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });

        m.insert(TokenType {
            name: "create_object",
            context: TokenContext::Command(Some("<OBJECTS_GENERATION>")),
            arg_types: [Some(ArgType::Token), None, None, None],
        });
        m.insert(TokenType {
            name: "set_scaling_to_map_size",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "number_of_groups",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "number_of_objects",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "group_variance",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "group_placement_radius",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "set_loose_grouping",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "set_tight_grouping",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "terrain_to_place_on",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Token), None, None, None],
        });
        m.insert(TokenType {
            name: "set_gaia_object_only",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "set_place_for_every_player",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "place_on_specific_land_id",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "min_distance_to_players",
            context: TokenContext::Attribute(Some("create_object")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
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

        m.insert(TokenType {
            name: "create_connect_all_players_land",
            context: TokenContext::Command(Some("<CONNECTION_GENERATION>")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "create_connect_teams_land",
            context: TokenContext::Command(Some("<CONNECTION_GENERATION>")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "create_connect_same_land_zones",
            context: TokenContext::Command(Some("<CONNECTION_GENERATION>")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "create_connect_all_lands",
            context: TokenContext::Command(Some("<CONNECTION_GENERATION>")),
            arg_types: [None, None, None, None],
        });
        m.insert(TokenType {
            name: "replace_terrain",
            context: connect_attribute_context.clone(),
            arg_types: [Some(ArgType::Token), Some(ArgType::Token), None, None],
        });
        m.insert(TokenType {
            name: "terrain_cost",
            context: connect_attribute_context.clone(),
            arg_types: [Some(ArgType::Token), Some(ArgType::Number), None, None],
        });
        m.insert(TokenType {
            name: "terrain_size",
            context: connect_attribute_context.clone(),
            arg_types: [Some(ArgType::Token), Some(ArgType::Number), Some(ArgType::Number), None],
        });
        m.insert(TokenType {
            name: "default_terrain_placement",
            context: connect_attribute_context.clone(),
            arg_types: [Some(ArgType::Token), None, None, None],
        });

        m.insert(TokenType {
            name: "create_elevation",
            context: TokenContext::Command(Some("<ELEVATION_GENERATION>")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });
        m.insert(TokenType {
            name: "spacing",
            context: TokenContext::Attribute(Some("create_elevation")),
            arg_types: [Some(ArgType::Number), None, None, None],
        });

        m.build()
    };
}
