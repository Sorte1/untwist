use untwist::{Progress, ProgressSink};

#[cfg(target_arch = "wasm32")]
mod listener {
    use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(catch, js_namespace = globalThis, js_name = untwistStage)]
        fn untwist_stage(name: &str) -> Result<(), JsValue>;
    }

    pub fn announce(name: &str) {
        untwist_stage(name).ok();
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod listener {
    pub fn announce(_: &str) {}
}

pub fn sink() -> ProgressSink {
    ProgressSink::new(|progress| {
        if let Progress::StageBegins(name) = progress {
            listener::announce(name);
        }
    })
}
