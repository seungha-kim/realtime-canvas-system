mod client_leader_document;
pub mod document_command;
mod document_storage;
pub mod materialize;
mod message;
mod traits;
mod transaction_manager;
mod transactional_storage;
mod types;

pub use client_leader_document::*;
pub use message::*;
pub use types::*;

pub extern crate bincode;
pub extern crate euclid;
pub extern crate serde;
pub extern crate serde_json;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
