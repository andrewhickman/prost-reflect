mod message;

use std::collections::{BTreeMap, HashMap};

use prost::bytes::Bytes;

use crate::{descriptor::FieldDescriptorKind, Descriptor, FieldDescriptor};

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicMessage {
    desc: Descriptor,
    fields: BTreeMap<u32, DynamicMessageField>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicMessageField {
    desc: FieldDescriptor,
    value: Value,
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
    Message(Option<DynamicMessage>),
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
    pub fn new(desc: Descriptor) -> Self {
        DynamicMessage {
            fields: desc
                .fields()
                .map(|field_desc| (field_desc.tag(), DynamicMessageField::new(field_desc)))
                .collect(),
            desc,
        }
    }

    pub fn descriptor(&self) -> Descriptor {
        self.desc.clone()
    }

    pub fn get_field(&self, tag: u32) -> Option<&DynamicMessageField> {
        self.fields.get(&tag)
    }

    pub fn get_field_mut(&mut self, tag: u32) -> Option<&mut DynamicMessageField> {
        self.fields.get_mut(&tag)
    }

    pub fn get_field_by_name(&self, name: &str) -> Option<&DynamicMessageField> {
        self.desc
            .get_field_by_name(name)
            .map(|field_desc| self.get_field(field_desc.tag()).expect("field not set"))
    }

    pub fn get_field_by_name_mut(&mut self, name: &str) -> Option<&mut DynamicMessageField> {
        self.desc
            .get_field_by_name(name)
            .map(move |field_desc| self.get_field_mut(field_desc.tag()).expect("field not set"))
    }
}

impl DynamicMessageField {
    fn new(desc: FieldDescriptor) -> Self {
        DynamicMessageField {
            value: Value::default_value(&desc),
            desc,
        }
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut Value {
        &mut self.value
    }

    pub fn clear(&mut self) {
        self.value = Value::default_value(&self.desc);
    }
}

impl Value {
    pub fn default_value(field_desc: &FieldDescriptor) -> Self {
        if field_desc.is_list() {
            Value::List(Vec::default())
        } else if field_desc.is_map() {
            Value::Map(HashMap::default())
        } else {
            Self::default_value_for_kind(&field_desc.kind())
        }
    }

    fn default_value_for_kind(kind: &FieldDescriptorKind) -> Self {
        match kind {
            FieldDescriptorKind::Message(_) => Value::Message(None),
            // TODO this is not correct for proto2 enums, which can have a non-zero default value.
            FieldDescriptorKind::Enum(_) => Value::EnumNumber(0),
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
