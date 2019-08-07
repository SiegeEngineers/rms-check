use rms_check_lsp::RMSCheckLSP;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn write_message(message: &str);
}

#[wasm_bindgen]
pub struct RMSCheckServer {
    lsp: RMSCheckLSP,
}

#[wasm_bindgen]
impl RMSCheckServer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let lsp = RMSCheckLSP::new(|message| {
            let message = serde_json::to_string(&message).unwrap();
            write_message(&message);
        });
        Self { lsp }
    }

    pub fn write(&mut self, message: &str) {
        if let Some(response) = self.lsp.handle_sync(message.parse().unwrap()) {
            let response = serde_json::to_string(&response).unwrap();
            write_message(&response);
        }
    }
}
