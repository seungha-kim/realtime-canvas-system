use euclid::default::Point2D;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum DocumentCommand {
    UpdateDocumentTitle { title: String },
    CreateCircle { pos: Point2D<f32>, radius: f32 },
    UpdateCirclePosition { pos: Point2D<f32> },
    UpdateCircleRadius { radius: f32 },

    // TODO
    Undo,
    Redo,
}
