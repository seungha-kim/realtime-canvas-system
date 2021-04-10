mod message;
mod types;

pub use message::*;
pub use types::*;

use serde::*;
pub extern crate bincode;
pub extern crate serde;
pub extern crate serde_json;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
