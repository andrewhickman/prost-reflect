use std::{
    borrow::Cow,
    collections::btree_map::{self, BTreeMap},
    fmt,
};

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

/// A set of extension fields in a protobuf message.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DynamicMessageFieldSet<T> {
    fields: BTreeMap<u32, DynamicMessageField<T>>,
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
            Some(field) => Cow::Borrowed(field.get()),
            None => Cow::Owned(desc.default_value()),
        }
    }

    pub(super) fn get_mut(&mut self, desc: &T) -> &mut Value {
        self.fields
            .entry(desc.number())
            .or_insert_with(|| DynamicMessageField::default(desc.clone()))
            .get_mut()
    }

    pub(super) fn set(&mut self, desc: &T, value: Value) {
        match self.fields.entry(desc.number()) {
            btree_map::Entry::Vacant(entry) => {
                entry.insert(DynamicMessageField::new(desc.clone(), value));
            }
            btree_map::Entry::Occupied(mut entry) => entry.get_mut().set(value),
        }
    }

    pub(super) fn clear(&mut self, desc: &T) {
        self.fields.remove(&desc.number());
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&T, &Value)> + '_ {
        self.fields
            .values()
            .filter(|v| v.has())
            .map(|field| (field.descriptor(), field.get()))
    }

    pub(super) fn clear_all(&mut self) {
        self.fields.clear();
    }
}

#[derive(Debug, Clone, PartialEq)]
struct DynamicMessageField<T> {
    desc: T,
    value: Value,
}

impl<T> DynamicMessageField<T>
where
    T: FieldDescriptorLike,
{
    fn default(desc: T) -> Self {
        DynamicMessageField {
            value: desc.default_value(),
            desc,
        }
    }

    fn new(desc: T, value: Value) -> Self {
        debug_assert!(
            desc.is_valid(&value),
            "invalid value {:?} for field {:?}",
            value,
            desc,
        );
        DynamicMessageField { value, desc }
    }

    fn has(&self) -> bool {
        self.desc.supports_presence() || !self.desc.is_default_value(&self.value)
    }

    fn get(&self) -> &Value {
        &self.value
    }

    fn get_mut(&mut self) -> &mut Value {
        &mut self.value
    }

    fn set(&mut self, value: Value) {
        debug_assert!(
            self.desc.is_valid(&value),
            "invalid value {:?} for field {:?}",
            value,
            self.desc,
        );
        self.value = value;
    }

    fn descriptor(&self) -> &T {
        &self.desc
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
