use std::collections::VecDeque;
use std::num::Wrapping;

use wasm_bindgen::prelude::*;

use session_state::SessionState;
use system::{
    bincode, serde_json, uuid, CommandId, CommandResult, IdentifiableCommand, IdentifiableEvent,
    SessionCommand, SessionEvent,
};

mod session_state;
mod utils;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub struct CanvasSystem {
    command_id_source: Wrapping<CommandId>,
    pending_identifiable_commands: VecDeque<IdentifiableCommand>,
    session: SessionState,
}

#[wasm_bindgen]
impl CanvasSystem {
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8]) -> Result<CanvasSystem, JsValue> {
        utils::set_panic_hook();
        console_log::init_with_level(log::Level::Trace).map_err(|_| JsValue::NULL)?;

        let event = bincode::deserialize::<IdentifiableEvent>(bytes).map_err(|_| JsValue::NULL)?;
        log::trace!("Initializing: {:?}", event);
        if let IdentifiableEvent::BySystem {
            session_event:
                SessionEvent::Init {
                    document_snapshot,
                    session_snapshot,
                    ..
                },
        } = event
        {
            Ok(CanvasSystem {
                command_id_source: Wrapping(0),
                pending_identifiable_commands: VecDeque::new(),
                session: SessionState::new(document_snapshot, session_snapshot),
            })
        } else {
            Err(JsValue::NULL)
        }
    }

    pub fn create_command(&mut self, json: String) -> Result<Box<[u8]>, JsValue> {
        let session_command =
            serde_json::from_str::<SessionCommand>(&json).map_err(|_| JsValue::NULL)?;
        let command_id = self.new_command_id();
        let identifiable_command = IdentifiableCommand {
            command_id,
            session_command,
        };
        Ok(bincode::serialize(&identifiable_command)
            .map_err(|_| JsValue::NULL)?
            .into_boxed_slice())
    }

    pub fn convert_event_to_json(&self, bytes: &[u8]) -> Result<String, JsValue> {
        let event = bincode::deserialize::<IdentifiableEvent>(bytes).map_err(|_| JsValue::NULL)?;
        serde_json::to_string(&event).map_err(|_| JsValue::NULL)
    }

    pub fn handle_event_from_server(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        let event = bincode::deserialize::<IdentifiableEvent>(bytes).map_err(|_| JsValue::NULL)?;
        log::trace!("New event from server: {:?}", event);
        let session_event = match event {
            IdentifiableEvent::ByMyself { result, .. } => match result {
                CommandResult::SessionEvent(session_event) => session_event,
                CommandResult::Error(system_error) => panic!("SystemError: {:?}", system_error),
            },
            IdentifiableEvent::BySystem { session_event } => session_event,
        };
        self.session
            .handle_session_event(session_event)
            .map_err(|_| JsValue::NULL)
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
        if let Ok(tx) = self.session.push_document_command(json) {
            let command_id = self.new_command_id();
            self.pending_identifiable_commands
                .push_back(IdentifiableCommand {
                    command_id,
                    session_command: SessionCommand::Transaction(tx),
                });
        }
    }

    pub fn undo(&mut self) -> Result<(), JsValue> {
        self.session
            .undo()
            .map(|tx| {
                let command_id = self.new_command_id();
                self.pending_identifiable_commands
                    .push_back(IdentifiableCommand {
                        command_id,
                        session_command: SessionCommand::Transaction(tx),
                    });
            })
            .map_err(|_| JsValue::NULL)
    }

    pub fn redo(&mut self) -> Result<(), JsValue> {
        self.session
            .redo()
            .map(|tx| {
                let command_id = self.new_command_id();
                self.pending_identifiable_commands
                    .push_back(IdentifiableCommand {
                        command_id,
                        session_command: SessionCommand::Transaction(tx),
                    });
            })
            .map_err(|_| JsValue::NULL)
    }

    pub fn consume_invalidated_object_ids(&mut self) -> String {
        self.session.consume_invalidated_object_ids()
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
        self.session.materialize_document()
    }

    pub fn materialize_session(&self) -> String {
        self.session.materialize_session()
    }

    pub fn materialize_object(&self, uuid_str: String) -> Result<String, JsValue> {
        let object_id = uuid::Uuid::parse_str(&uuid_str).map_err(|_| JsValue::NULL)?;
        self.session
            .materialize_object(&object_id)
            .ok_or(JsValue::NULL)
    }

    pub fn consume_latest_session_snapshot(&mut self) -> Option<String> {
        self.session.consume_latest_session_snapshot()
    }

    pub fn consume_live_pointer_events(&mut self) -> String {
        self.session.consume_live_pointer_events()
    }

    pub fn terminated(&self) -> bool {
        self.session.terminated()
    }
}
