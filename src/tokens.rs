use std::collections::HashMap;

/// Argument type.
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
        m.insert("<OBJECT_GENERATION>".into(), TokenType {
            name: "<OBJECT_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });
        m.insert("<CONNECTION_GENERATION>".into(), TokenType {
            name: "<CONNECTION_GENERATION>",
            context: TokenContext::Section,
            arg_types: [None, None, None, None],
        });

        m
    };
}
