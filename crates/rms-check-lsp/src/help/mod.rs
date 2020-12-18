mod en;

use lsp_types::{
    Documentation, ParameterInformation, ParameterLabel, SignatureHelp, SignatureInformation,
};
use rms_check::{ArgType, AtomKind, ByteIndex, Parser, RMSFile, TOKENS};

/// Helper struct to create SignatureInformation structures.
#[derive(Debug, Clone)]
struct SignatureBuilder {
    name: &'static str,
    description: Option<String>,
    args: Vec<ParameterInformation>,
}

impl SignatureBuilder {
    /// Create a new signature builder for the command with the given name.
    fn new(name: &'static str) -> Self {
        Self {
            name,
            description: None,
            args: vec![],
        }
    }

    /// Set the description of this command.
    fn description(&mut self, description: &str) -> &mut Self {
        self.description = Some(description.to_string());
        self
    }

    /// Add an argument to the command.
    fn arg(&mut self, name: &str, documentation: &str) -> &mut Self {
        let mut label = name.to_string();
        if let Some(ty) = TOKENS.get(self.name) {
            if let Some(arg) = ty.arg_type(self.args.len() as u8) {
                label += &format!(
                    ":{}",
                    match arg {
                        ArgType::Word => "Word",
                        ArgType::Number => "Number",
                        ArgType::Token => "Token",
                        ArgType::OptionalToken => "OptionalToken",
                        ArgType::Filename => "Filename",
                    }
                );
            }
        }

        self.args.push(ParameterInformation {
            label: ParameterLabel::Simple(label),
            documentation: Some(Documentation::String(documentation.to_string())),
        });
        self
    }

    /// Consume the builder and create a SignatureInformation instance.
    #[must_use]
    fn build(self) -> SignatureInformation {
        let mut label = self.name.to_string();
        for arg in &self.args {
            if let ParameterLabel::Simple(arg_name) = &arg.label {
                label += &format!(" {}", arg_name);
            } else {
                unreachable!("Expected ParameterLabel::Simple(), got {:?}", arg.label);
            }
        }
        SignatureInformation {
            label,
            documentation: self.description.map(Documentation::String),
            active_parameter: None,
            parameters: Some(self.args),
        }
    }
}

/// Get the language server SignatureInformation for a given command name.
fn get_signature(command_name: &str) -> Option<&SignatureInformation> {
    en::SIGNATURES.get(command_name)
}

pub fn find_signature_help(file: &RMSFile<'_>, position: ByteIndex) -> Option<SignatureHelp> {
    let parser = Parser::new(file.file_id(), file.main_source());
    for (atom, _) in parser {
        let (name, args) = match &atom.kind {
            // Turn args from a Vec<Word> into a Vec<&Word>
            AtomKind::Command { name, arguments } => (name, arguments.iter().collect()),
            AtomKind::Define { head, name }
            | AtomKind::Const {
                head,
                name,
                value: None,
            } => (head, vec![name]),
            AtomKind::Const {
                head,
                name,
                value: Some(value),
            } => (head, vec![name, value]),
            _ => continue,
        };
        let range = atom.location.range();
        if range.contains(&position) {
            let active_parameter = args
                .iter()
                .position(|word| word.location.range().contains(&position))
                .map(|index| index as u32);
            return get_signature(name.value).map(|sig| SignatureHelp {
                signatures: vec![sig.clone()],
                active_signature: Some(0),
                active_parameter,
            });
        }
    }
    None
}
