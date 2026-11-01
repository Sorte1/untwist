#[cfg(target_arch = "wasm32")]
mod performance {
    use wasm_bindgen::prelude::wasm_bindgen;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = performance, js_name = now)]
        pub fn now() -> f64;
    }
}

#[cfg(target_arch = "wasm32")]
pub fn register() {
    untwist_core::clock::set_stage_clock(performance::now);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn register() {}
