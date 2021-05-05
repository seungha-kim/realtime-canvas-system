use realtime_canvas_system::{ConnectionId, ServerLeaderDocument};

pub struct Session {
    connections: Vec<ConnectionId>,
    document: ServerLeaderDocument,
}

// TODO
