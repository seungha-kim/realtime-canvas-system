use super::message::*;
use super::traits::ReadableStorage;
use super::types::*;
use std::collections::HashMap;

pub struct TransactionManager {
    txs: HashMap<CommandId, Transaction>,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self {
            txs: HashMap::new(),
        }
    }

    pub fn push(&mut self, command_id: CommandId, tx: Transaction) {
        // ASSUME: CommandId 는 u16 으로 충분할 것
        assert!(!self.txs.contains_key(&command_id));
        self.txs.insert(command_id, tx);
    }

    pub fn pop(&mut self, command_id: CommandId) -> Option<Transaction> {
        self.txs.remove(&command_id)
    }
}

impl ReadableStorage for TransactionManager {
    fn get_string_prop(&self, target_key: &PropKey) -> Option<&str> {
        let mut result: Option<&str> = None;
        'outer: for (_, tx) in &self.txs {
            for command in &tx.items {
                if let DocumentMutation::UpdateObject(prop_key, prop_value) = command {
                    if target_key == prop_key {
                        if let PropValue::String(s) = prop_value {
                            result = Some(s.as_str());
                        }
                        continue 'outer;
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut manager = TransactionManager::new();

        let object_id = uuid::Uuid::new_v4();

        // NOTE: 트랜잭션은 객체 만들어지기 전에도 UpdateObject 할 수 있다고 가정
        manager.push(
            1,
            Transaction {
                items: vec![DocumentMutation::UpdateObject(
                    PropKey(object_id, "hello".to_string()),
                    PropValue::String("world".into()),
                )],
            },
        );

        assert_eq!(
            manager.get_string_prop(&PropKey(object_id, "hello".to_string())),
            Some("world")
        );

        assert_eq!(
            manager.get_string_prop(&PropKey(object_id, "asdf".into())),
            None
        );

        let other_id = uuid::Uuid::new_v4();
        assert_eq!(
            manager.get_string_prop(&PropKey(other_id, "hello".into())),
            None
        );
    }

    #[test]
    #[should_panic]
    fn should_panic_when_pushing_same_command_id() {
        let mut manager = TransactionManager::new();

        let object_id = uuid::Uuid::new_v4();

        manager.push(
            1,
            Transaction {
                items: vec![DocumentMutation::UpdateObject(
                    PropKey(object_id, "hello".to_string()),
                    PropValue::String("world".into()),
                )],
            },
        );

        // same command id
        manager.push(
            1,
            Transaction {
                items: vec![DocumentMutation::UpdateObject(
                    PropKey(object_id, "hello".to_string()),
                    PropValue::String("world".into()),
                )],
            },
        );
    }
}
