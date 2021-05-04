use crate::message::DocumentMutation;
use crate::PropKey;

pub trait PropReadable {
    fn get_string_prop(&self, key: &PropKey) -> Option<&str>;
}

pub trait DocumentReadable {
    fn document_id(&self) -> uuid::Uuid;
}
