use std::{borrow::Cow, collections::BTreeMap, fmt};

use crate::{
    ExtensionDescriptor, FieldDescriptor, Kind, MessageDescriptor, OneofDescriptor, Value,
};

pub(super) trait FieldDescriptorLike: fmt::Debug + Clone {
    fn get(message: &MessageDescriptor, number: u32) -> Option<Self>;
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

    fn has(&self, value: &Value) -> bool {
        self.supports_presence() || !self.is_default_value(value)
    }
}

/// A set of extension fields in a protobuf message.
#[derive(Default, Debug, Clone, PartialEq)]
pub(super) struct DynamicMessageFieldSet {
    fields: BTreeMap<u32, Value>,
}

impl DynamicMessageFieldSet {
    pub(super) fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub(super) fn has(&self, desc: &impl FieldDescriptorLike) -> bool {
        self.fields
            .get(&desc.number())
            .map(|value| desc.has(value))
            .unwrap_or(false)
    }

    pub(super) fn get(&self, desc: &impl FieldDescriptorLike) -> Cow<'_, Value> {
        match self.fields.get(&desc.number()) {
            Some(value) => Cow::Borrowed(value),
            None => Cow::Owned(desc.default_value()),
        }
    }

    pub(super) fn get_mut(&mut self, desc: &impl FieldDescriptorLike) -> &mut Value {
        self.clear_oneof_fields(desc);
        self.fields
            .entry(desc.number())
            .or_insert_with(|| desc.default_value())
    }

    pub(super) fn set(&mut self, desc: &impl FieldDescriptorLike, value: Value) {
        debug_assert!(
            desc.is_valid(&value),
            "invalid value {:?} for field {:?}",
            value,
            desc,
        );

        self.clear_oneof_fields(desc);
        self.fields.insert(desc.number(), value);
    }

    fn clear_oneof_fields(&mut self, desc: &impl FieldDescriptorLike) {
        if let Some(oneof_desc) = desc.containing_oneof() {
            for oneof_field in oneof_desc.fields() {
                if oneof_field.number() != desc.number() {
                    self.clear(&oneof_field);
                }
            }
        }
    }

    pub(super) fn clear(&mut self, desc: &impl FieldDescriptorLike) {
        self.fields.remove(&desc.number());
    }

    pub(super) fn iter_fields<'a>(
        &'a self,
        message: &'a MessageDescriptor,
    ) -> impl Iterator<Item = (FieldDescriptor, &Value)> + 'a {
        self.iter::<FieldDescriptor>(message)
    }

    pub(super) fn iter_extensions<'a>(
        &'a self,
        message: &'a MessageDescriptor,
    ) -> impl Iterator<Item = (ExtensionDescriptor, &Value)> + 'a {
        self.iter::<ExtensionDescriptor>(message)
    }

    fn iter<'a, T: FieldDescriptorLike>(
        &'a self,
        message: &'a MessageDescriptor,
    ) -> impl Iterator<Item = (T, &Value)> + 'a {
        self.fields.iter().filter_map(move |(&number, value)| {
            let desc = T::get(message, number)?;
            if desc.has(value) {
                Some((desc, value))
            } else {
                None
            }
        })
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

impl FieldDescriptorLike for FieldDescriptor {
    fn get(message: &MessageDescriptor, number: u32) -> Option<Self> {
        message.get_field(number)
    }

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
    fn get(message: &MessageDescriptor, number: u32) -> Option<Self> {
        message.get_extension(number)
    }

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
