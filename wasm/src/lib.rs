mod utils;

use system::{bincode, serde_json, SystemCommand, SystemEvent};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub struct CanvasSystem {}

#[wasm_bindgen]
pub enum SystemError {
    InvalidCanvasId,
}

#[wasm_bindgen]
impl CanvasSystem {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        utils::set_panic_hook();

        CanvasSystem {}
    }

    pub fn translate_command_to_json(&self, bytes: &[u8]) -> String {
        let command = bincode::deserialize::<SystemCommand>(bytes).unwrap();
        serde_json::to_string(&command).unwrap()
    }

    pub fn translate_command_from_json(&self, json: String) -> Box<[u8]> {
        let command = serde_json::from_str::<SystemCommand>(&json).unwrap();
        bincode::serialize(&command).unwrap().into_boxed_slice()
    }

    pub fn translate_event_to_json(&self, bytes: &[u8]) -> String {
        let command = bincode::deserialize::<SystemEvent>(bytes).unwrap();
        serde_json::to_string(&command).unwrap()
    }

    pub fn translate_event_from_json(&self, json: String) -> Box<[u8]> {
        let command = serde_json::from_str::<SystemEvent>(&json).unwrap();
        bincode::serialize(&command).unwrap().into_boxed_slice()
    }

    // NOTE: Rust 객체를 넘기는 쪽도 해봤는데, 이렇게 하면 일일이 free 호출해주어야 함.
    // 그냥 JSON 넘기는 쪽이 메모리 관리 측면에서 걱정이 없어서 좋다.
}
