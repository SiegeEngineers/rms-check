use jsonrpc_core::{ErrorCode, IoHandler, Params};
use languageserver_types::{
    CodeActionProviderCapability, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, InitializeParams, InitializeResult, PublishDiagnosticsParams,
    ServerCapabilities, TextDocumentItem, TextDocumentSyncCapability, TextDocumentSyncKind,
};
use rms_check::{Compatibility, RMSCheck};
use serde_json::{self, json};
use std::sync::{Arc, Mutex};

type RpcResult = jsonrpc_core::Result<serde_json::Value>;

struct Inner<Emit>
where
    Emit: Fn(serde_json::Value) + Send + 'static,
{
    emit: Emit,
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
        let doc = params.text_document;

        self.check_and_publish(doc);
    }

    fn changed(&mut self, params: DidChangeTextDocumentParams) {}
    fn closed(&mut self, params: DidCloseTextDocumentParams) {}

    fn check_and_publish(&self, doc: TextDocumentItem) {
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
            inner: Arc::new(Mutex::new(Inner { emit })),
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
    }

    /*
    fn emit(&self, message: serde_json::Value) {
        let emit = self.inner.lock().unwrap().emit;
        emit(message);
    }
    */

    pub fn handle_sync(&mut self, message: serde_json::Value) -> Option<serde_json::Value> {
        self.handler
            .handle_request_sync(&message.to_string())
            .map(|string| string.parse().unwrap())
    }
}
