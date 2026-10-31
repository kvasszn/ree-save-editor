use crate::save::types::{FieldValue};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum CopyBuffer {
    #[default]
    Null,
    FieldValue(FieldValue), // This only appears on classes and structs i think?
}

impl CopyBuffer {
    pub fn paste_into_class(&self, target: &FieldValue) -> Option<FieldValue> {
        let Self::FieldValue(buffered) = self else { return None };
        buffered.is_class_same(target).then(|| buffered.clone())
    }

    pub fn paste_into_array(&self, target: &FieldValue) -> Option<FieldValue> {
        let Self::FieldValue(buffered) = self else { return None };
        let buffered_arr = buffered.as_array()?;
        let target_arr = target.as_array()?;

        let compatible = buffered_arr.member_type == target_arr.member_type
            && buffered_arr.member_size == target_arr.member_size
            && buffered_arr.array_type == target_arr.array_type;

        compatible.then(|| buffered.clone())
    }

    pub fn paste_into(&self, target: &FieldValue) -> Option<FieldValue> {
        self.paste_into_class(target)
            .or_else(|| self.paste_into_array(target))
    }
}
