use std::{borrow::Cow, collections::BTreeMap, fmt};

use crate::{ExtensionDescriptor, FieldDescriptor, Kind, OneofDescriptor, Value};

pub(super) trait FieldDescriptorLike: fmt::Debug + Clone {
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
}

/// A set of extension fields in a protobuf message.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DynamicMessageFieldSet<T> {
    pub(super) fields: BTreeMap<u32, DynamicMessageField<T>>,
}

impl<T> Default for DynamicMessageFieldSet<T> {
    fn default() -> Self {
        DynamicMessageFieldSet {
            fields: Default::default(),
        }
    }
}

impl<T> DynamicMessageFieldSet<T>
where
    T: FieldDescriptorLike,
{
    pub(super) fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub(super) fn has(&self, desc: &T) -> bool {
        self.fields
            .get(&desc.number())
            .map(|field| field.has())
            .unwrap_or(false)
    }

    pub(super) fn get(&self, desc: &T) -> Cow<'_, Value> {
        match self.fields.get(&desc.number()) {
            Some(field) => field.get(),
            None => Cow::Owned(desc.default_value()),
        }
    }

    pub(super) fn get_mut(&mut self, desc: &T) -> &mut DynamicMessageField<T> {
        self.fields
            .entry(desc.number())
            .or_insert_with(|| DynamicMessageField::new(desc.clone()))
    }

    pub(super) fn set(&mut self, desc: &T, value: crate::Value) {
        self.get_mut(desc).set(value);
    }

    pub(super) fn clear(&mut self, desc: &T) {
        self.fields.remove(&desc.number());
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
