mod message;

use std::collections::{BTreeMap, HashMap};

use prost::bytes::Bytes;

use crate::{descriptor::FieldDescriptorKind, Descriptor, FieldDescriptor};

#[derive(Debug, Clone)]
pub struct DynamicMessage {
    desc: Descriptor,
    fields: BTreeMap<u32, DynamicValue>,
}

/// A dynamically-typed protobuf value.
///
/// Note this type may map to multiple possible protobuf wire formats, so it must be
/// serialized as part of a DynamicMessage.
#[derive(Debug, Clone)]
pub enum DynamicValue {
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
    List(Vec<DynamicValue>),
    Map(HashMap<MapKey, DynamicValue>),
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
    pub fn new(desc: Descriptor) -> Self {
        DynamicMessage {
            desc,
            fields: BTreeMap::new(),
        }
    }
}

impl DynamicValue {
    pub fn default_value(field_desc: &FieldDescriptor) -> Self {
        if field_desc.is_list() {
            DynamicValue::List(Vec::default())
        } else if field_desc.is_map() {
            DynamicValue::Map(HashMap::default())
        } else {
            Self::default_value_inner(field_desc)
        }
    }

    fn default_value_inner(field_desc: &FieldDescriptor) -> Self {
        match field_desc.kind() {
            FieldDescriptorKind::Message(desc) => DynamicValue::Message(DynamicMessage::new(desc)),
            // TODO this is not correct for proto2 enums, which can have a non-zero default value.
            FieldDescriptorKind::Enum(_) => DynamicValue::EnumNumber(0),
            FieldDescriptorKind::Double => DynamicValue::F64(0.0),
            FieldDescriptorKind::Float => DynamicValue::F32(0.0),
            FieldDescriptorKind::Int32
            | FieldDescriptorKind::Sint32
            | FieldDescriptorKind::Sfixed32 => DynamicValue::I32(0),
            FieldDescriptorKind::Int64
            | FieldDescriptorKind::Sint64
            | FieldDescriptorKind::Sfixed64 => DynamicValue::I64(0),
            FieldDescriptorKind::Uint32 | FieldDescriptorKind::Fixed32 => DynamicValue::U32(0),
            FieldDescriptorKind::Uint64 | FieldDescriptorKind::Fixed64 => DynamicValue::U64(0),
            FieldDescriptorKind::Bool => DynamicValue::Bool(false),
            FieldDescriptorKind::String => DynamicValue::String(String::default()),
            FieldDescriptorKind::Bytes => DynamicValue::Bytes(Bytes::default()),
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            DynamicValue::Bool(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match *self {
            DynamicValue::U32(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match *self {
            DynamicValue::U64(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            DynamicValue::I64(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            DynamicValue::I32(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match *self {
            DynamicValue::F32(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            DynamicValue::F64(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_enum_number(&self) -> Option<i32> {
        match *self {
            DynamicValue::EnumNumber(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            DynamicValue::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&Bytes> {
        match self {
            DynamicValue::Bytes(value) => Some(value),
            _ => None,
        }
    }
}

impl MapKey {
    pub fn default_value(desc: &FieldDescriptor) -> Self {
        match desc.kind() {
            FieldDescriptorKind::Int32
            | FieldDescriptorKind::Sint32
            | FieldDescriptorKind::Sfixed32 => MapKey::I32(0),
            FieldDescriptorKind::Int64
            | FieldDescriptorKind::Sint64
            | FieldDescriptorKind::Sfixed64 => MapKey::I64(0),
            FieldDescriptorKind::Uint32 | FieldDescriptorKind::Fixed32 => MapKey::U32(0),
            FieldDescriptorKind::Uint64 | FieldDescriptorKind::Fixed64 => MapKey::U64(0),
            FieldDescriptorKind::Bool => MapKey::Bool(false),
            _ => panic!("invalid type for map key"),
        }
    }
}

impl From<MapKey> for DynamicValue {
    fn from(value: MapKey) -> Self {
        match value {
            MapKey::Bool(value) => DynamicValue::Bool(value),
            MapKey::I32(value) => DynamicValue::I32(value),
            MapKey::I64(value) => DynamicValue::I64(value),
            MapKey::U32(value) => DynamicValue::U32(value),
            MapKey::U64(value) => DynamicValue::U64(value),
            MapKey::String(value) => DynamicValue::String(value),
        }
    }
}
