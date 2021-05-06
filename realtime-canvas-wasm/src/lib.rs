use realtime_canvas_system::{
    bincode, serde_json, ClientReplicaDocument, CommandId, CommandResult, DocumentCommand,
    IdentifiableCommand, IdentifiableEvent, Materialize, ObjectId, SessionCommand, SessionEvent,
    SessionId, SystemCommand, SystemEvent, Transaction,
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

enum SystemState {
    Lobby,
    Joined(SessionState),
}

struct SessionState {
    session_id: SessionId,
    document: ClientReplicaDocument,
    invalidated_object_ids: HashSet<ObjectId>,
}

#[wasm_bindgen]
pub struct CanvasSystem {
    command_id_source: Wrapping<CommandId>,
    pending_identifiable_commands: VecDeque<IdentifiableCommand>,
    state: SystemState,
}

#[wasm_bindgen]
impl CanvasSystem {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        utils::set_panic_hook();
        console_log::init_with_level(log::Level::Trace);

        CanvasSystem {
            command_id_source: Wrapping(0),
            pending_identifiable_commands: VecDeque::new(),
            state: SystemState::Lobby,
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

    pub fn push_event(&mut self, bytes: &[u8]) {
        let event = bincode::deserialize::<IdentifiableEvent>(bytes).unwrap();
        log::trace!("New event from server: {:?}", event);
        let system_event = match event {
            IdentifiableEvent::ByMyself { command_id, result } => match result {
                CommandResult::SystemEvent(system_event) => system_event,
                CommandResult::Error(system_error) => panic!("SystemError: {:?}", system_error),
            },
            IdentifiableEvent::BySystem { system_event } => system_event,
        };
        match system_event {
            SystemEvent::SessionEvent(session_event) => match session_event {
                SessionEvent::TransactionAck(tx_id) => {
                    if let SystemState::Joined(ref mut session_state) = self.state {
                        session_state.document.handle_ack(&tx_id);
                    } else {
                        log::warn!("System isn't a session");
                    }
                }
                SessionEvent::TransactionNack(tx_id, _reason) => {
                    if let SystemState::Joined(ref mut session_state) = self.state {
                        session_state.document.handle_nack(&tx_id);
                    } else {
                        log::warn!("System isn't a session");
                    }
                }
                SessionEvent::OthersTransaction(tx) => {
                    if let SystemState::Joined(ref mut session_state) = self.state {
                        if let Ok(result) = session_state.document.handle_transaction(tx) {
                            for id in result.invalidated_object_ids {
                                session_state.invalidated_object_ids.insert(id);
                            }
                        }
                    } else {
                        log::warn!("System isn't in a session");
                    }
                }
                SessionEvent::SomeoneJoined(connection_id) => {
                    // TODO: invalidate UI from system
                    log::info!("{} joined", connection_id);
                }
                SessionEvent::SomeoneLeft(connection_id) => {
                    // TODO: invalidate UI from system
                    log::info!("{} left", connection_id);
                }
                SessionEvent::Fragment(fragment) => {
                    log::trace!("Fragment: {:?}", fragment);
                }
            },
            SystemEvent::JoinedSession {
                session_id,
                document_snapshot,
                initial_state,
            } => {
                self.state = SystemState::Joined(SessionState {
                    session_id,
                    document: ClientReplicaDocument::new(document_snapshot),
                    invalidated_object_ids: HashSet::new(),
                })
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
        if let SystemState::Joined(ref mut session_state) = self.state {
            let command = serde_json::from_str::<DocumentCommand>(&json).unwrap();

            if session_state.invalidated_object_ids.len() > 0 {
                log::warn!("invalidate_object_ids must be consumed for each command");
            }
            // TODO: Err
            if let Ok(result) = session_state.document.handle_command(command) {
                for invalidated_object_id in result.invalidated_object_ids {
                    session_state
                        .invalidated_object_ids
                        .insert(invalidated_object_id);
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
        } else {
            log::warn!("System isn't in a session");
        }
    }

    pub fn consume_invalidated_object_ids(&mut self) -> String {
        if let SystemState::Joined(ref mut session_state) = self.state {
            log::trace!(
                "Objects being invalidated: {:?}",
                session_state.invalidated_object_ids
            );
            let result = serde_json::to_string(&session_state.invalidated_object_ids).unwrap();
            session_state.invalidated_object_ids.clear();
            result
        } else {
            log::warn!("System isn't in a session");
            "[]".into()
        }
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
        if let SystemState::Joined(ref session_state) = self.state {
            let document = session_state.document.materialize_document();
            return serde_json::to_string(&document).unwrap();
        } else {
            log::warn!("System isn't a session");
            "{}".into()
        }
    }
}
