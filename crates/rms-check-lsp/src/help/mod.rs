mod en;

use codespan::{ByteIndex, FileId, Files};
use lsp_types::{
    Documentation, ParameterInformation, ParameterLabel, SignatureHelp, SignatureInformation,
};
use rms_check::{Atom, Parser};

#[derive(Debug, Clone)]
pub struct Signature {
    pub name: &'static str,
    pub args: &'static [&'static str],
    pub short: &'static str,
    pub long: Option<&'static str>,
}

fn get_signature(command_name: &str) -> Option<&Signature> {
    en::SIGNATURES.get(command_name)
}

fn signature_to_lsp(sig: &Signature) -> SignatureInformation {
    let mut label = sig.name.to_string();

    for arg in sig.args {
        label.push(' ');
        label.push_str(arg);
    }

    SignatureInformation {
        label,
        documentation: Some(Documentation::String(match sig.long {
            Some(long) => format!("{}\n{}", sig.short, long),
            _ => sig.short.to_string(),
        })),
        parameters: sig
            .args
            .iter()
            .map(|name| ParameterInformation {
                label: ParameterLabel::Simple(name.to_string()),
                documentation: None,
            })
            .map(Some)
            .collect(),
    }
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
                    signatures: vec![signature_to_lsp(sig)],
                    active_signature: Some(0),
                    active_parameter,
                });
            }
        }
    }
    None
}
