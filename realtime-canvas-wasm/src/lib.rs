use realtime_canvas_system::{
    bincode, serde_json, ClientReplicaDocument, CommandId, DocumentCommand, IdentifiableCommand,
    IdentifiableEvent, Materialize, ObjectId, SessionCommand, SystemCommand, Transaction,
};
use std::collections::{HashSet, VecDeque};
use std::num::Wrapping;
use wasm_bindgen::__rt::std::alloc::System;
use wasm_bindgen::prelude::*;

mod utils;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub struct CanvasSystem {
    command_id_source: Wrapping<CommandId>,
    local_document: ClientReplicaDocument,
    invalidated_object_ids: HashSet<ObjectId>,
    pending_identifiable_commands: VecDeque<IdentifiableCommand>,
}

#[wasm_bindgen]
impl CanvasSystem {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        utils::set_panic_hook();
        console_log::init_with_level(log::Level::Trace);

        CanvasSystem {
            command_id_source: Wrapping(0),
            local_document: ClientReplicaDocument::new(),
            invalidated_object_ids: HashSet::new(),
            pending_identifiable_commands: VecDeque::new(),
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

    pub fn push_document_command(&mut self, json: String) {
        let command = serde_json::from_str::<DocumentCommand>(&json).unwrap();

        if self.invalidated_object_ids.len() > 0 {
            log::warn!("invalidate_object_ids must be consumed for each command");
        }
        // TODO: Err
        if let Ok(result) = self.local_document.handle_command(command) {
            for invalidated_object_id in result.invalidated_object_ids {
                self.invalidated_object_ids.insert(invalidated_object_id);
            }
            let command_id = self.new_command_id();
            self.pending_identifiable_commands
                .push_back(IdentifiableCommand {
                    command_id,
                    system_command: SystemCommand::SessionCommand(SessionCommand::Transaction(
                        result.transaction,
                    )),
                });
        }
    }

    pub fn consume_invalidated_object_ids(&mut self) -> String {
        let result = serde_json::to_string(&self.invalidated_object_ids).unwrap();
        self.invalidated_object_ids.clear();
        result
    }

    pub fn consume_pending_identifiable_command(&mut self) -> Option<Box<[u8]>> {
        self.pending_identifiable_commands
            .pop_front()
            .and_then(|command| {
                log::trace!("Consumed: {:?}", command);
                bincode::serialize(&command)
                    .ok()
                    .map(|v| v.into_boxed_slice())
            })
    }

    pub fn materialize_document(&self) -> String {
        let document = self.local_document.materialize_document();
        return serde_json::to_string(&document).unwrap();
    }
}
