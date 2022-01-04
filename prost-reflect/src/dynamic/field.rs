use std::{borrow::Cow, fmt};

use crate::{ExtensionDescriptor, FieldDescriptor, Kind, OneofDescriptor, Value};

pub(super) trait FieldDescriptorLike: fmt::Debug {
    fn number(&self) -> u32;
    fn default_value(&self) -> Value;
    fn is_default_value(&self, value: &Value) -> bool;
    fn is_valid(&self, value: &Value) -> bool;
    fn containing_oneof(&self) -> Option<OneofDescriptor>;
    fn supports_presence(&self) -> bool;
    fn kind(&self) -> Kind;
    fn is_group(&self) -> bool;
    fn is_list(&self) -> bool;
    fn is_map(&self) -> bool;
    fn is_packed(&self) -> bool;
    fn is_packable(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct DynamicMessageField<T> {
    pub(super) desc: T,
    pub(super) value: Option<Value>,
}

impl<T> DynamicMessageField<T>
where
    T: FieldDescriptorLike,
{
    pub fn new(desc: T) -> Self {
        DynamicMessageField {
            value: if desc.supports_presence() {
                None
            } else {
                Some(desc.default_value())
            },
            desc,
        }
    }

    pub fn get(&self) -> Cow<'_, Value> {
        match &self.value {
            Some(value) => Cow::Borrowed(value),
            None => Cow::Owned(self.desc.default_value()),
        }
    }

    pub fn has(&self) -> bool {
        if self.desc.supports_presence() {
            self.value.is_some()
        } else {
            !self.desc.is_default_value(self.value.as_ref().unwrap())
        }
    }

    pub fn set(&mut self, value: Value) {
        debug_assert!(
            self.desc.is_valid(&value),
            "invalid value {:?} for field {:?}",
            value,
            self.desc,
        );
        self.value = Some(value);
    }

    pub fn clear(&mut self) {
        self.value = if self.desc.supports_presence() {
            None
        } else {
            Some(self.desc.default_value())
        };
    }
}

impl FieldDescriptorLike for FieldDescriptor {
    fn number(&self) -> u32 {
        self.number()
    }

    fn default_value(&self) -> Value {
        Value::default_value_for_field(self)
    }

    fn is_default_value(&self, value: &Value) -> bool {
        value.is_default_for_field(self)
    }

    fn is_valid(&self, value: &Value) -> bool {
        value.is_valid_for_field(self)
    }

    fn containing_oneof(&self) -> Option<OneofDescriptor> {
        self.containing_oneof()
    }

    fn supports_presence(&self) -> bool {
        self.supports_presence()
    }

    fn kind(&self) -> Kind {
        self.kind()
    }

    fn is_group(&self) -> bool {
        self.is_group()
    }

    fn is_list(&self) -> bool {
        self.is_list()
    }

    fn is_map(&self) -> bool {
        self.is_map()
    }

    fn is_packed(&self) -> bool {
        self.is_packed()
    }

    fn is_packable(&self) -> bool {
        self.is_packable()
    }
}

impl FieldDescriptorLike for ExtensionDescriptor {
    fn number(&self) -> u32 {
        self.number()
    }

    fn default_value(&self) -> Value {
        Value::default_value_for_extension(self)
    }

    fn is_default_value(&self, value: &Value) -> bool {
        value.is_default_for_extension(self)
    }

    fn is_valid(&self, value: &Value) -> bool {
        value.is_valid_for_extension(self)
    }

    fn containing_oneof(&self) -> Option<OneofDescriptor> {
        None
    }

    fn supports_presence(&self) -> bool {
        self.supports_presence()
    }

    fn kind(&self) -> Kind {
        self.kind()
    }

    fn is_group(&self) -> bool {
        self.is_group()
    }

    fn is_list(&self) -> bool {
        self.is_list()
    }

    fn is_map(&self) -> bool {
        self.is_map()
    }

    fn is_packed(&self) -> bool {
        self.is_packed()
    }

    fn is_packable(&self) -> bool {
        self.is_packable()
    }
}
