mod client_replica_document;
pub mod document_command;
mod document_storage;
pub mod materialize;
mod message;
mod server_leader_document;
mod traits;
mod transaction_manager;
mod transactional_storage;
mod types;

pub use client_replica_document::*;
pub use document_command::*;
pub use materialize::*;
pub use message::*;
pub use server_leader_document::*;
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
