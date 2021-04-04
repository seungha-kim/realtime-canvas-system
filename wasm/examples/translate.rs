use system::{bincode, Command};
use realtime_canvas::CanvasSystem;

fn main() {
    let command = Command::LineTo { x: 10.0, y: 10.0 };
    let serialized = bincode::serialize(&command).unwrap();
    let sys = CanvasSystem::new();
    let json = sys.translate_to_json(&serialized);
    println!("{}", json);
    let bytes = sys.translate_from_json(json);
    let command2 = bincode::deserialize::<Command>(&bytes).unwrap();
    println!("{:?}", command);
    println!("{:?}", command2);
}
