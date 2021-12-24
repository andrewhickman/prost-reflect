mod message;

use std::collections::{BTreeMap, HashMap};

use prost::bytes::Bytes;

use crate::{descriptor::ty, Descriptor, FieldDescriptor};

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
    pub fn default_value(desc: &FieldDescriptor) -> Self {
        match desc.ty() {
            ty::Type::Message(_) => {
                DynamicValue::Message(DynamicMessage::new(desc.message_descriptor().unwrap()))
            }
            // TODO this is not correct for proto2 enums, which can have a non-zero default value.
            ty::Type::Enum(_) => DynamicValue::EnumNumber(0),
            ty::Type::Scalar(scalar) => match scalar {
                ty::Scalar::Double => DynamicValue::F64(0.0),
                ty::Scalar::Float => DynamicValue::F32(0.0),
                ty::Scalar::Int32 | ty::Scalar::Sint32 | ty::Scalar::Sfixed32 => {
                    DynamicValue::I32(0)
                }
                ty::Scalar::Int64 | ty::Scalar::Sint64 | ty::Scalar::Sfixed64 => {
                    DynamicValue::I64(0)
                }
                ty::Scalar::Uint32 | ty::Scalar::Fixed32 => DynamicValue::U32(0),
                ty::Scalar::Uint64 | ty::Scalar::Fixed64 => DynamicValue::U64(0),
                ty::Scalar::Bool => DynamicValue::Bool(false),
                ty::Scalar::String => DynamicValue::String(String::default()),
                ty::Scalar::Bytes => DynamicValue::Bytes(Bytes::default()),
            },
            ty::Type::List(_) => DynamicValue::List(Vec::default()),
            ty::Type::Map(_) => DynamicValue::Map(HashMap::default()),
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
