use crate::document_storage::DocumentSnapshot;
use crate::{ObjectId, ObjectKind, PropKey, PropKind};
use base95::Base95;
use std::collections::HashSet;
use std::str::FromStr;

pub trait PropReadable {
    fn get_string_prop(&self, key: &PropKey) -> Option<&str>;
    fn get_id_prop(&self, key: &PropKey) -> Option<&ObjectId>;
    fn get_float_prop(&self, key: &PropKey) -> Option<&f32>;

    fn get_object_kind(&self, object_id: &ObjectId) -> Option<&ObjectKind>;
    fn is_deleted(&self, object_id: &ObjectId) -> Option<bool>;

    /// 저장소가 가지고 있는 ObjectId 들을 반환. 중복될 수 있음 - 추후 최적화 시 삭제 예정 (static dispatch)
    fn containing_objects(&self) -> Box<dyn Iterator<Item = &ObjectId> + '_>;

    fn get_children_indices(&self, target_parent_id: &ObjectId) -> Vec<(ObjectId, Base95)> {
        // TODO: optimize
        let ids = self
            .containing_objects()
            .filter(|object_id| !self.is_deleted(object_id).unwrap_or(true))
            .filter(|object_id| {
                self.get_id_prop(&PropKey(**object_id, PropKind::Parent))
                    .map(|parent_id| parent_id == target_parent_id)
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<HashSet<_>>();

        let mut result = ids
            .iter()
            .map(|object_id| {
                let index = self
                    .get_string_prop(&PropKey(object_id.clone(), PropKind::Index))
                    .and_then(|index_str| Base95::from_str(index_str).ok())
                    .unwrap_or(Base95::mid());
                (object_id.clone(), index)
            })
            .collect::<Vec<_>>();
        result.sort_by(|(_, index1), (_, index2)| index1.cmp(index2));

        result
    }
}

pub trait DocumentReadable {
    fn document_id(&self) -> uuid::Uuid;

    fn snapshot(&self) -> DocumentSnapshot;
}
