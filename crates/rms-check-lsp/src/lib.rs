//! Transport-agnostic language server protocol implementation for Age of Empires 2 random map
//! scripts, using rms-check.

#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(unused)]

use jsonrpc_core::{ErrorCode, IoHandler, Params};
use lsp_types::{
    CodeAction, CodeActionParams, CodeActionProviderCapability, Diagnostic,
    DiagnosticRelatedInformation, DiagnosticSeverity, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentFormattingParams, FoldingRange,
    FoldingRangeParams, FoldingRangeProviderCapability, InitializeParams, InitializeResult,
    Location, MessageType, NumberOrString, Position, PublishDiagnosticsParams, ServerCapabilities,
    ServerInfo, ShowMessageParams, SignatureHelpOptions, TextDocumentItem,
    TextDocumentPositionParams, TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Url,
    WorkDoneProgressOptions, WorkspaceEdit,
};
use multisplice::Multisplice;
use rms_check::{
    ByteIndex, Compatibility, FileId, FormatOptions, RMSCheck, RMSFile, Severity, SourceLocation,
};
use serde_json::{self, json};
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{Arc, Mutex};

mod folds;
mod help;

type RpcResult = jsonrpc_core::Result<serde_json::Value>;

fn internal_error(message: impl Display) -> jsonrpc_core::Error {
    jsonrpc_core::Error {
        code: ErrorCode::InternalError,
        message: message.to_string(),
        data: None,
    }
}

fn unknown_file() -> jsonrpc_core::Error {
    jsonrpc_core::Error::invalid_params("Request referenced an unknown file")
}

fn out_of_range() -> jsonrpc_core::Error {
    internal_error("Range conversion between rms-check and the Language Server Protocol failed. This is a bug.")
}

struct Document {
    version: i64,
    // Can be 'static because we'll only pass in owned data.
    file: RMSFile<'static>,
    diagnostics: Vec<rms_check::Diagnostic>,
}

impl Document {
    fn new(file: RMSFile<'static>, version: i64) -> Self {
        Self {
            version,
            file,
            diagnostics: vec![],
        }
    }

    fn to_lsp_range(&self, location: SourceLocation) -> Option<lsp_types::Range> {
        let start = self.file.get_location(location.file(), location.start())?;
        let end = self.file.get_location(location.file(), location.end())?;
        Some(lsp_types::Range {
            start: Position {
                line: start.0,
                character: start.1,
            },
            end: Position {
                line: end.0,
                character: end.1,
            },
        })
    }

    fn to_source_location(&self, file: FileId, range: lsp_types::Range) -> Option<SourceLocation> {
        let start = self
            .file
            .get_byte_index(file, range.start.line, range.start.character)?;
        let end = self
            .file
            .get_byte_index(file, range.end.line, range.end.character)?;
        Some(SourceLocation::new(file, start..end))
    }
}

/// Sync state holder, so only the outer layer has to deal with Arcs.
struct Inner<Emit>
where
    Emit: Fn(serde_json::Value) + Send + 'static,
{
    emit: Emit,
    documents: HashMap<Url, Document>,
}

