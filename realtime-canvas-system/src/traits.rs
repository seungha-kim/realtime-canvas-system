use crate::message::DocumentMutation;
use crate::PropKey;

pub trait ReadableStorage {
    fn get_string_prop(&self, key: &PropKey) -> Option<&str>;
}
