mod utils;

use system::{
    bincode, serde_json, CommandId, IdentifiableCommand, IdentifiableEvent, SystemCommand,
};
use wasm_bindgen::__rt::core::num::Wrapping;
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub struct CanvasSystem {
    command_id_source: Wrapping<CommandId>,
}

#[wasm_bindgen]
impl CanvasSystem {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        utils::set_panic_hook();

        CanvasSystem {
            command_id_source: Wrapping(0),
        }
    }

    pub fn create_command(&mut self, json: String) -> Box<[u8]> {
        let system_command = serde_json::from_str::<SystemCommand>(&json).unwrap();
        let command_id = self.new_command_id();
        let identifiable_command = IdentifiableCommand {
            command_id,
            system_command,
        };
        bincode::serialize(&identifiable_command)
            .unwrap()
            .into_boxed_slice()
    }

    pub fn convert_event_to_json(&self, bytes: &[u8]) -> String {
        let event = bincode::deserialize::<IdentifiableEvent>(bytes).unwrap();
        serde_json::to_string(&event).unwrap()
    }

    pub fn last_command_id(&self) -> CommandId {
        self.command_id_source.0
    }

    // NOTE: Rust 객체를 넘기는 쪽도 해봤는데, 이렇게 하면 일일이 free 호출해주어야 함.
    // 그냥 JSON 넘기는 쪽이 메모리 관리 측면에서 걱정이 없어서 좋다.

    fn new_command_id(&mut self) -> CommandId {
        self.command_id_source += Wrapping(1);
        self.command_id_source.0
    }
}
