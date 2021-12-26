mod message;

use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

use prost::bytes::Bytes;

use crate::{descriptor::Kind, FieldDescriptor, MessageDescriptor};

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
                Some(Value::default_value_for_field(&desc))
            },
            desc,
        }
    }

    pub fn get(&self) -> Cow<'_, Value> {
        match &self.value {
            Some(value) => Cow::Borrowed(value),
            None => Cow::Owned(Value::default_value_for_field(&self.desc)),
        }
    }

    pub fn is_populated(&self) -> bool {
        if self.desc.supports_presence() {
            self.value.is_some()
        } else {
            !self
                .value
                .as_ref()
                .unwrap()
                .is_default_for_field(&self.desc)
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
            Some(Value::default_value_for_field(&self.desc))
        };
    }
}

impl Value {
    /// Returns the default value for the given protobuf field.
    pub fn default_value_for_field(field_desc: &FieldDescriptor) -> Self {
        if field_desc.is_list() {
            Value::List(Vec::default())
        } else if field_desc.is_map() {
            Value::Map(HashMap::default())
        } else if let Some(default_value) = field_desc.default_value() {
            default_value.clone()
        } else {
            Self::default_value(&field_desc.kind())
        }
    }

    /// Returns the default value for the given protobuf type `kind`.
    pub fn default_value(kind: &Kind) -> Self {
        match kind {
            Kind::Message(desc) => Value::Message(DynamicMessage::new(desc.clone())),
            Kind::Enum(enum_ty) => Value::EnumNumber(enum_ty.default_value().number()),
            Kind::Double => Value::F64(0.0),
            Kind::Float => Value::F32(0.0),
            Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => Value::I32(0),
            Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => Value::I64(0),
            Kind::Uint32 | Kind::Fixed32 => Value::U32(0),
            Kind::Uint64 | Kind::Fixed64 => Value::U64(0),
            Kind::Bool => Value::Bool(false),
            Kind::String => Value::String(String::default()),
            Kind::Bytes => Value::Bytes(Bytes::default()),
        }
    }

    /// Returns `true` if this is the default value for the given protobuf field.
    pub fn is_default_for_field(&self, field_desc: &FieldDescriptor) -> bool {
        *self == Value::default_value_for_field(field_desc)
    }

    /// Returns `true` if this is the default value for the given protobuf type `kind`.
    pub fn is_default(&self, kind: &Kind) -> bool {
        *self == Value::default_value(kind)
    }

