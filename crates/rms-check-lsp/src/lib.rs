use codespan::{CodeMap, FileName};
use jsonrpc_core::{ErrorCode, IoHandler, Params};
use languageserver_types::{
    CodeAction, CodeActionParams, CodeActionProviderCapability, Diagnostic,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    FoldingRange, FoldingRangeParams, FoldingRangeProviderCapability, InitializeParams,
    InitializeResult, NumberOrString, PublishDiagnosticsParams, ServerCapabilities,
    TextDocumentItem, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use rms_check::{Compatibility, Parser, RMSCheck, RMSCheckResult, Warning};
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
        capabilities.folding_range_provider = Some(FoldingRangeProviderCapability::Simple(true));
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

    fn folding_ranges(&self, params: FoldingRangeParams) -> RpcResult {
        let doc = self.documents.get(&params.text_document.uri).unwrap();
        let mut map = CodeMap::new();
        let file = map.add_filemap(FileName::virtual_(doc.uri.to_string()), doc.text.clone());
        let parser = Parser::new(&*file);

        let line = |index| file.location(index).unwrap().0.to_usize() as u64;
        let col = |index| file.location(index).unwrap().1.to_usize() as u64;
        let fold = || FoldingRange {
            start_line: 0,
            end_line: 0,
            start_character: Default::default(),
            end_character: Default::default(),
            kind: Default::default(),
        };

        let mut folds = vec![];
        let mut waiting_folds = vec![];
        use rms_check::Atom::*;
        for (atom, _) in parser {
            match atom {
                Comment(start, _, Some(end)) => {
                    let start_line = line(start.span.start());
                    let end_line = line(end.span.start());
                    if end_line > start_line {
                        folds.push(FoldingRange {
                            start_line,
                            end_line,
                            end_character: Some(col(end.span.end())),
                            ..fold()
                        });
                    }
                }
                OpenBlock(_) => waiting_folds.push(atom),
                CloseBlock(end) => match waiting_folds.pop() {
                    Some(OpenBlock(start)) => folds.push(FoldingRange {
                        start_line: line(start.span.end()),
                        end_line: line(end.span.start()),
                        start_character: Some(col(start.span.end())),
                        end_character: Some(col(end.span.start())),
                        ..fold()
                    }),
                    _ => (),
                },
                If(_, _) => waiting_folds.push(atom),
                ElseIf(end, _) | Else(end) => {
                    let start = match waiting_folds.pop() {
                        Some(If(start, _)) | Some(ElseIf(start, _)) => start,
                        _ => continue,
                    };
                    let start_line = line(start.span.start());
                    let mut end_line = line(end.span.start());
                    if end_line > start_line {
                        end_line -= 1;
                        folds.push(FoldingRange {
                            start_line,
                            end_line,
                            ..fold()
                        });
                    }
                    waiting_folds.push(atom);
                }
                EndIf(end) => match waiting_folds.pop() {
                    Some(If(start, _)) | Some(ElseIf(start, _)) | Some(Else(start)) => {
                        folds.push(FoldingRange {
                            start_line: line(start.span.start()),
                            end_line: line(end.span.start()),
                            ..fold()
                        })
                    }
                    _ => (),
                },
                StartRandom(_) => waiting_folds.push(atom),
                PercentChance(end, _) => {
                    if let Some(PercentChance(start, _)) = waiting_folds.last() {
                        let start_line = line(start.span.start());
                        let mut end_line = line(end.span.start());
                        if end_line > start_line {
                            end_line -= 1;
                            folds.push(FoldingRange {
                                start_line,
                                end_line,
                                ..fold()
                            });
                        }
                        waiting_folds.pop();
                    }
                    waiting_folds.push(atom);
                }
                EndRandom(end) => {
                    if let Some(PercentChance(start, _)) = waiting_folds.last() {
                        let start_line = line(start.span.start());
                        let mut end_line = line(end.span.start());
                        if end_line > start_line {
                            end_line -= 1;
                            folds.push(FoldingRange {
                                start_line,
                                end_line,
                                ..fold()
                            });
                        }
                        waiting_folds.pop();
                    }
                    if let Some(StartRandom(start)) = waiting_folds.last() {
                        folds.push(FoldingRange {
                            start_line: line(start.span.start()),
                            end_line: line(end.span.start()),
                            ..fold()
                        });
                        waiting_folds.pop();
                    }
                }
                _ => (),
            }
        }

        serde_json::to_value(folds).map_err(|_| jsonrpc_core::Error::new(ErrorCode::InternalError))
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

type Emit = Box<dyn Fn(serde_json::Value) + Send + 'static>;
pub struct RMSCheckLSP {
    inner: Arc<Mutex<Inner<Emit>>>,
    handler: IoHandler,
}

impl RMSCheckLSP {
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
    }

    pub fn handle_sync(&mut self, message: serde_json::Value) -> Option<serde_json::Value> {
        self.handler
            .handle_request_sync(&message.to_string())
            .map(|string| string.parse().unwrap())
    }
}