impl<Emit> Inner<Emit>
where
    Emit: Fn(serde_json::Value) + Send + 'static,
{
    /// Convert an rms-check warning to an LSP diagnostic.
    fn to_lsp_diagnostic(
        &self,
        doc: &Document,
        input: &rms_check::Diagnostic,
    ) -> Result<lsp_types::Diagnostic, jsonrpc_core::Error> {
        Ok(Diagnostic {
            range: doc
                .to_lsp_range(input.location())
                .ok_or_else(out_of_range)?,
            severity: Some(match input.severity() {
                Severity::ParseError => DiagnosticSeverity::Error,
                Severity::Error => DiagnosticSeverity::Error,
                Severity::Warning => DiagnosticSeverity::Warning,
                Severity::Hint => DiagnosticSeverity::Information,
            }),
            source: Some("rms-check".to_string()),
            code: input.code().map(str::to_string).map(NumberOrString::String),
            message: input.message().to_string(),
            related_information: Some(
                input
                    .labels()
                    .map(|label| {
                        Ok(DiagnosticRelatedInformation {
                            location: Location {
                                uri: doc
                                    .file
                                    .name(label.location().file())
                                    .parse()
                                    .map_err(internal_error)?,
                                range: doc
                                    .to_lsp_range(label.location())
                                    .ok_or_else(out_of_range)?,
                            },
                            message: label.message().to_string(),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            tags: None,
        })
    }

    /// Initialize the language server.
    fn initialize(&mut self, _params: InitializeParams) -> RpcResult {
        let capabilities = ServerCapabilities {
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            document_formatting_provider: Some(true),
            folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
            signature_help_provider: Some(SignatureHelpOptions {
                trigger_characters: Some(vec![" ".to_string(), "\t".to_string()]),
                retrigger_characters: None,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
            }),
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::Incremental,
            )),
            ..ServerCapabilities::default()
        };
        let result = InitializeResult {
            capabilities,
            server_info: Some(ServerInfo {
                name: "rms-check".to_string(),
                version: None,
            }),
        };
        serde_json::to_value(result).map_err(internal_error)
    }

    /// A document was opened, lint.
    fn opened(&mut self, params: DidOpenTextDocumentParams) -> Result<(), jsonrpc_core::Error> {
        let TextDocumentItem {
            uri, version, text, ..
        } = params.text_document;
        self.documents.insert(
            uri.clone(),
            Document::new(RMSFile::from_string(uri.clone(), text), version),
        );

        self.run_checks_and_publish(uri)
    }

    /// A document changed, re-lint.
    fn changed(&mut self, params: DidChangeTextDocumentParams) -> Result<(), jsonrpc_core::Error> {
        let uri = params.text_document.uri;
        if let Some(doc) = self.documents.get_mut(&uri) {
            if let Some(version) = params.text_document.version {
                if doc.version > version {
                    return Err(jsonrpc_core::Error::invalid_params(format!(
                        "Error applying incremental change: version mismatch: {} > {}",
                        doc.version, version
                    )));
                }
            }

            let mut splicer = Multisplice::new(doc.file.main_source());
            for change in params.content_changes {
                if let Some(range) = change.range {
                    if let Some(location) = doc.to_source_location(doc.file.file_id(), range) {
                        let start = usize::from(location.range().start);
                        let end = usize::from(location.range().end);
                        splicer.splice(start, end, change.text);
                    } else {
                        return Err(jsonrpc_core::Error::invalid_params(
                            "Error applying incremental change: range out of bounds",
                        ));
                    }
                } else {
                    splicer.splice_range(.., change.text);
                }
            }
            doc.version += 1;
            doc.file = RMSFile::from_string(uri.as_str(), splicer.to_string());
            self.run_checks_and_publish(uri)?;
        }

        Ok(())
    }

    /// A document was closed, clean up.
    fn closed(&mut self, params: DidCloseTextDocumentParams) -> Result<(), jsonrpc_core::Error> {
        self.documents.remove(&params.text_document.uri);
        Ok(())
    }

    /// Retrieve code actions for a cursor position.
    fn code_action(&mut self, params: CodeActionParams) -> RpcResult {
        let doc = self
            .documents
            .get(&params.text_document.uri)
            .ok_or_else(unknown_file)?;
        let source_range = doc
            .to_source_location(doc.file.file_id(), params.range)
            .ok_or_else(out_of_range)?;

        let matching_diagnostics = doc.diagnostics.iter().filter(|diagnostic| {
            let range = diagnostic.location().range();
            range.contains(&source_range.start()) || range.contains(&source_range.end())
        });

        let mut code_actions = vec![];
        for diagnostic in matching_diagnostics {
            for fix in diagnostic.fixes().chain(diagnostic.suggestions()) {
                if let Some(replacement) = fix.replacement() {
                    code_actions.push(CodeAction {
                        title: fix.message().to_string(),
                        kind: Some("quickfix".to_string()),
                        diagnostics: Some(vec![self.to_lsp_diagnostic(&doc, diagnostic)?]),
                        edit: Some(WorkspaceEdit {
                            changes: Some({
                                let mut map = HashMap::new();
                                let edit = TextEdit {
                                    range: doc
                                        .to_lsp_range(fix.location())
                                        .ok_or_else(out_of_range)?,
                                    new_text: replacement.to_string(),
                                };
                                map.insert(params.text_document.uri.clone(), vec![edit]);
                                map
                            }),
                            document_changes: None,
                        }),
                        command: None,
                        is_preferred: None,
                    });
                }
            }
        }

        serde_json::to_value(code_actions).map_err(internal_error)
    }

    /// Retrieve folding ranges for the document.
    fn folding_ranges(&self, params: FoldingRangeParams) -> RpcResult {
        let doc = self
            .documents
            .get(&params.text_document.uri)
            .ok_or_else(unknown_file)?;
        let folder = folds::FoldingRanges::new(&doc.file);

        let folds: Vec<FoldingRange> = folder.collect();

        serde_json::to_value(folds).map_err(internal_error)
    }

    /// Get signature help.
    fn signature_help(&self, params: TextDocumentPositionParams) -> RpcResult {
        let doc = self
            .documents
            .get(&params.text_document.uri)
            .ok_or_else(unknown_file)?;
        let Position { line, character } = params.position;
        let help = help::find_signature_help(
            &doc.file,
            doc.file
                .get_byte_index(doc.file.file_id(), line, character)
                .ok_or_else(out_of_range)?,
        );

        serde_json::to_value(help).map_err(internal_error)
    }

    /// Format a document.
    fn format(&self, params: DocumentFormattingParams) -> RpcResult {
        let doc = self
            .documents
            .get(&params.text_document.uri)
            .ok_or_else(unknown_file)?;

        let options = FormatOptions::default()
            .tab_size(params.options.tab_size as u32)
            .use_spaces(params.options.insert_spaces);
        let result = options.format(doc.file.main_source());

        serde_json::to_value(vec![TextEdit {
            range: doc
                .to_lsp_range(SourceLocation::new(
                    doc.file.file_id(),
                    ByteIndex::from(0)..ByteIndex::from(doc.file.main_source().len()),
                ))
                .ok_or_else(out_of_range)?,
            new_text: result,
        }])
        .map_err(internal_error)
    }

    /// Run rms-check.
    fn run_checks(&mut self, uri: Url) {
        let doc = match self.documents.get_mut(&uri) {
            Some(doc) => doc,
            _ => return,
        };

        let result = RMSCheck::default()
            .compatibility(Compatibility::Conquerors)
            .check(&doc.file);

        doc.diagnostics = result.into_iter().collect();
    }

    /// Run rms-check for a file and publish the resulting diagnostics.
    fn run_checks_and_publish(&mut self, uri: Url) -> Result<(), jsonrpc_core::Error> {
        self.run_checks(uri.clone());

        let doc = match self.documents.get(&uri) {
            Some(doc) => doc,
            _ => return Err(unknown_file()),
        };
        let diagnostics: Vec<lsp_types::Diagnostic> = doc
            .diagnostics
            .iter()
            .map(|diagnostic| self.to_lsp_diagnostic(&doc, diagnostic))
            .collect::<Result<Vec<_>, _>>()?;

        let params = PublishDiagnosticsParams::new(uri, diagnostics, Some(doc.version));
        (self.emit)(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": params,
        }));

        Ok(())
    }
}

type Emit = Box<dyn Fn(serde_json::Value) + Send + 'static>;

/// LSP wrapper that handles JSON-RPC.
pub struct RMSCheckLSP {
    inner: Arc<Mutex<Inner<Emit>>>,
    handler: IoHandler,
}

impl RMSCheckLSP {
    /// Create a new rms-check language server.
    ///
    /// The callback is called whenever the language server emits a JSON-RPC message.
    pub fn new(emit: impl Fn(serde_json::Value) + Send + 'static + Sized) -> RMSCheckLSP {
        let mut instance = RMSCheckLSP {
            inner: Arc::new(Mutex::new(Inner {
                emit: Box::new(emit),
                documents: Default::default(),
            })),
            handler: IoHandler::new(),
        };
        instance.install_handlers();
        instance
    }

    /// Install JSON-RPC methods and notification handlers.
    fn install_handlers(&mut self) {
        self.add_method("initialize", |inner, params: InitializeParams| {
            inner.initialize(params)
        });

        self.add_notification("initialized", |_inner, _params: ()| Ok(()));

        self.add_notification(
            "textDocument/didOpen",
            |inner, params: DidOpenTextDocumentParams| inner.opened(params),
        );

        self.add_notification(
            "textDocument/didChange",
            |inner, params: DidChangeTextDocumentParams| inner.changed(params),
        );

        self.add_notification(
            "textDocument/didClose",
            |inner, params: DidCloseTextDocumentParams| inner.closed(params),
        );

        self.add_method(
            "textDocument/codeAction",
            |inner, params: CodeActionParams| inner.code_action(params),
        );

        self.add_method(
            "textDocument/foldingRange",
            |inner, params: FoldingRangeParams| inner.folding_ranges(params),
        );

        self.add_method(
            "textDocument/signatureHelp",
            |inner, params: TextDocumentPositionParams| inner.signature_help(params),
        );

        self.add_method(
            "textDocument/formatting",
            |inner, params: DocumentFormattingParams| inner.format(params),
        );
    }

    fn add_notification<TParams, TCallback>(&mut self, name: &'static str, callback: TCallback)
    where
        TParams: for<'de> serde::Deserialize<'de>,
        TCallback: (Fn(&mut Inner<Emit>, TParams) -> Result<(), jsonrpc_core::Error>)
            + Send
            + Sync
            + 'static,
    {
        let inner = Arc::clone(&self.inner);
        self.handler.add_notification(name, move |params: Params| {
            let params_clone: serde_json::Value = params.clone().into();
            let handle_error = |error: jsonrpc_core::Error| match inner.lock() {
                Ok(guard) => (guard.emit)(json!({
                    "jsonrpc": "2.0",
                    "method": "window/showMessage",
                    "params": ShowMessageParams {
                        typ: MessageType::Error,
                        message: format!(
                            "internal rms-check error while handling notification `{}`\n\nParams: {:?}\nError: {}", name, params_clone, error),
                    },
                })),
                Err(lock_err) => {
                    // if we can't emit there's not much we _can_ do…just hope this never happens
                    // and panic
                    drop(lock_err);
                    panic!("could not obtain lock to emit error: {}", error)
                }
            };

            let params: TParams = match params.parse() {
                Ok(result) => result,
                Err(err) => return handle_error(err),
            };
            let mut guard = match inner.lock() {
                Ok(guard) => guard,
                Err(err) => return handle_error(internal_error(err)),
            };

            match callback(&mut guard, params) {
                Ok(()) => (),
                Err(err) => handle_error(err),
            }
        })
    }

    fn add_method<TParams, TCallback>(&mut self, name: &'static str, callback: TCallback)
    where
        TParams: for<'de> serde::Deserialize<'de>,
        TCallback: (Fn(&mut Inner<Emit>, TParams) -> RpcResult) + Send + Sync + 'static,
    {
        let inner = Arc::clone(&self.inner);
        self.handler.add_method(name, move |params: Params| {
            let params: TParams = params.parse()?;
            let mut guard = inner.lock().map_err(internal_error)?;

            callback(&mut guard, params)
        });
    }

    /// Handle a JSON-RPC message.
    pub fn handle_sync(&mut self, message: serde_json::Value) -> Option<serde_json::Value> {
        self.handler
            .handle_request_sync(&message.to_string())
            .map(|string| string.parse().unwrap())
    }
}
