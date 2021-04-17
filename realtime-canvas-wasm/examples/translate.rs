use realtime_canvas_system::{bincode, IdentifiableEvent, SessionState, SystemEvent};
use realtime_canvas_wasm::CanvasSystem;

fn main() {
    let event = IdentifiableEvent::BySystem {
        system_event: SystemEvent::JoinedSession {
            session_id: 1,
            initial_state: SessionState {
                connections: Vec::new(),
            },
        },
    };
    let serialized = bincode::serialize(&event).unwrap();
    let sys = CanvasSystem::new();
    let json = sys.convert_event_to_json(&serialized);
    println!("{}", json);
    let command2 = bincode::deserialize::<IdentifiableEvent>(&serialized).unwrap();
    println!("{:?}", event);
    println!("{:?}", command2);
}
