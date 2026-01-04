use crate::save::types::{Class, Field};
#[derive(Debug, Clone)]
pub enum CopyBuffer {
    Null,
    Array(Class),
    Field(Field),
}
