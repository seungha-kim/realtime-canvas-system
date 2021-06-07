use crate::Color;
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
        fill_color: Color,
    },
    CreateFrame {
        pos: Point2D<f32>,
        w: f32,
        h: f32,
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
    DeleteObject {
        id: uuid::Uuid,
    },
    UpdateIndex {
        id: uuid::Uuid,
        int_index: usize,
    },
    // TODO
    Undo,
    Redo,
}
