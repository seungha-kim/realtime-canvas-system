use super::message::*;
use super::traits::PropReadable;

#[derive(Debug)]
pub struct TransactionManager {
    txs: Vec<Transaction>,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self { txs: Vec::new() }
    }

    pub fn get(&self, tx_id: &TransactionId) -> Option<&Transaction> {
        self.txs.iter().find(|tx| &tx.id == tx_id)
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

    fn last_mutation<'a, T, F>(&'a self, matcher: F) -> Option<T>
    where
        F: Fn(&'a DocumentMutation) -> Option<T>,
    {
        for tx in self.txs.iter().rev() {
            for command in tx.items.iter().rev() {
                if let Some(matched) = matcher(command) {
                    return Some(matched);
                }
            }
        }
        None
    }
}

impl PropReadable for TransactionManager {
    fn get_prop(&self, object_id: &ObjectId, prop_kind: &PropKind) -> Option<&PropValue> {
        self.last_mutation(|command| match command {
            DocumentMutation::UpsertProp(can_object_id, can_prop_kind, prop_value)
                if can_object_id == object_id && can_prop_kind == prop_kind =>
            {
                prop_value.as_ref()
            }
            _ => None,
        })
    }

    fn get_object_kind(&self, target_object_id: &ObjectId) -> Option<&ObjectKind> {
        self.last_mutation(|command| match command {
            DocumentMutation::CreateObject(object_id, object_kind)
                if object_id == target_object_id =>
            {
                Some(object_kind)
            }
            _ => None,
        })
    }

    fn is_deleted(&self, object_id: &ObjectId) -> Option<bool> {
        for tx in &self.txs {
            for mutation in &tx.items {
                match mutation {
                    DocumentMutation::DeleteObject(candidate) if candidate == object_id => {
                        return Some(true)
                    }
                    _ => {}
                }
            }
        }
        None
    }

    fn get_all_props_of_object(&self, object_id: &ObjectId) -> Vec<(PropKind, Option<PropValue>)> {
        let mut result = Vec::new();
        for tx in &self.txs {
            for mutation in &tx.items {
                match mutation {
                    DocumentMutation::UpsertProp(can_object_id, prop_kind, prop_value)
                        if can_object_id == object_id =>
                    {
                        if let Some((_, pv)) = result.iter_mut().find(|(pk, _)| pk == prop_kind) {
                            *pv = prop_value.clone();
                        }
                    }
                    _ => {}
                }
            }
        }
        result
    }

    fn containing_objects(&self) -> Box<dyn Iterator<Item = &ObjectId> + '_> {
        Box::new(
            self.txs
                .iter()
                .flat_map(|tx| tx.items.iter())
                .filter_map(|item| match item {
                    DocumentMutation::UpsertProp(object_id, ..) => Some(object_id),
                    _ => None,
                }),
        )
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
            items: vec![DocumentMutation::UpsertProp(
                object_id,
                PropKind::Name,
                Some(PropValue::String("world".into())),
            )],
        });

        assert_eq!(
            manager.get_string_prop(&object_id, &PropKind::Name),
            Some("world")
        );

        assert_eq!(
            manager.get_string_prop(&object_id, &PropKind::RadiusH),
            None
        );

        let other_id = uuid::Uuid::new_v4();
        assert_eq!(manager.get_string_prop(&other_id, &PropKind::Name), None);
    }

    #[test]
    #[should_panic]
    fn should_panic_when_pushing_same_command_id() {
        let mut manager = TransactionManager::new();

        let object_id = uuid::Uuid::new_v4();
        let tx_id = uuid::Uuid::new_v4();

        manager.push(Transaction {
            id: tx_id,
            items: vec![DocumentMutation::UpsertProp(
                object_id,
                PropKind::Name,
                Some(PropValue::String("world".into())),
            )],
        });

        // same command id
        manager.push(Transaction {
            id: tx_id,
            items: vec![DocumentMutation::UpsertProp(
                object_id,
                PropKind::Name,
                Some(PropValue::String("world".into())),
            )],
        });
    }
}
