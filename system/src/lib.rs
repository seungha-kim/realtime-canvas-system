use serde::*;
pub extern crate serde;
pub extern crate bincode;
pub extern crate serde_json;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Command {
    MoveTo {x: f32, y: f32},
    LineTo {x: f32, y: f32},
    Fragment {x1: f32, y1: f32, x2: f32, y2: f32},
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
