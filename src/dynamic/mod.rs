mod message;

use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

use prost::bytes::Bytes;

use crate::{descriptor::FieldDescriptorKind, FieldDescriptor, MessageDescriptor};

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicMessage {
    desc: MessageDescriptor,
    fields: BTreeMap<u32, DynamicMessageField>,
}

#[derive(Debug, Clone, PartialEq)]
struct DynamicMessageField {
    desc: FieldDescriptor,
    value: Option<Value>,
}

/// A dynamically-typed protobuf value.
///
/// Note this type may map to multiple possible protobuf wire formats, so it must be
/// serialized as part of a DynamicMessage.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Bytes),
    EnumNumber(i32),
    Message(DynamicMessage),
    List(Vec<Value>),
    Map(HashMap<MapKey, Value>),
}

/// A dynamically-typed key for a protobuf map.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MapKey {
    Bool(bool),
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    String(String),
}

impl DynamicMessage {
    pub fn new(desc: MessageDescriptor) -> Self {
        DynamicMessage {
            fields: desc
                .fields()
                .map(|field_desc| (field_desc.tag(), DynamicMessageField::new(field_desc)))
                .collect(),
            desc,
        }
    }

    pub fn descriptor(&self) -> MessageDescriptor {
        self.desc.clone()
    }

    pub fn has_field(&self, tag: u32) -> bool {
        self.fields
            .get(&tag)
            .map_or(false, |field| field.is_populated())
    }

    pub fn get_field(&self, tag: u32) -> Option<Cow<'_, Value>> {
        self.fields.get(&tag).map(|field| field.get())
    }

    pub fn set_field(&mut self, tag: u32, value: Value) {
        if let Some(field) = self.fields.get_mut(&tag) {
            field.set(value);

            if let Some(oneof) = field.desc.containing_oneof() {
                for oneof_field in oneof.fields() {
                    if oneof_field.tag() != tag {
                        self.clear_field(oneof_field.tag());
                    }
                }
            }
        }
    }

    pub fn clear_field(&mut self, tag: u32) {
        if let Some(field) = self.fields.get_mut(&tag) {
            field.clear();
        }
    }

    pub fn has_field_by_name(&self, name: &str) -> bool {
        self.desc
            .get_field_by_name(name)
            .map_or(false, |field_desc| self.has_field(field_desc.tag()))
    }

    pub fn get_field_by_name(&self, name: &str) -> Option<Cow<'_, Value>> {
        self.desc
            .get_field_by_name(name)
            .map(|field_desc| self.get_field(field_desc.tag()).expect("field not set"))
    }

    pub fn set_field_by_name(&mut self, name: &str, value: Value) {
        if let Some(field_desc) = self.desc.get_field_by_name(name) {
            self.set_field(field_desc.tag(), value)
        }
    }
}

impl DynamicMessageField {
    pub fn new(desc: FieldDescriptor) -> Self {
        DynamicMessageField {
            value: if desc.supports_presence() {
                None
            } else {
                Some(Value::default_value(&desc))
            },
            desc,
        }
    }

    pub fn get(&self) -> Cow<'_, Value> {
        match &self.value {
            Some(value) => Cow::Borrowed(value),
            None => Cow::Owned(Value::default_value(&self.desc)),
        }
    }

    pub fn is_populated(&self) -> bool {
        if self.desc.supports_presence() {
            self.value.is_some()
        } else {
            !self.value.as_ref().unwrap().is_default(&self.desc)
        }
    }

    pub fn set(&mut self, value: Value) {
        // TODO need to check validity
        self.value = Some(value);
    }

    pub fn clear(&mut self) {
        self.value = if self.desc.supports_presence() {
            None
        } else {
            Some(Value::default_value(&self.desc))
        };
    }
}

impl Value {
    pub fn default_value(field_desc: &FieldDescriptor) -> Self {
        if field_desc.is_list() {
            Value::List(Vec::default())
        } else if field_desc.is_map() {
            Value::Map(HashMap::default())
        } else if let Some(default_value) = field_desc.default_value() {
            default_value.clone()
        } else {
            Self::default_value_for_kind(&field_desc.kind())
        }
    }

    fn default_value_for_kind(kind: &FieldDescriptorKind) -> Self {
        match kind {
            FieldDescriptorKind::Message(desc) => Value::Message(DynamicMessage::new(desc.clone())),
            FieldDescriptorKind::Enum(enum_ty) => {
                Value::EnumNumber(enum_ty.default_value().number())
            }
            FieldDescriptorKind::Double => Value::F64(0.0),
            FieldDescriptorKind::Float => Value::F32(0.0),
            FieldDescriptorKind::Int32
            | FieldDescriptorKind::Sint32
            | FieldDescriptorKind::Sfixed32 => Value::I32(0),
            FieldDescriptorKind::Int64
            | FieldDescriptorKind::Sint64
            | FieldDescriptorKind::Sfixed64 => Value::I64(0),
            FieldDescriptorKind::Uint32 | FieldDescriptorKind::Fixed32 => Value::U32(0),
            FieldDescriptorKind::Uint64 | FieldDescriptorKind::Fixed64 => Value::U64(0),
            FieldDescriptorKind::Bool => Value::Bool(false),
            FieldDescriptorKind::String => Value::String(String::default()),
            FieldDescriptorKind::Bytes => Value::Bytes(Bytes::default()),
        }
    }

    pub fn is_default(&self, field_desc: &FieldDescriptor) -> bool {
        *self == Value::default_value(field_desc)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Bool(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match *self {
            Value::U32(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match *self {
            Value::U64(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Value::I64(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            Value::I32(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match *self {
            Value::F32(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Value::F64(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_enum_number(&self) -> Option<i32> {
        match *self {
            Value::EnumNumber(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&Bytes> {
        match self {
            Value::Bytes(value) => Some(value),
            _ => None,
        }
    }
}

impl MapKey {
    pub fn default_value(kind: &FieldDescriptorKind) -> Self {
        match *kind {
            FieldDescriptorKind::Int32
            | FieldDescriptorKind::Sint32
            | FieldDescriptorKind::Sfixed32 => MapKey::I32(0),
            FieldDescriptorKind::Int64
            | FieldDescriptorKind::Sint64
            | FieldDescriptorKind::Sfixed64 => MapKey::I64(0),
            FieldDescriptorKind::Uint32 | FieldDescriptorKind::Fixed32 => MapKey::U32(0),
            FieldDescriptorKind::Uint64 | FieldDescriptorKind::Fixed64 => MapKey::U64(0),
            FieldDescriptorKind::Bool => MapKey::Bool(false),
            FieldDescriptorKind::String => MapKey::String(String::default()),
            _ => panic!("invalid type for map key"),
        }
    }

    pub fn is_default(&self, field_desc: &FieldDescriptorKind) -> bool {
        *self == MapKey::default_value(field_desc)
    }
}

impl From<MapKey> for Value {
    fn from(value: MapKey) -> Self {
        match value {
            MapKey::Bool(value) => Value::Bool(value),
            MapKey::I32(value) => Value::I32(value),
            MapKey::I64(value) => Value::I64(value),
            MapKey::U32(value) => Value::U32(value),
            MapKey::U64(value) => Value::U64(value),
            MapKey::String(value) => Value::String(value),
        }
    }
}
