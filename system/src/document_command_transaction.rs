use crate::euclid::default::Transform2D;
use crate::{
    Color, DocumentCommand, DocumentMutation, DocumentReadable, ObjectId, ObjectKind, PropKey,
    PropKind, PropReadable, PropValue, Transaction,
};
use base95::Base95;
use std::str::FromStr;

pub fn convert_command_to_tx<R: PropReadable + DocumentReadable>(
    readable: &R,
    command: DocumentCommand,
) -> Result<Transaction, ()> {
    match command {
        DocumentCommand::UpdateDocumentName { name } => {
            Ok(Transaction::new(vec![DocumentMutation::UpsertProp(
                PropKey(readable.document_id(), PropKind::Name),
                Some(PropValue::String(name)),
            )]))
        }
        DocumentCommand::CreateOval {
            pos,
            r_h,
            r_v,
            fill_color,
        } => {
            let id = uuid::Uuid::new_v4();
            // TODO: parent_id 입력 받기
            let parent_id = readable.document_id();
            let index = create_last_index_of_parent(readable, &parent_id);

            Ok(Transaction::new(vec![
                DocumentMutation::CreateObject(id, ObjectKind::Oval),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::Parent),
                    Some(PropValue::Reference(parent_id)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::Index),
                    Some(PropValue::String(index.to_string())),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::PosX),
                    Some(PropValue::Float(pos.x)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::PosY),
                    Some(PropValue::Float(pos.y)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::RadiusH),
                    Some(PropValue::Float(r_h)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::RadiusV),
                    Some(PropValue::Float(r_v)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::FillColor),
                    Some(PropValue::Color(fill_color)),
                ),
            ]))
        }
        DocumentCommand::CreateFrame { pos, h, w } => {
            let id = uuid::Uuid::new_v4();
            // TODO: parent_id 입력 받기
            let parent_id = readable.document_id();
            let index = create_last_index_of_parent(readable, &parent_id);

            // FIXME: 테스트용 oval
            let oval_id = uuid::Uuid::new_v4();

            Ok(Transaction::new(vec![
                DocumentMutation::CreateObject(id, ObjectKind::Frame),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::Parent),
                    Some(PropValue::Reference(parent_id)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::Index),
                    Some(PropValue::String(index.to_string())),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::PosX),
                    Some(PropValue::Float(pos.x)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::PosY),
                    Some(PropValue::Float(pos.y)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::Width),
                    Some(PropValue::Float(w)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::Height),
                    Some(PropValue::Float(h)),
                ),
                // FIXME: 테스트용 Oval
                DocumentMutation::CreateObject(oval_id, ObjectKind::Oval),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::Parent),
                    Some(PropValue::Reference(id)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::Index),
                    Some(PropValue::String(Base95::mid().to_string())),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::PosX),
                    Some(PropValue::Float(0.0)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::PosY),
                    Some(PropValue::Float(0.0)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::RadiusH),
                    Some(PropValue::Float(30.0)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::RadiusV),
                    Some(PropValue::Float(30.0)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::FillColor),
                    Some(PropValue::Color(Color {
                        r: 50,
                        g: 50,
                        b: 50,
                    })),
                ),
            ]))
        }
        DocumentCommand::UpdateName { id, name } => {
            Ok(Transaction::new(vec![DocumentMutation::UpsertProp(
                PropKey(id, PropKind::Name),
                Some(PropValue::String(name)),
            )]))
        }
        DocumentCommand::UpdatePosition { id, pos } => Ok(Transaction::new(vec![
            DocumentMutation::UpsertProp(
                PropKey(id, PropKind::PosX),
                Some(PropValue::Float(pos.x)),
            ),
            DocumentMutation::UpsertProp(
                PropKey(id, PropKind::PosY),
                Some(PropValue::Float(pos.y)),
            ),
        ])),
        DocumentCommand::DeleteObject { id } => {
            Ok(Transaction::new(vec![DocumentMutation::DeleteObject(id)]))
        }
        DocumentCommand::UpdateIndex { id, int_index } => {
            let new_index_str = readable
                .get_id_prop(&PropKey(id, PropKind::Parent))
                .ok_or(())
                .and_then(|parent_id| {
                    let indices = readable.get_children_indices(&parent_id);
                    if int_index > indices.len() {
                        Err(())
                    } else if int_index == 0 {
                        Ok(Base95::avg_with_zero(&indices[0].1))
                    } else if int_index == indices.len() {
                        Ok(Base95::avg_with_one(&indices[indices.len() - 1].1))
                    } else {
                        Ok(Base95::avg(
                            &indices[int_index - 1].1,
                            &indices[int_index].1,
                        ))
                    }
                })
                .map(|new_index| new_index.to_string())?;

            Ok(Transaction::new(vec![DocumentMutation::UpsertProp(
                PropKey(id, PropKind::Index),
                Some(PropValue::String(new_index_str)),
            )]))
        }
        DocumentCommand::UpdateParent { id, parent_id } => {
            let index = create_last_index_of_parent(readable, &parent_id);

            let current_global_transform = readable.get_global_transform(&id);
            let target_parent_global_transform = readable.get_global_transform(&parent_id);
            let new_local_transform = current_global_transform.then(
                &target_parent_global_transform
                    .inverse()
                    .unwrap_or(Transform2D::identity()),
            );

            Ok(Transaction::new(vec![
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::Parent),
                    Some(PropValue::Reference(parent_id)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::Index),
                    Some(PropValue::String(index.to_string())),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::PosX),
                    Some(PropValue::Float(new_local_transform.m31)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::PosY),
                    Some(PropValue::Float(new_local_transform.m32)),
                ),
            ]))
        }
        _ => unimplemented!(),
    }
}

fn create_last_index_of_parent<R: PropReadable + DocumentReadable>(
    readable: &R,
    parent_id: &ObjectId,
) -> Base95 {
    let children = readable.get_children_indices(&parent_id);

    children
        .last()
        .and_then(|(last_child_id, _)| {
            readable.get_string_prop(&PropKey(last_child_id.clone(), PropKind::Index))
        })
        // TODO: Base95::from_str 실패하는 경우에 대한 처리
        .and_then(|last_index_str| Base95::from_str(last_index_str).ok())
        .map(|last_index| Base95::avg_with_one(&last_index))
        .unwrap_or(Base95::mid())
}
