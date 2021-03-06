mod client_follower_document;
mod document;
pub mod document_command;
mod document_command_transaction;
pub mod materialize;
mod message;
mod server_leader_document;
mod traits;
mod transaction_manager;
mod transactional_document;

pub use client_follower_document::*;
pub use document::*;
pub use document_command::*;
pub use materialize::*;
pub use message::*;
pub use server_leader_document::*;
pub use traits::*;

pub extern crate bincode;
pub extern crate euclid;
pub extern crate serde;
pub extern crate serde_json;
pub extern crate uuid;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
