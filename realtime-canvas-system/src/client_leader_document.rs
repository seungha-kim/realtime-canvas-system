use std::collections::{HashSet, VecDeque};

use crate::materialize::Materialize;
use crate::transactional_storage::TransactionalStorage;

use super::message::*;
use super::types::*;
use crate::traits::PropReadable;
use crate::DocumentCommand;

pub struct ClientLeaderDocument {
    storage: TransactionalStorage,
}

impl Materialize<TransactionalStorage> for ClientLeaderDocument {
    fn readable(&self) -> &TransactionalStorage {
        &self.storage
    }
}

impl ClientLeaderDocument {
    pub fn new() -> Self {
        Self {
            storage: TransactionalStorage::new(),
        }
    }

    pub fn handle_command(&mut self, command: DocumentCommand) -> HashSet<ObjectId> {
        self.storage.handle_command(command)
    }
}