    /// Returns the value if it is a `Value::Bool`, or `None` if it is any other type.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::Bool`, or `None` if it is any other type.
    pub fn as_bool_mut(&mut self) -> Option<&mut bool> {
        match self {
            Value::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `Value::U32`, or `None` if it is any other type.
    pub fn as_u32(&self) -> Option<u32> {
        match *self {
            Value::U32(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::U32`, or `None` if it is any other type.
    pub fn as_u32_mut(&mut self) -> Option<&mut u32> {
        match self {
            Value::U32(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `Value::U64`, or `None` if it is any other type.
    pub fn as_u64(&self) -> Option<u64> {
        match *self {
            Value::U64(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::U64`, or `None` if it is any other type.
    pub fn as_u64_mut(&mut self) -> Option<&mut u64> {
        match self {
            Value::U64(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `Value::I64`, or `None` if it is any other type.
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Value::I64(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::I64`, or `None` if it is any other type.
    pub fn as_i64_mut(&mut self) -> Option<&mut i64> {
        match self {
            Value::I64(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `Value::I32`, or `None` if it is any other type.
    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            Value::I32(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::I32`, or `None` if it is any other type.
    pub fn as_i32_mut(&mut self) -> Option<&mut i32> {
        match self {
            Value::I32(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `Value::F32`, or `None` if it is any other type.
    pub fn as_f32(&self) -> Option<f32> {
        match *self {
            Value::F32(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::F32`, or `None` if it is any other type.
    pub fn as_f32_mut(&mut self) -> Option<&mut f32> {
        match self {
            Value::F32(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `Value::F64`, or `None` if it is any other type.
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Value::F64(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::F64`, or `None` if it is any other type.
    pub fn as_f64_mut(&mut self) -> Option<&mut f64> {
        match self {
            Value::F64(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `Value::EnumNumber`, or `None` if it is any other type.
    pub fn as_enum_number(&self) -> Option<i32> {
        match *self {
            Value::EnumNumber(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::EnumNumber`, or `None` if it is any other type.
    pub fn as_enum_number_mut(&mut self) -> Option<&mut i32> {
        match self {
            Value::EnumNumber(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `Value::String`, or `None` if it is any other type.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::String`, or `None` if it is any other type.
    pub fn as_string_mut(&mut self) -> Option<&mut String> {
        match self {
            Value::String(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `Value::Bytes`, or `None` if it is any other type.
    pub fn as_bytes(&self) -> Option<&Bytes> {
        match self {
            Value::Bytes(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::Bytes`, or `None` if it is any other type.
    pub fn as_bytes_mut(&mut self) -> Option<&mut Bytes> {
        match self {
            Value::Bytes(value) => Some(value),
            _ => None,
        }
    }
}

impl MapKey {
    /// Returns the default value for the given protobuf type `kind`.
    ///
    /// # Panics
    ///
    /// Panics if `kind` is not a valid map key type (an integral type or string).
    pub fn default_value(kind: &Kind) -> Self {
        match *kind {
            Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => MapKey::I32(0),
            Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => MapKey::I64(0),
            Kind::Uint32 | Kind::Fixed32 => MapKey::U32(0),
            Kind::Uint64 | Kind::Fixed64 => MapKey::U64(0),
            Kind::Bool => MapKey::Bool(false),
            Kind::String => MapKey::String(String::default()),
            _ => panic!("invalid type for map key"),
        }
    }

    /// Returns `true` if this is the default value for the given protobuf type `kind`.
    ///
    /// # Panics
    ///
    /// Panics if `kind` is not a valid map key type (an integral type or string).
    pub fn is_default(&self, kind: &Kind) -> bool {
        *self == MapKey::default_value(kind)
    }

    /// Returns the value if it is a `MapKey::Bool`, or `None` if it is any other type.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            MapKey::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `MapKey::Bool`, or `None` if it is any other type.
    pub fn as_bool_mut(&mut self) -> Option<&mut bool> {
        match self {
            MapKey::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `MapKey::U32`, or `None` if it is any other type.
    pub fn as_u32(&self) -> Option<u32> {
        match *self {
            MapKey::U32(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `MapKey::U32`, or `None` if it is any other type.
    pub fn as_u32_mut(&mut self) -> Option<&mut u32> {
        match self {
            MapKey::U32(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `MapKey::U64`, or `None` if it is any other type.
    pub fn as_u64(&self) -> Option<u64> {
        match *self {
            MapKey::U64(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `MapKey::U64`, or `None` if it is any other type.
    pub fn as_u64_mut(&mut self) -> Option<&mut u64> {
        match self {
            MapKey::U64(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `MapKey::I64`, or `None` if it is any other type.
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            MapKey::I64(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `MapKey::I64`, or `None` if it is any other type.
    pub fn as_i64_mut(&mut self) -> Option<&mut i64> {
        match self {
            MapKey::I64(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `MapKey::I32`, or `None` if it is any other type.
    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            MapKey::I32(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `MapKey::I32`, or `None` if it is any other type.
    pub fn as_i32_mut(&mut self) -> Option<&mut i32> {
        match self {
            MapKey::I32(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value if it is a `MapKey::String`, or `None` if it is any other type.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            MapKey::String(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `MapKey::String`, or `None` if it is any other type.
    pub fn as_string_mut(&mut self) -> Option<&mut String> {
        match self {
            MapKey::String(value) => Some(value),
            _ => None,
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
