use crate::document_storage::DocumentSnapshot;
use crate::{ObjectId, ObjectKind, PropKey, PropKind};
use std::collections::HashSet;

pub trait PropReadable {
    fn get_string_prop(&self, key: &PropKey) -> Option<&str>;
    fn get_id_prop(&self, key: &PropKey) -> Option<&ObjectId>;
    fn get_float_prop(&self, key: &PropKey) -> Option<&f32>;

    fn get_object_kind(&self, object_id: &ObjectId) -> Option<&ObjectKind>;

    /// 저장소가 가지고 있는 ObjectId 들을 반환. 중복될 수 있음 - 추후 최적화 시 삭제 예정 (static dispatch)
    fn containing_objects(&self) -> Box<dyn Iterator<Item = &ObjectId> + '_>;

    fn get_children(&self, target_parent_id: &ObjectId) -> HashSet<ObjectId> {
        // TODO: optimize, order
        self.containing_objects()
            .filter(|object_id| {
                self.get_id_prop(&PropKey(**object_id, PropKind::Parent))
                    .map(|parent_id| parent_id == target_parent_id)
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }
}

pub trait DocumentReadable {
    fn document_id(&self) -> uuid::Uuid;

    fn snapshot(&self) -> DocumentSnapshot;
}
