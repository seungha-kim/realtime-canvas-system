use euclid::default::Point2D;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum DocumentCommand {
    UpdateDocumentName {
        name: String,
    },
    CreateOval {
        pos: Point2D<f32>,
        r_h: f32,
        r_v: f32,
    },
    UpdatePosition {
        id: uuid::Uuid,
        pos: Point2D<f32>,
    },
    UpdateOvalRadius {
        r_h: f32,
        r_v: f32,
    },
    UpdateName {
        id: uuid::Uuid,
        name: String,
    },
    // TODO
    Undo,
    Redo,
}
