use jsonrpc_core::{ErrorCode, IoHandler, Params};
use languageserver_types::{
    CodeAction, CodeActionParams, CodeActionProviderCapability, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
    PublishDiagnosticsParams, ServerCapabilities, TextDocumentItem, TextDocumentSyncCapability,
    TextDocumentSyncKind, Url,
};
use rms_check::{Compatibility, RMSCheck, RMSCheckResult};
use serde_json::{self, json};
use std::{
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
    fn initialize(&mut self, params: InitializeParams) -> RpcResult {
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
        match self.documents.get_mut(&params.text_document.uri) {
            Some(doc) => {
                doc.text = params.content_changes.remove(0).text;
                self.check_and_publish(params.text_document.uri);
            }
            _ => (),
        };
    }

    fn closed(&mut self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
    }

    fn code_action(&self, params: CodeActionParams) {
        (self.emit)(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": vec![CodeAction {
                kind: Some("quickfix".to_string()),
                title: "Delete everything".to_string(),
                edit: None,
                diagnostics: None,
                command: None,
            }],
        }));

        // let result = self.check(&params.text_document);
        // for warn in result.iter() {
        //     if let Some(label) = warn.diagnostic().labels.first() {
        //     }
        // }
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
            let diag = codespan_lsp::make_lsp_diagnostic(
                result.codemap(),
                warn.diagnostic().clone(),
                |_filename| Ok(doc.uri.clone()),
            )
            .unwrap();
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
                .add_notification("textDocument/codeAction", move |params: Params| {
                    let params: CodeActionParams = params.parse().unwrap();
                    inner.lock().unwrap().code_action(params)
                });
        }
    }

    pub fn handle_sync(&mut self, message: serde_json::Value) -> Option<serde_json::Value> {
        self.handler
            .handle_request_sync(&message.to_string())
            .map(|string| string.parse().unwrap())
    }
}
