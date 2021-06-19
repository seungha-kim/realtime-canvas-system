use std::collections::VecDeque;
use std::num::Wrapping;

use wasm_bindgen::prelude::*;

use session_state::SessionState;
use system::{
    bincode, serde_json, uuid, CommandId, CommandResult, IdentifiableCommand, IdentifiableEvent,
    SessionCommand, SystemCommand, SystemEvent,
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
    session: Option<SessionState>,
}

#[wasm_bindgen]
impl CanvasSystem {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        utils::set_panic_hook();
        console_log::init_with_level(log::Level::Trace).unwrap();

        CanvasSystem {
            command_id_source: Wrapping(0),
            pending_identifiable_commands: VecDeque::new(),
            session: None,
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

    pub fn handle_event_from_server(&mut self, bytes: &[u8]) {
        let event = bincode::deserialize::<IdentifiableEvent>(bytes).unwrap();
        log::trace!("New event from server: {:?}", event);
        let system_event = match event {
            IdentifiableEvent::ByMyself { result, .. } => match result {
                CommandResult::SystemEvent(system_event) => system_event,
                CommandResult::Error(system_error) => panic!("SystemError: {:?}", system_error),
            },
            IdentifiableEvent::BySystem { system_event } => system_event,
        };
        match system_event {
            SystemEvent::SessionEvent(session_event) => {
                self.session
                    .as_mut()
                    .map(|s| s.handle_session_event(session_event));
            }
            SystemEvent::JoinedSession {
                session_id,
                document_snapshot,
                session_snapshot,
            } => {
                self.session = Some(SessionState::new(
                    session_id,
                    document_snapshot,
                    session_snapshot,
                ));
            }
            system_event => {
                log::warn!("Unhandled SystemEvent: {:?}", system_event);
            }
        }
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
        self.session
            .as_mut()
            .and_then(|s| s.push_document_command(json).ok())
            .map(|tx| {
                let command_id = self.new_command_id();
                self.pending_identifiable_commands
                    .push_back(IdentifiableCommand {
                        command_id,
                        system_command: SystemCommand::SessionCommand(SessionCommand::Transaction(
                            tx,
                        )),
                    });
            });
    }

    pub fn undo(&mut self) {
        self.session.as_mut().and_then(|s| s.undo().ok()).map(|tx| {
            let command_id = self.new_command_id();
            self.pending_identifiable_commands
                .push_back(IdentifiableCommand {
                    command_id,
                    system_command: SystemCommand::SessionCommand(SessionCommand::Transaction(tx)),
                });
        });
    }

    pub fn consume_invalidated_object_ids(&mut self) -> String {
        self.session
            .as_mut()
            .map(|s| s.consume_invalidated_object_ids())
            .unwrap_or("[]".into())
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

    pub fn materialize_document(&self) -> Option<String> {
        self.session.as_ref().map(|s| s.materialize_document())
    }

    pub fn materialize_session(&self) -> Option<String> {
        self.session.as_ref().map(|s| s.materialize_session())
    }

    pub fn materialize_object(&self, uuid_str: String) -> Option<String> {
        let object_id = uuid::Uuid::parse_str(&uuid_str).unwrap();
        self.session
            .as_ref()
            .map(|s| s.materialize_object(&object_id))
    }

    pub fn consume_latest_session_snapshot(&mut self) -> Option<String> {
        self.session
            .as_mut()
            .and_then(|s| s.consume_latest_session_snapshot())
    }

    pub fn consume_live_pointer_events(&mut self) -> Option<String> {
        self.session
            .as_mut()
            .map(|s| s.consume_live_pointer_events())
    }
}
