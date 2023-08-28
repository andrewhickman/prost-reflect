use std::{
    borrow::Cow,
    collections::btree_map::{self, BTreeMap},
    fmt,
};

use crate::{
    Cardinality, ExtensionDescriptor, FieldDescriptor, Kind, MessageDescriptor, OneofDescriptor,
    Value,
};

use super::unknown::{UnknownField, UnknownFieldSet};

pub(crate) trait FieldDescriptorLike: fmt::Debug {
    fn text_name(&self) -> &str;
    fn number(&self) -> u32;
    fn default_value(&self) -> Value;
    fn is_default_value(&self, value: &Value) -> bool;
    fn is_valid(&self, value: &Value) -> bool;
    fn containing_oneof(&self) -> Option<OneofDescriptor>;
    fn supports_presence(&self) -> bool;
    fn kind(&self) -> Kind;
    fn is_group(&self) -> bool;
    fn cardinality(&self) -> Cardinality;
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
    fields: BTreeMap<u32, ValueOrUnknown>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ValueOrUnknown {
    Value(Value),
    Unknown(UnknownFieldSet),
}

pub(super) enum ValueAndDescriptor<'a> {
    Field(Cow<'a, Value>, FieldDescriptor),
    Extension(Cow<'a, Value>, ExtensionDescriptor),
    Unknown(u32, &'a UnknownFieldSet),
}

impl DynamicMessageFieldSet {
    fn get_value(&self, number: u32) -> Option<&Value> {
        match self.fields.get(&number) {
            Some(ValueOrUnknown::Value(value)) => Some(value),
            Some(ValueOrUnknown::Unknown(_)) | None => None,
        }
    }

    pub(super) fn has(&self, desc: &impl FieldDescriptorLike) -> bool {
        self.get_value(desc.number())
            .map(|value| desc.has(value))
            .unwrap_or(false)
    }

    pub(super) fn get(&self, desc: &impl FieldDescriptorLike) -> Cow<'_, Value> {
        match self.get_value(desc.number()) {
            Some(value) => Cow::Borrowed(value),
            None => Cow::Owned(desc.default_value()),
        }
    }

    pub(super) fn get_mut(&mut self, desc: &impl FieldDescriptorLike) -> &mut Value {
        self.clear_oneof_fields(desc);
        match self.fields.entry(desc.number()) {
            btree_map::Entry::Occupied(entry) => match entry.into_mut() {
                ValueOrUnknown::Value(value) => value,
                value @ ValueOrUnknown::Unknown(_) => {
                    *value = ValueOrUnknown::Value(desc.default_value());
                    value.unwrap_value_mut()
                }
            },
            btree_map::Entry::Vacant(entry) => entry
                .insert(ValueOrUnknown::Value(desc.default_value()))
                .unwrap_value_mut(),
        }
    }

    pub(super) fn set(&mut self, desc: &impl FieldDescriptorLike, value: Value) {
        debug_assert!(
            desc.is_valid(&value),
            "invalid value {:?} for field {:?}",
            value,
            desc,
        );

        self.clear_oneof_fields(desc);
        self.fields
            .insert(desc.number(), ValueOrUnknown::Value(value));
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

    pub(crate) fn add_unknown(&mut self, number: u32, unknown: UnknownField) {
        match self.fields.entry(number) {
            btree_map::Entry::Occupied(mut entry) => match entry.get_mut() {
                ValueOrUnknown::Value(_) => {
                    panic!("expected no field to be found with number {}", number)
                }
                ValueOrUnknown::Unknown(unknowns) => unknowns.insert(unknown),
            },
            btree_map::Entry::Vacant(entry) => {
                entry.insert(ValueOrUnknown::Unknown(UnknownFieldSet::from_iter([
                    unknown,
                ])));
            }
        }
    }

    pub(super) fn clear(&mut self, desc: &impl FieldDescriptorLike) {
        self.fields.remove(&desc.number());
    }

    pub(crate) fn take(&mut self, desc: &impl FieldDescriptorLike) -> Option<Value> {
        match self.fields.remove(&desc.number()) {
            Some(ValueOrUnknown::Value(value)) if desc.has(&value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn iter<'a>(
        &'a self,
        message: &'a MessageDescriptor,
    ) -> impl Iterator<Item = ValueAndDescriptor> + 'a {
        self.fields
            .iter()
            .filter_map(move |(&number, value)| match value {
                ValueOrUnknown::Value(value) => {
                    if let Some(field) = message.get_field(number) {
                        if field.has(value) {
                            Some(ValueAndDescriptor::Field(Cow::Borrowed(value), field))
                        } else {
                            None
                        }
                    } else if let Some(extension) = message.get_extension(number) {
                        if extension.has(value) {
                            Some(ValueAndDescriptor::Extension(
                                Cow::Borrowed(value),
                                extension,
                            ))
                        } else {
                            None
                        }
                    } else {
                        panic!("no field found with number {}", number)
                    }
                }
                ValueOrUnknown::Unknown(unknown) => {
                    Some(ValueAndDescriptor::Unknown(number, unknown))
                }
            })
    }

    #[cfg(feature = "serde")]
    pub(crate) fn iter_include_default<'a>(
        &'a self,
        message: &'a MessageDescriptor,
    ) -> impl Iterator<Item = ValueAndDescriptor> + 'a {
        let fields = message
            .fields()
            .filter(move |f| !f.supports_presence() || self.has(f))
            .map(move |f| ValueAndDescriptor::Field(self.get(&f), f));
        let others = self
            .fields
            .iter()
            .filter_map(move |(&number, value)| match value {
                ValueOrUnknown::Value(value) => {
                    if let Some(extension) = message.get_extension(number) {
                        if extension.has(value) {
                            Some(ValueAndDescriptor::Extension(
                                Cow::Borrowed(value),
                                extension,
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                ValueOrUnknown::Unknown(unknown) => {
                    Some(ValueAndDescriptor::Unknown(number, unknown))
                }
            });
        fields.chain(others)
    }

    pub(crate) fn iter_fields<'a>(
        &'a self,
        message: &'a MessageDescriptor,
    ) -> impl Iterator<Item = (FieldDescriptor, &'a Value)> + 'a {
        self.fields.iter().filter_map(move |(&number, value)| {
            let value = match value {
                ValueOrUnknown::Value(value) => value,
                _ => return None,
            };
            let field = match message.get_field(number) {
                Some(field) => field,
                _ => return None,
            };
            if field.has(value) {
                Some((field, value))
            } else {
                None
            }
        })
    }

    pub(crate) fn iter_extensions<'a>(
        &'a self,
        message: &'a MessageDescriptor,
    ) -> impl Iterator<Item = (ExtensionDescriptor, &'a Value)> + 'a {
        self.fields.iter().filter_map(move |(&number, value)| {
            let value = match value {
                ValueOrUnknown::Value(value) => value,
                _ => return None,
            };
            let field = match message.get_extension(number) {
                Some(field) => field,
                _ => return None,
            };
            if field.has(value) {
                Some((field, value))
            } else {
                None
            }
        })
    }

    pub(super) fn iter_unknown(&self) -> impl Iterator<Item = &'_ UnknownField> {
        self.fields.values().flat_map(move |value| match value {
            ValueOrUnknown::Value(_) => [].iter(),
            ValueOrUnknown::Unknown(unknowns) => unknowns.iter(),
        })
    }

    pub(super) fn clear_all(&mut self) {
        self.fields.clear();
    }
}

impl ValueOrUnknown {
    fn unwrap_value_mut(&mut self) -> &mut Value {
        match self {
            ValueOrUnknown::Value(value) => value,
            ValueOrUnknown::Unknown(_) => unreachable!(),
        }
    }
}

impl FieldDescriptorLike for FieldDescriptor {
    fn text_name(&self) -> &str {
        self.name()
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

    fn cardinality(&self) -> Cardinality {
        self.cardinality()
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
    fn text_name(&self) -> &str {
        self.json_name()
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

    fn cardinality(&self) -> Cardinality {
        self.cardinality()
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
