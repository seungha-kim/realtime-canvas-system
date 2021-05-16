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
    UpdateOvalPosition {
        pos: Point2D<f32>,
    },
    UpdateOvalRadius {
        r_h: f32,
        r_v: f32,
    },

    // TODO
    Undo,
    Redo,
}
