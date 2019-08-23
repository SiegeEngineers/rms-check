//! Transport-agnostic language server protocol implementation for Age of Empires 2 random map
//! scripts, using rms-check.

#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(unused)]

use codespan::{FileId, Files};
use help::find_signature_help;
use jsonrpc_core::{ErrorCode, IoHandler, Params};
use lsp_types::{
    CodeAction, CodeActionParams, CodeActionProviderCapability, Diagnostic,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    FoldingRange, FoldingRangeParams, FoldingRangeProviderCapability, InitializeParams,
    InitializeResult, NumberOrString, PublishDiagnosticsParams, ServerCapabilities,
    SignatureHelpOptions, TextDocumentItem, TextDocumentPositionParams, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextEdit, Url, WorkspaceEdit,
};
use rms_check::{AutoFixReplacement, Compatibility, RMSCheck, RMSCheckResult, Warning};
use serde_json::{self, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

mod folds;
mod help;

type RpcResult = jsonrpc_core::Result<serde_json::Value>;

/// Sync state holder, so only the outer layer has to deal with Arcs.
struct Inner<Emit>
where
    Emit: Fn(serde_json::Value) + Send + 'static,
{
    emit: Emit,
    documents: HashMap<Url, TextDocumentItem>,
}

impl<Emit> Inner<Emit>
where
    Emit: Fn(serde_json::Value) + Send + 'static,
{
    /// Convert a codespan file name to an LSP file URL.
    fn codespan_name_to_url(&self, filename: &str) -> Result<Url, ()> {
        if filename.starts_with("file://") {
            return filename.parse().map_err(|_| ());
        }

        Err(())
    }

    /// Convert an rms-check warning to an LSP diagnostic.
    fn make_lsp_diagnostic(&self, (_, files): (&[FileId], &Files), warn: &Warning) -> Diagnostic {
        let diag = codespan_lsp::make_lsp_diagnostic(
            files,
            "rms-check".to_string(),
            warn.diagnostic().clone(),
            |f| self.codespan_name_to_url(files.name(f)),
        )
        .unwrap();

        Diagnostic {
            code: warn
                .diagnostic()
                .code
                .as_ref()
                .map(|code| NumberOrString::String(code.to_string())),
            ..diag
        }
    }

    /// Initialize the language server.
    fn initialize(&mut self, _params: InitializeParams) -> RpcResult {
        let mut capabilities = ServerCapabilities::default();
        capabilities.code_action_provider = Some(CodeActionProviderCapability::Simple(true));
        capabilities.text_document_sync =
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::Full));
        capabilities.folding_range_provider = Some(FoldingRangeProviderCapability::Simple(true));
        capabilities.signature_help_provider = Some(SignatureHelpOptions {
            trigger_characters: Some(vec![" ".to_string(), "\t".to_string()]),
        });
        let result = InitializeResult { capabilities };
        serde_json::to_value(result).map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))
    }

    /// A document was opened, lint.
    fn opened(&mut self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        self.documents.insert(uri.clone(), params.text_document);

        self.check_and_publish(uri);
    }

    /// A document changed, re-lint.
    fn changed(&mut self, mut params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        match self.documents.get_mut(&params.text_document.uri) {
            Some(doc) => {
                doc.text = params.content_changes.remove(0).text;
                self.check_and_publish(uri);
            }
            _ => (),
        };
    }

    /// A document was closed, clean up.
    fn closed(&mut self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
    }

    /// Retrieve code actions for a cursor position.
    fn code_action(&mut self, params: CodeActionParams) -> RpcResult {
        let doc = self.documents.get(&params.text_document.uri).unwrap();
        let result = self.check(&doc);
        let filename = doc.uri.to_string();
        let (file_ids, files) = result.files();
        let file_id = file_ids
            .iter()
            .cloned()
            .find(|&id| files.name(id) == &filename)
            .ok_or(jsonrpc_core::Error::new(ErrorCode::InternalError))?;
        let start = codespan_lsp::position_to_byte_index(files, file_id, &params.range.start)
            .map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))?;
        let end = codespan_lsp::position_to_byte_index(files, file_id, &params.range.end)
            .map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))?;

        let warnings = result.iter().filter(|warn| {
            let label = &warn.diagnostic().primary_label;
            start >= label.span.start() && start <= label.span.end()
                || end >= label.span.start() && end <= label.span.end()
        });

        let mut actions = vec![];
        for warn in warnings {
            for sugg in warn.suggestions() {
                if !sugg.replacement().is_fixable() {
                    continue;
                }
                actions.push(CodeAction {
                    title: sugg.message().to_string(),
                    kind: Some("quickfix".to_string()),
                    diagnostics: Some(vec![self.make_lsp_diagnostic(result.files(), warn)]),
                    edit: Some(WorkspaceEdit {
                        changes: Some({
                            let mut map = HashMap::new();
                            map.insert(
                                doc.uri.clone(),
                                vec![TextEdit {
                                    range: codespan_lsp::byte_span_to_range(
                                        files,
                                        file_id,
                                        sugg.span(),
                                    )
                                    .unwrap(),
                                    new_text: match sugg.replacement() {
                                        AutoFixReplacement::Safe(s) => s.clone(),
                                        _ => unreachable!(),
                                    },
                                }],
                            );
                            map
                        }),
                        document_changes: None,
                    }),
                    command: None,
                });
            }
        }

        serde_json::to_value(actions)
            .map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))
    }

    /// Retrieve folding ranges for the document.
    fn folding_ranges(&self, params: FoldingRangeParams) -> RpcResult {
        let doc = self.documents.get(&params.text_document.uri).unwrap();
        let mut files = Files::new();
        let file_id = files.add(doc.uri.as_str(), &doc.text);
        let folder = folds::FoldingRanges::new(&files, file_id);

        let folds: Vec<FoldingRange> = folder.collect();

        serde_json::to_value(folds).map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))
    }

    /// Get signature help.
    fn signature_help(&self, params: TextDocumentPositionParams) -> RpcResult {
        let doc = self.documents.get(&params.text_document.uri).unwrap();
        let mut files = Files::new();
        let file_id = files.add(doc.uri.as_str(), &doc.text);
        let help = help::find_signature_help(
            &files,
            file_id,
            codespan_lsp::position_to_byte_index(&files, file_id, &params.position)
                .map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))?,
        );

        serde_json::to_value(help).map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))
    }

    /// Run rms-check.
    fn check(&self, doc: &TextDocumentItem) -> RMSCheckResult {
        RMSCheck::default()
            .compatibility(Compatibility::Conquerors)
            .add_source(doc.uri.as_str(), &doc.text)
            .check()
    }

    /// Run rms-check for a file and publish the resulting diagnostics.
    fn check_and_publish(&self, uri: Url) {
        let mut diagnostics = vec![];
        let doc = match self.documents.get(&uri) {
            Some(doc) => doc,
            _ => return,
        };
        let result = self.check(&doc);
        for warn in result.iter() {
            let diag = self.make_lsp_diagnostic(result.files(), warn);
            diagnostics.push(diag);
        }

        let params = PublishDiagnosticsParams::new(doc.uri.clone(), diagnostics);
        (self.emit)(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": params,
        }));
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
        {
            let inner = Arc::clone(&self.inner);
            self.handler
                .add_method("initialize", move |params: Params| {
                    let params: InitializeParams = params.parse()?;
                    inner
                        .lock()
                        .map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))?
                        .initialize(params)
                });
        }

        self.handler
            .add_notification("initialized", move |_params: Params| {});

        {
            let inner = Arc::clone(&self.inner);
            self.handler
                .add_notification("textDocument/didOpen", move |params: Params| {
                    let params: DidOpenTextDocumentParams = params.parse().unwrap();
                    inner.lock().unwrap().opened(params)
                });
        }

        {
            let inner = Arc::clone(&self.inner);
            self.handler
                .add_notification("textDocument/didChange", move |params: Params| {
                    let params: DidChangeTextDocumentParams = params.parse().unwrap();
                    inner.lock().unwrap().changed(params)
                });
        }

        {
            let inner = Arc::clone(&self.inner);
            self.handler
                .add_notification("textDocument/didClose", move |params: Params| {
                    let params: DidCloseTextDocumentParams = params.parse().unwrap();
                    inner.lock().unwrap().closed(params)
                });
        }

        {
            let inner = Arc::clone(&self.inner);
            self.handler
                .add_method("textDocument/codeAction", move |params: Params| {
                    let params: CodeActionParams = params.parse().unwrap();
                    inner
                        .lock()
                        .map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))?
                        .code_action(params)
                });
        }

        {
            let inner = Arc::clone(&self.inner);
            self.handler
                .add_method("textDocument/foldingRange", move |params: Params| {
                    let params: FoldingRangeParams = params.parse().unwrap();
                    inner
                        .lock()
                        .map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))?
                        .folding_ranges(params)
                });
        }

        {
            let inner = Arc::clone(&self.inner);
            self.handler
                .add_method("textDocument/signatureHelp", move |params: Params| {
                    let params: TextDocumentPositionParams = params.parse().unwrap();
                    inner
                        .lock()
                        .map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))?
                        .signature_help(params)
                });
        }
    }

    /// Handle a JSON-RPC message.
    pub fn handle_sync(&mut self, message: serde_json::Value) -> Option<serde_json::Value> {
        self.handler
            .handle_request_sync(&message.to_string())
            .map(|string| string.parse().unwrap())
    }
}
