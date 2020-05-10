use lazy_static::lazy_static;
use std::collections::HashMap;

/// Argument type.
#[derive(Debug, Clone, Copy)]
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

/// Defines where a token can appear.
#[derive(Debug, Clone, Copy)]
pub enum TokenContext {
    /// A flow control token, can appear just about anywhere.
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
    AnyOf(&'static [TokenContext]),
}

/// A list of token argument types (up to 4).
pub type TokenArgTypes = [Option<ArgType>; 4];
/// Describes some characteristic of a token.
#[derive(Debug, Clone)]
pub struct TokenType {
    /// The token's name, as it appears in RMS source code.
    pub name: &'static str,
    /// The context where the token may appear.
    context: TokenContext,
    /// The argument types for this token.
    arg_types: TokenArgTypes,
}
impl TokenType {
    /// Get the type of the `n`th argument.
    pub const fn arg_type(&self, n: u8) -> &Option<ArgType> {
        &self.arg_types[n as usize]
    }

    /// Get the number of arguments required by this token type.
    pub fn arg_len(&self) -> u8 {
        self.arg_types.iter().position(Option::is_none).unwrap_or(4) as u8
    }

    /// Get the context for this type, describing where it can appear.
    pub const fn context(&self) -> &TokenContext {
        &self.context
    }
}

/// A map holding token types, indexed by their name.
type TokenMap = HashMap<String, TokenType>;

/// Utility for initialising a TokenMap, without having to repeat the token names all the time.
struct TokenMapBuilder(TokenMap);
impl TokenMapBuilder {
    /// Initialise a TokenMap builder.
    fn new() -> Self {
        TokenMapBuilder(TokenMap::new())
    }

    /// Add a new token type to the map.
    fn insert(&mut self, t: TokenType) {
        self.0.insert(t.name.into(), t);
    }

