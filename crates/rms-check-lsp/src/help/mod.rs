mod en;

use codespan::{ByteIndex, FileId, Files};
use rms_check::{ArgType, TOKENS};
use lsp_types::{
    Documentation, ParameterInformation, ParameterLabel, SignatureHelp, SignatureInformation,
};
use rms_check::{Atom, Parser};

#[derive(Debug, Clone)]
struct SignatureBuilder {
    name: &'static str,
    description: Option<String>,
    args: Vec<ParameterInformation>,
}

impl SignatureBuilder {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            description: None,
            args: vec![],
        }
    }

    fn description(&mut self, description: &str) -> &mut Self {
        self.description = Some(description.to_string());
        self
    }

    fn arg(&mut self, name: &str, documentation: &str) -> &mut Self {
        let mut label = name.to_string();
        if let Some(ty) = TOKENS.get(self.name) {
            if let Some(arg) = ty.arg_type(self.args.len() as u8) {
                label += &format!(":{}", match arg {
                    ArgType::Word => "Word",
                    ArgType::Number => "Number",
                    ArgType::Token => "Token",
                    ArgType::OptionalToken => "OptionalToken",
                    ArgType::Filename => "Filename",
                });
            }
        }

        self.args.push(ParameterInformation {
            label: ParameterLabel::Simple(label),
            documentation: Some(Documentation::String(documentation.to_string())),
        });
        self
    }

    #[must_use]
    fn build(self) -> SignatureInformation {
        let mut label = self.name.to_string();
        for arg in &self.args {
            if let ParameterLabel::Simple(arg_name) = &arg.label {
                label += &format!(" {}", arg_name);
            } else {
                unreachable!();
            }
        }
        SignatureInformation {
            label,
            documentation: self.description.map(Documentation::String),
            parameters: Some(self.args),
        }
    }
}

fn get_signature(command_name: &str) -> Option<&SignatureInformation> {
    en::SIGNATURES.get(command_name)
}

pub fn find_signature_help(
    files: &Files,
    file_id: FileId,
    position: ByteIndex,
) -> Option<SignatureHelp> {
    let parser = Parser::new(file_id, files.source(file_id));
    for (atom, _) in parser {
        if let Atom::Command(name, args) = &atom {
            let span = atom.span();
            if span.start() <= position && span.end() >= position {
                let active_parameter = args
                    .iter()
                    .position(|word| word.start() <= position && word.end() >= position)
                    .map(|index| index as i64);
                return get_signature(name.value).map(|sig| SignatureHelp {
                    signatures: vec![sig.clone()],
                    active_signature: Some(0),
                    active_parameter,
                });
            }
        }
    }
    None
}
