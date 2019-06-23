//! Hacky, yes :(

use jsonrpc_core::{ErrorCode, IoHandler, Params};
use languageserver_types::{
    CodeActionProviderCapability, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
    PublishDiagnosticsParams, ServerCapabilities, TextDocumentItem, TextDocumentSyncCapability,
    TextDocumentSyncKind,
};
use rms_check::{Compatibility, RMSCheck};
use serde_json::{self, json};
use std::io::{self, BufRead, Write};

/// More or less copied from RLS:
/// https://github.com/rust-lang/rls/blob/36def189c0ef802b7ca07878100c856f492532cb/rls/src/server/io.rs
fn read_message<R: BufRead>(from: &mut R) -> io::Result<String> {
    let mut length = None;

    loop {
        let mut line = String::new();
        from.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }

        let parts: Vec<&str> = line.split(": ").collect();
        if parts.len() != 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Header '{}' is malformed", line),
            ));
        }
        let header_name = parts[0].to_lowercase();
        let header_value = parts[1].trim();
        match header_name.as_ref() {
            "content-length" => {
                length = Some(usize::from_str_radix(header_value, 10).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "content-length is not a number")
                })?)
            }
            "content-type" => (),
            _ => (),
        }
    }

    let length = length.unwrap();
    let mut message = vec![0; length];
    from.read_exact(&mut message)?;
    String::from_utf8(message).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn write_message(message: &str) {
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    write!(
        writer,
        "Content-Length: {}\r\n\r\n{}",
        message.len(),
        message
    )
    .unwrap();
    writer.flush().unwrap();
}

fn check_and_publish(doc: TextDocumentItem) {
    let checker = RMSCheck::default()
        .compatibility(Compatibility::Conquerors)
        .add_source(doc.uri.as_str(), &doc.text);

    let mut diagnostics = vec![];
    let result = checker.check();
    for warn in result.iter() {
        let diag = codespan_lsp::make_lsp_diagnostic(
            result.codemap(),
            warn.diagnostic().clone(),
            |_filename| Ok(doc.uri.clone()),
        )
        .unwrap();
        diagnostics.push(diag);
    }

    let params = PublishDiagnosticsParams::new(doc.uri.clone(), diagnostics);
    let message = serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": params,
    }))
    .unwrap();
    write_message(&message);
}

fn main() {
    let mut handler = IoHandler::new();
    handler.add_method("initialize", |params: Params| {
        let _params: InitializeParams = params.parse()?;
        let mut capabilities = ServerCapabilities::default();
        capabilities.code_action_provider = Some(CodeActionProviderCapability::Simple(true));
        capabilities.text_document_sync =
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::Full));
        let result = InitializeResult { capabilities };
        serde_json::to_value(result).map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))
    });
    handler.add_notification("initialized", |_params: Params| {});
    handler.add_notification("textDocument/didOpen", |params: Params| {
        let params: DidOpenTextDocumentParams = params.parse().unwrap();
        let doc = params.text_document;

        check_and_publish(doc);
    });

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    loop {
        let message = read_message(&mut reader).expect("could not read message");

        if let Some(response) = handler.handle_request_sync(&message) {
            write_message(&response);
        }
    }
}
