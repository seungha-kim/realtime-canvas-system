use crate::document_storage::DocumentSnapshot;
use crate::{Color, ObjectId, ObjectKind, PropKind, PropValue};
use base95::Base95;
use euclid::default::Transform2D;
use std::collections::HashSet;
use std::str::FromStr;

pub trait PropReadable {
    fn get_prop(&self, object_id: &ObjectId, prop_kind: &PropKind) -> Option<&PropValue>;
    fn get_object_kind(&self, object_id: &ObjectId) -> Option<&ObjectKind>;
    fn is_deleted(&self, object_id: &ObjectId) -> Option<bool>;

    fn get_string_prop(&self, object_id: &ObjectId, prop_kind: &PropKind) -> Option<&str> {
        self.get_prop(object_id, prop_kind)
            .and_then(|prop_value| prop_value.as_string())
    }
    fn get_id_prop(&self, object_id: &ObjectId, prop_kind: &PropKind) -> Option<&ObjectId> {
        self.get_prop(object_id, prop_kind)
            .and_then(|prop_value| prop_value.as_reference())
    }
    fn get_float_prop(&self, object_id: &ObjectId, prop_kind: &PropKind) -> Option<&f32> {
        self.get_prop(object_id, prop_kind)
            .and_then(|prop_value| prop_value.as_float())
    }
    fn get_color_prop(&self, object_id: &ObjectId, prop_kind: &PropKind) -> Option<&Color> {
        self.get_prop(object_id, prop_kind)
            .and_then(|prop_value| prop_value.as_color())
    }

    // transform = from inner space point to outer space point..?
    fn get_global_transform(&self, object_id: &ObjectId) -> Transform2D<f32> {
        let mut result = Transform2D::identity();
        let mut current_object_id_opt = Some(object_id);
        loop {
            if let Some(current_object_id) = current_object_id_opt {
                let local_transform = self.get_local_transform(current_object_id);
                result = result.then(&local_transform);
                current_object_id_opt = self.get_id_prop(current_object_id, &PropKind::Parent);
            } else {
                break result;
            }
        }
    }

    fn get_local_transform(&self, object_id: &ObjectId) -> Transform2D<f32> {
        let pos_x = self
            .get_float_prop(object_id, &PropKind::PosX)
            .unwrap_or(&0.0);
        let pos_y = self
            .get_float_prop(object_id, &PropKind::PosY)
            .unwrap_or(&0.0);
        // TODO: scale, rotation, ...
        Transform2D::translation(*pos_x, *pos_y)
    }

    /// 저장소가 가지고 있는 ObjectId 들을 반환. 중복될 수 있음 - 추후 최적화 시 삭제 예정 (static dispatch)
    fn containing_objects(&self) -> Box<dyn Iterator<Item = &ObjectId> + '_>;

    fn get_children_indices(&self, target_parent_id: &ObjectId) -> Vec<(ObjectId, Base95)> {
        // TODO: optimize
        let ids = self
            .containing_objects()
            .filter(|object_id| !self.is_deleted(object_id).unwrap_or(true))
            .filter(|object_id| {
                self.get_id_prop(*object_id, &PropKind::Parent)
                    .map(|parent_id| parent_id == target_parent_id)
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<HashSet<_>>();

        let mut result = ids
            .iter()
            .map(|object_id| {
                let index = self
                    .get_string_prop(object_id, &PropKind::Index)
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