    /// Finish the TokenMap.
    #[allow(clippy::missing_const_for_fn)] // false positive
    fn build(self) -> TokenMap {
        self.0
    }
}

/// Terser syntax for creating token types.
macro_rules! token {
    ( $name:expr, $context:expr ) => {
        TokenType {
            name: $name,
            context: $context,
            arg_types: [None, None, None, None],
        }
    };
    ( $name:expr, $context:expr, [ $arg1:ident ] ) => {
        TokenType {
            name: $name,
            context: $context,
            arg_types: [Some(ArgType::$arg1), None, None, None],
        }
    };
    ( $name:expr, $context:expr, [ $arg1:ident, $arg2:ident ] ) => {
        TokenType {
            name: $name,
            context: $context,
            arg_types: [Some(ArgType::$arg1), Some(ArgType::$arg2), None, None],
        }
    };
    ( $name:expr, $context:expr, [ $arg1:ident, $arg2:ident, $arg3:ident ] ) => {
        TokenType {
            name: $name,
            context: $context,
            arg_types: [
                Some(ArgType::$arg1),
                Some(ArgType::$arg2),
                Some(ArgType::$arg3),
                None,
            ],
        }
    };
    ( $name:expr, $context:expr, [ $arg1:ident, $arg2:ident, $arg3:ident, $arg4:ident ] ) => {
        TokenType {
            name: $name,
            context: $context,
            arg_types: [
                Some(ArgType::$arg1),
                Some(ArgType::$arg2),
                Some(ArgType::$arg3),
                Some(ArgType::$arg4),
            ],
        }
    };
}

lazy_static! {
    /// All known tokens.
    pub static ref TOKENS: HashMap<String, TokenType> = {
        let mut m = TokenMapBuilder::new();
        m.insert(token!("#define", TokenContext::Flow, [Word]));
        m.insert(token!("#undefine", TokenContext::Flow, [Word]));
        m.insert(token!("#const", TokenContext::Flow, [Word, Number]));

        m.insert(token!("if", TokenContext::Flow, [OptionalToken]));
        m.insert(token!("elseif", TokenContext::Flow, [OptionalToken]));
        m.insert(token!("else", TokenContext::Flow));
        m.insert(token!("endif", TokenContext::Flow));

        m.insert(token!("start_random", TokenContext::Flow));
        m.insert(token!("percent_chance", TokenContext::Flow, [Number]));
        m.insert(token!("end_random", TokenContext::Flow));

        m.insert(token!("#include", TokenContext::Flow, [Filename]));
        m.insert(token!("#include_drs", TokenContext::Flow, [Filename, Number]));

        m.insert(token!("<PLAYER_SETUP>", TokenContext::Section));
        m.insert(token!("<LAND_GENERATION>", TokenContext::Section));
        m.insert(token!("<ELEVATION_GENERATION>", TokenContext::Section));
        m.insert(token!("<TERRAIN_GENERATION>", TokenContext::Section));
        m.insert(token!("<CLIFF_GENERATION>", TokenContext::Section));
        m.insert(token!("<OBJECTS_GENERATION>", TokenContext::Section));
        m.insert(token!("<CONNECTION_GENERATION>", TokenContext::Section));

        m.insert(token!("color_correction", TokenContext::TopLevelAttribute(None), [Token]));
        m.insert(token!("ai_info_map_type", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")), [Token, Number, Number, Number]));
        m.insert(token!("random_placement", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>"))));
        m.insert(token!("direct_placement", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>"))));
        m.insert(token!("circle_placement", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>"))));
        m.insert(token!("circle_radius", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")), [Number]));
        m.insert(token!("nomad_resources", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>"))));
        m.insert(token!("grouped_by_team", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>"))));
        m.insert(token!("effect_amount", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")), [Token, Token, Token, Number]));
        m.insert(token!("effect_percent", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")), [Token, Token, Token, Number]));
        m.insert(token!("terrain_state", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")), [Number, Number, Number, Number]));
        m.insert(token!("weather_type", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")), [Number, Number, Number, Number]));
        m.insert(token!("guard_state", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")), [Token, Token, Number, Number]));
        m.insert(token!("enable_waves", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")), [Number]));
        m.insert(token!("terrain_mask", TokenContext::TopLevelAttribute(Some("<PLAYER_SETUP>")), [Number]));

        let land_attribute_context = TokenContext::AnyOf(&[
           TokenContext::Attribute(Some("create_land")),
           TokenContext::Attribute(Some("create_player_lands")),
        ]);

        m.insert(token!("create_land", TokenContext::Command(Some("<LAND_GENERATION>"))));
        m.insert(token!("create_player_lands", TokenContext::Command(Some("<LAND_GENERATION>"))));
        m.insert(token!("land_percent", land_attribute_context, [Number]));
        m.insert(token!("land_position", land_attribute_context, [Number, Number]));
        m.insert(token!("land_id", land_attribute_context, [Number]));
        m.insert(token!("terrain_type", TokenContext::AnyOf(&[
           TokenContext::Attribute(Some("create_land")),
           TokenContext::Attribute(Some("create_player_lands")),
           TokenContext::Attribute(Some("create_terrain")),
        ]), [Token]));
        m.insert(token!("base_size", land_attribute_context, [Number]));
        m.insert(token!("base_elevation", land_attribute_context, [Number]));
        m.insert(token!("left_border", land_attribute_context, [Number]));
        m.insert(token!("right_border", land_attribute_context, [Number]));
        m.insert(token!("top_border", land_attribute_context, [Number]));
        m.insert(token!("bottom_border", land_attribute_context, [Number]));
        m.insert(token!("border_fuzziness", land_attribute_context, [Number]));
        m.insert(token!("zone", land_attribute_context, [Number]));
        m.insert(token!("set_zone_by_team", land_attribute_context));
        m.insert(token!("set_zone_randomly", land_attribute_context));
        m.insert(token!("other_zone_avoidance_distance", land_attribute_context, [Number]));
        m.insert(token!("assign_to_player", TokenContext::Attribute(Some("create_land")), [Number]));
        m.insert(token!("assign_to", TokenContext::Attribute(Some("create_land")), [Token, Number, Number, Number]));

        m.insert(token!("base_terrain", TokenContext::AnyOf(&[
            TokenContext::TopLevelAttribute(Some("<LAND_GENERATION>")),
            TokenContext::Attribute(Some("create_land")),
            TokenContext::Attribute(Some("create_player_lands")),
            TokenContext::Attribute(Some("create_elevation")),
            TokenContext::Attribute(Some("create_terrain")),
            TokenContext::Attribute(Some("create_object")),
        ]), [Token]));

        m.insert(token!("min_number_of_cliffs", TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")), [Number]));
        m.insert(token!("max_number_of_cliffs", TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")), [Number]));
        m.insert(token!("min_length_of_cliff", TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")), [Number]));
        m.insert(token!("max_length_of_cliff", TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")), [Number]));
        m.insert(token!("cliff_curliness", TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")), [Number]));
        m.insert(token!("min_distance_cliffs", TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")), [Number]));
        m.insert(token!("min_terrain_distance", TokenContext::TopLevelAttribute(Some("<CLIFF_GENERATION>")), [Number]));

        m.insert(token!("create_terrain", TokenContext::Command(Some("<TERRAIN_GENERATION>")), [Token]));
        m.insert(token!("percent_of_land", TokenContext::Attribute(Some("create_terrain")), [Number]));
        m.insert(token!("number_of_tiles", TokenContext::AnyOf(&[
             TokenContext::Attribute(Some("create_terrain")),
             TokenContext::Attribute(Some("create_elevation")),
        ]), [Number]));
        m.insert(token!("number_of_clumps", TokenContext::AnyOf(&[
             TokenContext::Attribute(Some("create_terrain")),
             TokenContext::Attribute(Some("create_elevation")),
        ]), [Number]));
        m.insert(token!("set_scale_by_groups", TokenContext::AnyOf(&[
             TokenContext::Attribute(Some("create_terrain")),
             TokenContext::Attribute(Some("create_elevation")),
        ])));
        m.insert(token!("set_scale_by_size", TokenContext::AnyOf(&[
             TokenContext::Attribute(Some("create_terrain")),
             TokenContext::Attribute(Some("create_elevation")),
        ])));
        m.insert(token!("spacing_to_other_terrain_types", TokenContext::Attribute(Some("create_terrain")), [Number]));
        m.insert(token!("height_limits", TokenContext::Attribute(Some("create_terrain")), [Number, Number]));
        m.insert(token!("set_flat_terrain_only", TokenContext::Attribute(Some("create_terrain"))));
        m.insert(token!("set_avoid_player_start_areas", TokenContext::Attribute(Some("create_terrain"))));
        m.insert(token!("clumping_factor", TokenContext::Attribute(Some("create_terrain")), [Number]));
        m.insert(token!("base_layer", TokenContext::Attribute(Some("create_terrain")), [Token]));

        m.insert(token!("create_object", TokenContext::Command(Some("<OBJECTS_GENERATION>")), [Token]));
        let create_object = TokenContext::Attribute(Some("create_object"));
        m.insert(token!("set_scaling_to_map_size", create_object));
        m.insert(token!("set_scaling_to_player_number", create_object));
        m.insert(token!("number_of_groups", create_object, [Number]));
        m.insert(token!("number_of_objects", create_object, [Number]));
        m.insert(token!("group_variance", create_object, [Number]));
        m.insert(token!("group_placement_radius", create_object, [Number]));
        m.insert(token!("set_loose_grouping", create_object));
        m.insert(token!("set_tight_grouping", create_object));
        m.insert(token!("terrain_to_place_on", create_object, [Token]));
        m.insert(token!("layer_to_place_on", create_object, [Token]));
        m.insert(token!("set_gaia_object_only", create_object));
        m.insert(token!("set_place_for_every_player", create_object));
        m.insert(token!("place_on_specific_land_id", create_object, [Number]));
        m.insert(token!("min_distance_to_players", create_object, [Number]));
        m.insert(token!("max_distance_to_players", create_object, [Number]));
        m.insert(token!("max_distance_to_other_zones", create_object, [Number]));
        m.insert(token!("min_distance_group_placement", create_object, [Number]));
        m.insert(token!("temp_min_distance_group_placement", create_object, [Number]));
        m.insert(token!("resource_delta", create_object, [Number]));
        m.insert(token!("avoid_forest_zone", create_object, [Number]));
        m.insert(token!("place_on_forest_zone", create_object));
        m.insert(token!("avoid_cliff_zone", create_object, [Number]));
        m.insert(token!("actor_area", create_object, [Number]));
        m.insert(token!("actor_area_radius", create_object, [Number]));
        m.insert(token!("actor_area_to_place_in", create_object, [Number]));
        m.insert(token!("avoid_actor_area", create_object, [Number]));
        m.insert(token!("avoid_all_actor_areas", create_object));
        m.insert(token!("force_placement", create_object));
        m.insert(token!("find_closest", create_object));
        m.insert(token!("second_object", create_object, [Token]));

        let connect_attribute_context = TokenContext::AnyOf(&[
            TokenContext::Attribute(Some("create_connect_all_players_land")),
            TokenContext::Attribute(Some("create_connect_teams_land")),
            TokenContext::Attribute(Some("create_connect_same_land_zones")),
            TokenContext::Attribute(Some("create_connect_all_lands")),
        ]);

        m.insert(token!("create_connect_all_players_land", TokenContext::Command(Some("<CONNECTION_GENERATION>"))));
        m.insert(token!("create_connect_teams_lands", TokenContext::Command(Some("<CONNECTION_GENERATION>"))));
        m.insert(token!("create_connect_same_land_zones", TokenContext::Command(Some("<CONNECTION_GENERATION>"))));
        m.insert(token!("create_connect_all_lands", TokenContext::Command(Some("<CONNECTION_GENERATION>"))));
        m.insert(token!("create_connect_to_nonplayer_land", TokenContext::Command(Some("<CONNECTION_GENERATION>"))));
        m.insert(token!("replace_terrain", connect_attribute_context, [Token, Token]));
        m.insert(token!("terrain_cost", connect_attribute_context, [Token, Number]));
        m.insert(token!("terrain_size", connect_attribute_context, [Token, Number, Number]));
        m.insert(token!("default_terrain_replacement", connect_attribute_context, [Token]));

        m.insert(token!("create_elevation", TokenContext::Command(Some("<ELEVATION_GENERATION>")), [Number]));
        m.insert(token!("spacing", TokenContext::Attribute(Some("create_elevation")), [Number]));
        m.insert(token!("enable_balanced_elevation", TokenContext::Attribute(Some("enable_balanced_elevation"))));

        m.insert(token!("effect_amount", TokenContext::Command(Some("<PLAYER_SETUP>")), [Token, Token, Token, Number]));
        m.insert(token!("effect_percent", TokenContext::Command(Some("<PLAYER_SETUP>")), [Token, Token, Token, Number]));

        m.build()
    };
}
