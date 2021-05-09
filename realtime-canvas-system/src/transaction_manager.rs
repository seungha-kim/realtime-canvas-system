use super::message::*;
use super::traits::PropReadable;
use std::collections::HashMap;
use uuid::Uuid;

pub struct TransactionManager {
    txs: Vec<Transaction>,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self { txs: Vec::new() }
    }

    pub fn push(&mut self, tx: Transaction) {
        // ASSUME: CommandId 는 u16 으로 충분할 것
        debug_assert!(self
            .txs
            .iter()
            .find(|existing| existing.id == tx.id)
            .is_none());
        self.txs.push(tx);
    }

    pub fn remove(&mut self, tx_id: &TransactionId) -> Option<Transaction> {
        self.txs
            .iter()
            .position(|tx| tx.id == *tx_id)
            .map(|pos| self.txs.remove(pos))
    }
}

impl PropReadable for TransactionManager {
    fn get_string_prop(&self, target_key: &PropKey) -> Option<&str> {
        let mut result: Option<&str> = None;
        // TODO: 전부 탐색할 필요 없이, 끝에서부터 역순으로 탐색해서 처음으로 만족하는 요소 반환
        'outer: for tx in &self.txs {
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
        manager.push(Transaction {
            id: uuid::Uuid::new_v4(),
            items: vec![DocumentMutation::UpdateObject(
                PropKey(object_id, "hello".to_string()),
                PropValue::String("world".into()),
            )],
        });

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
        let tx_id = uuid::Uuid::new_v4();

        manager.push(Transaction {
            id: tx_id,
            items: vec![DocumentMutation::UpdateObject(
                PropKey(object_id, "hello".to_string()),
                PropValue::String("world".into()),
            )],
        });

        // same command id
        manager.push(Transaction {
            id: tx_id,
            items: vec![DocumentMutation::UpdateObject(
                PropKey(object_id, "hello".to_string()),
                PropValue::String("world".into()),
            )],
        });
    }
}
