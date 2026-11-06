use serde::Serialize;
use untwist::{Output, SuppliedWasm, WasmFile, WasmOrigin, WasmReference, WasmSupply};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
#[derive(Default)]
pub struct WasmFiles {
    supply: WasmSupply,
}

#[wasm_bindgen]
impl WasmFiles {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, name: String, bytes: Vec<u8>, url: Option<String>) {
        let origin = match url {
            Some(_) => WasmOrigin::Fetched,
            None => WasmOrigin::Supplied,
        };
        self.supply.files.push(SuppliedWasm {
            name,
            bytes,
            origin,
            url,
        });
    }
}

impl WasmFiles {
    pub fn supply(&self) -> &WasmSupply {
        &self.supply
    }
}

#[derive(Serialize)]
struct WasmJson<'a> {
    files: &'a [WasmFile],
    unresolved: &'a [WasmReference],
    unchecked: bool,
}

pub fn settle(result: &mut Output, files: &WasmFiles) -> (String, Vec<Vec<u8>>) {
    result.settle_wasm(files.supply(), true);
    let json = serde_json::to_string(&WasmJson {
        files: &result.wasm,
        unresolved: &result.wasm_unresolved,
        unchecked: result.wasm_unchecked,
    })
    .unwrap_or_default();
    let bytes = result.wasm.iter().map(|file| file.bytes.clone()).collect();
    (json, bytes)
}
