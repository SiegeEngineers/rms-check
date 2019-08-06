use codespan::{CodeMap, FileName};
use jsonrpc_core::{ErrorCode, IoHandler, Params};
use languageserver_types::{
    CodeAction, CodeActionParams, CodeActionProviderCapability, Diagnostic,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, NumberOrString, PublishDiagnosticsParams,
    ServerCapabilities, TextDocumentItem, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use rms_check::{Compatibility, RMSCheck, RMSCheckResult, Warning};
use serde_json::{self, json};
use std::{
    cmp::Ordering,
    collections::HashMap,
    sync::{Arc, Mutex},
};

type RpcResult = jsonrpc_core::Result<serde_json::Value>;

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
    fn codemap_name_to_file(&self, filename: &FileName) -> Result<Url, ()> {
        let filename = match filename {
            FileName::Virtual(filename) => filename.to_string(),
            // should not be any real filenames when using the language server
            FileName::Real(_) => return Err(()),
        };

        if filename.starts_with("file://") {
            return filename.parse().map_err(|_| ());
        }

        Err(())
    }

    fn make_lsp_diagnostic(&self, codemap: &CodeMap, warn: &Warning) -> Diagnostic {
        let diag = codespan_lsp::make_lsp_diagnostic(codemap, warn.diagnostic().clone(), |f| {
            self.codemap_name_to_file(f)
        })
        .unwrap();

        Diagnostic {
            code: warn
                .diagnostic()
                .code
                .as_ref()
                .map(|code| NumberOrString::String(code.to_string())),
            source: Some("rms-check".to_string()),
            ..diag
        }
    }

    fn initialize(&mut self, _params: InitializeParams) -> RpcResult {
        let mut capabilities = ServerCapabilities::default();
        capabilities.code_action_provider = Some(CodeActionProviderCapability::Simple(true));
        capabilities.text_document_sync =
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::Full));
        let result = InitializeResult { capabilities };
        serde_json::to_value(result).map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))
    }

    fn opened(&mut self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        self.documents.insert(uri.clone(), params.text_document);

        self.check_and_publish(uri);
    }

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

    fn closed(&mut self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
    }

    fn code_action(&mut self, params: CodeActionParams) -> RpcResult {
        let doc = self.documents.get(&params.text_document.uri).unwrap();
        let result = self.check(&doc);
        let filename = doc.uri.to_string();
        let file_map = result
            .codemap()
            .iter()
            .find(|map| match map.name() {
                FileName::Virtual(n) => *n == filename,
                _ => false,
            })
            .unwrap();
        let start = codespan_lsp::position_to_byte_index(file_map, &params.range.start)
            .map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))?;
        let end = codespan_lsp::position_to_byte_index(file_map, &params.range.end)
            .map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))?;

        let warnings = result.iter().filter(|warn| {
            if let Some(label) = warn.diagnostic().labels.first() {
                return label.span.containment(start) == Ordering::Equal
                    || label.span.containment(end) == Ordering::Equal;
            }
            return false;
        });

        let mut actions = vec![];
        for warn in warnings {
            for sugg in warn.suggestions() {
                actions.push(CodeAction {
                    title: sugg.message().to_string(),
                    kind: Some("quickfix".to_string()),
                    diagnostics: Some(vec![self.make_lsp_diagnostic(result.codemap(), warn)]),
                    edit: None,
                    command: None,
                });
            }
        }

        serde_json::to_value(actions)
            .map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))
    }

    fn check(&self, doc: &TextDocumentItem) -> RMSCheckResult {
        RMSCheck::default()
            .compatibility(Compatibility::Conquerors)
            .add_source(doc.uri.as_str(), &doc.text)
            .check()
    }

    fn check_and_publish(&self, uri: Url) {
        let mut diagnostics = vec![];
        let doc = match self.documents.get(&uri) {
            Some(doc) => doc,
            _ => return,
        };
        let result = self.check(&doc);
        for warn in result.iter() {
            let diag = self.make_lsp_diagnostic(result.codemap(), warn);
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

pub struct RMSCheckLSP<Emit>
where
    Emit: Fn(serde_json::Value) + Send + 'static,
{
    inner: Arc<Mutex<Inner<Emit>>>,
    handler: IoHandler,
}

impl<Emit> RMSCheckLSP<Emit>
where
    Emit: Fn(serde_json::Value) + Send + 'static,
{
    pub fn new(emit: Emit) -> RMSCheckLSP<Emit> {
        let mut instance = RMSCheckLSP {
            inner: Arc::new(Mutex::new(Inner {
                emit,
                documents: Default::default(),
            })),
            handler: IoHandler::new(),
        };
        instance.install_handlers();
        instance
    }

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
    }

    pub fn handle_sync(&mut self, message: serde_json::Value) -> Option<serde_json::Value> {
        self.handler
            .handle_request_sync(&message.to_string())
            .map(|string| string.parse().unwrap())
    }
}
