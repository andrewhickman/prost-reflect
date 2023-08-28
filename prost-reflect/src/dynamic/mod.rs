/// Parsing and formatting for the protobuf [text format](https://developers.google.com/protocol-buffers/docs/text-format-spec).
///
/// This module contains options for customizing the text format output. See the associated functions [`DynamicMessage::parse_text_format()`] and
/// [`DynamicMessage::to_text_format()`].
#[cfg(feature = "text-format")]
#[cfg_attr(docsrs, doc(cfg(feature = "text-format")))]
pub mod text_format;

mod fields;
mod message;
#[cfg(feature = "serde")]
mod serde;
#[cfg(not(feature = "text-format"))]
mod text_format;
mod unknown;

use std::{borrow::Cow, collections::HashMap, error::Error, fmt};

#[cfg(feature = "serde")]
pub use self::serde::{DeserializeOptions, SerializeOptions};
pub use self::unknown::UnknownField;

pub(crate) use self::fields::FieldDescriptorLike;

use prost::{
    bytes::{Buf, Bytes},
    DecodeError, Message,
};

use self::fields::DynamicMessageFieldSet;
use crate::{
    descriptor::Kind, ExtensionDescriptor, FieldDescriptor, MessageDescriptor, ReflectMessage,
};

/// [`DynamicMessage`] provides encoding, decoding and reflection of a protobuf message.
///
/// It wraps a [`MessageDescriptor`] and the [`Value`] for each field of the message, and implements
/// [`Message`][`prost::Message`].
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicMessage {
    desc: MessageDescriptor,
    fields: DynamicMessageFieldSet,
}

/// A dynamically-typed protobuf value.
///
/// Note this type may map to multiple possible protobuf wire formats, so it must be
/// serialized as part of a DynamicMessage.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A boolean value, encoded as the `bool` protobuf type.
    Bool(bool),
    /// A 32-bit signed integer, encoded as one of the `int32`, `sint32` or `sfixed32` protobuf types.
    I32(i32),
    /// A 64-bit signed integer, encoded as one of the `int64`, `sint64` or `sfixed64` protobuf types.
    I64(i64),
    /// A 32-bit unsigned integer, encoded as one of the `uint32` or `ufixed32` protobuf types.
    U32(u32),
    /// A 64-bit unsigned integer, encoded as one of the `uint64` or `ufixed64` protobuf types.
    U64(u64),
    /// A 32-bit floating point number, encoded as the `float` protobuf type.
    F32(f32),
    /// A 64-bit floating point number, encoded as the `double` protobuf type.
    F64(f64),
    /// A string, encoded as the `string` protobuf type.
    String(String),
    /// A byte string, encoded as the `bytes` protobuf type.
    Bytes(Bytes),
    /// An enumeration value, encoded as a protobuf enum.
    EnumNumber(i32),
    /// A protobuf message.
    Message(DynamicMessage),
    /// A list of values, encoded as a protobuf repeated field.
    List(Vec<Value>),
    /// A map of values, encoded as a protobuf map field.
    Map(HashMap<MapKey, Value>),
}

/// A dynamically-typed key for a protobuf map.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MapKey {
    /// A boolean value, encoded as the `bool` protobuf type.
    Bool(bool),
    /// A 32-bit signed integer, encoded as one of the `int32`, `sint32` or `sfixed32` protobuf types.
    I32(i32),
    /// A 64-bit signed integer, encoded as one of the `int64`, `sint64` or `sfixed64` protobuf types.
    I64(i64),
    /// A 32-bit unsigned integer, encoded as one of the `uint32` or `ufixed32` protobuf types.
    U32(u32),
    /// A 64-bit unsigned integer, encoded as one of the `uint64` or `ufixed64` protobuf types.
    U64(u64),
    /// A string, encoded as the `string` protobuf type.
    String(String),
}

/// Error type returned by [`DynamicMessage::try_set_field()`].
#[derive(Debug, Clone, PartialEq)]
pub enum SetFieldError {
    /// The field was not found.
    NotFound,
    /// The value type was not compatible with the field type (see [`Value::is_valid_for_field`]).
    InvalidType {
        /// The descriptor for the field which could not be set.
        field: FieldDescriptor,
        /// The invalid value.
        value: Value,
    },
}

impl DynamicMessage {
    /// Creates a new, empty instance of [`DynamicMessage`] for the message type specified by the [`MessageDescriptor`].
    pub fn new(desc: MessageDescriptor) -> Self {
        DynamicMessage {
            fields: DynamicMessageFieldSet::default(),
            desc,
        }
    }

    /// Decodes an instance of the message type specified by the [`MessageDescriptor`] from the buffer and merges it into a
    /// new instance of [`DynamicMessage`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_types::FileDescriptorSet;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value};
    /// # let pool = DescriptorPool::decode(include_bytes!("../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("package.MyMessage").unwrap();
    /// let dynamic_message = DynamicMessage::decode(message_descriptor, b"\x08\x96\x01".as_ref()).unwrap();
    /// assert_eq!(dynamic_message.get_field_by_name("foo").unwrap().as_ref(), &Value::I32(150));
    /// ```
    pub fn decode<B>(desc: MessageDescriptor, buf: B) -> Result<Self, DecodeError>
    where
        B: Buf,
    {
        let mut message = DynamicMessage::new(desc);
        message.merge(buf)?;
        Ok(message)
    }

    /// Returns `true` if this message has the given field set.
    ///
    /// If the field type supports distinguishing whether a value has been set (see [`supports_presence`][FieldDescriptor::supports_presence]),
    /// such as for messages, then this method returns `true` only if a value has been set. For
    /// other types, such as integers, it returns `true` if the value is set to a non-default value.
    ///
    /// If this method returns `false`, then the field will not be included in the encoded bytes
    /// of this message.
    ///
    /// # Examples
    ///
    /// This example uses the following message definition:
    ///
    /// ```lang-protobuf
    /// message MyMessage {
    ///     int32 foo = 1;
    ///
    ///     oneof optional {
    ///         int32 bar = 2;
    ///     }
    /// }
    /// ```
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_types::FileDescriptorSet;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value};
    /// # let pool = DescriptorPool::decode(include_bytes!("../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("package.MyMessage").unwrap();
    /// let foo = message_descriptor.get_field_by_name("foo").unwrap();
    /// let bar = message_descriptor.get_field_by_name("bar").unwrap();
    ///
    /// assert!(!foo.supports_presence());
    /// assert!(bar.supports_presence());
    ///
    /// let mut dynamic_message = DynamicMessage::new(message_descriptor);
    /// assert!(!dynamic_message.has_field(&foo));
    /// assert!(!dynamic_message.has_field(&bar));
    ///
    /// dynamic_message.set_field(&foo, Value::I32(0));
    /// dynamic_message.set_field(&bar, Value::I32(0));
    /// assert!(!dynamic_message.has_field(&foo));
    /// assert!(dynamic_message.has_field(&bar));
    ///
    /// dynamic_message.set_field(&foo, Value::I32(5));
    /// dynamic_message.set_field(&bar, Value::I32(6));
    /// assert!(dynamic_message.has_field(&foo));
    /// assert!(dynamic_message.has_field(&bar));
    /// ```
    pub fn has_field(&self, field_desc: &FieldDescriptor) -> bool {
        self.fields.has(field_desc)
    }

    /// Gets the value of the given field, or the default value if it is unset.
    pub fn get_field(&self, field_desc: &FieldDescriptor) -> Cow<'_, Value> {
        self.fields.get(field_desc)
    }

    /// Gets a mutable reference to the value ofthe given field. If the field is not set,
    /// it is inserted with its default value.
    pub fn get_field_mut(&mut self, field_desc: &FieldDescriptor) -> &mut Value {
        self.fields.get_mut(field_desc)
    }

    /// Sets the value of the given field.
    ///
    /// # Panics
    ///
    /// This method may panic if the value type is not compatible with the field type, as defined
    /// by [`Value::is_valid_for_field`]. Consider using [`try_set_field()`](DynamicMessage::try_set_field)
    /// for a non-panicking version.
    pub fn set_field(&mut self, field_desc: &FieldDescriptor, value: Value) {
        self.try_set_field(field_desc, value).unwrap()
    }

    /// Tries to set the value of the given field, returning an error if the value is an invalid type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_types::FileDescriptorSet;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value, SetFieldError};
    /// # let pool = DescriptorPool::decode(include_bytes!("../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("package.MyMessage").unwrap();
    /// let mut dynamic_message = DynamicMessage::new(message_descriptor.clone());
    /// let foo = message_descriptor.get_field_by_name("foo").unwrap();
    /// assert_eq!(dynamic_message.try_set_field(&foo, Value::I32(5)), Ok(()));
    /// assert_eq!(dynamic_message.try_set_field(&foo, Value::String("bar".to_owned())), Err(SetFieldError::InvalidType {
    ///     field: foo,
    ///     value: Value::String("bar".to_owned()),
    /// }));
    /// ```
    pub fn try_set_field(
        &mut self,
        field_desc: &FieldDescriptor,
        value: Value,
    ) -> Result<(), SetFieldError> {
        if value.is_valid_for_field(field_desc) {
            self.fields.set(field_desc, value);
            Ok(())
        } else {
            Err(SetFieldError::InvalidType {
                field: field_desc.clone(),
                value,
            })
        }
    }

    /// Clears the given field.
    ///
    /// After calling this method, `has_field` will return false for the field,
    /// and it will not be included in the encoded bytes of this message.
    pub fn clear_field(&mut self, field_desc: &FieldDescriptor) {
        self.fields.clear(field_desc);
    }

    /// Returns `true` if this message has a field set with the given number.
    ///
    /// See [`has_field`][Self::has_field] for more details.
    pub fn has_field_by_number(&self, number: u32) -> bool {
        self.desc
            .get_field(number)
            .map_or(false, |field_desc| self.has_field(&field_desc))
    }

    /// Gets the value of the field with the given number, or the default value if it is unset.
    ///
    /// If the message has no field with the given number, `None` is returned.
    ///
    /// See [`get_field`][Self::get_field] for more details.
    pub fn get_field_by_number(&self, number: u32) -> Option<Cow<'_, Value>> {
        self.desc
            .get_field(number)
            .map(|field_desc| self.get_field(&field_desc))
    }

    /// Gets a mutable reference to the value of the field with the given number. If the field
    /// is not set, it is inserted with its default value.
    ///
    /// If the message has no field with the given number, `None` is returned.
    ///
    /// See [`get_field_mut`][Self::get_field_mut] for more details.
    pub fn get_field_by_number_mut(&mut self, number: u32) -> Option<&mut Value> {
        self.desc
            .get_field(number)
            .map(move |field_desc| self.get_field_mut(&field_desc))
    }

    /// Sets the value of the field with number `number`.
    ///
    /// If no field with the given number exists, this method does nothing.
    ///
    /// See [`set_field`][Self::set_field] for more details.
    pub fn set_field_by_number(&mut self, number: u32, value: Value) {
        if let Some(field_desc) = self.desc.get_field(number) {
            self.set_field(&field_desc, value)
        }
    }

    /// Tries to set the value of the field with number `number`, returning an error if the value is an invalid type or does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_types::FileDescriptorSet;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value, SetFieldError};
    /// # let pool = DescriptorPool::decode(include_bytes!("../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("package.MyMessage").unwrap();
    /// let mut dynamic_message = DynamicMessage::new(message_descriptor.clone());
    /// assert_eq!(dynamic_message.try_set_field_by_number(1, Value::I32(5)), Ok(()));
    /// assert_eq!(dynamic_message.try_set_field_by_number(1, Value::String("bar".to_owned())), Err(SetFieldError::InvalidType {
    ///     field: message_descriptor.get_field(1).unwrap(),
    ///     value: Value::String("bar".to_owned()),
    /// }));
    /// assert_eq!(dynamic_message.try_set_field_by_number(42, Value::I32(5)), Err(SetFieldError::NotFound));
    /// ```
    pub fn try_set_field_by_number(
        &mut self,
        number: u32,
        value: Value,
    ) -> Result<(), SetFieldError> {
        if let Some(field_desc) = self.desc.get_field(number) {
            self.try_set_field(&field_desc, value)
        } else {
            Err(SetFieldError::NotFound)
        }
    }

    /// Clears the field with the given number.
    ///
    /// If no field with the given number exists, this method does nothing.
    ///
    /// See [`clear_field`][Self::clear_field] for more details.
    pub fn clear_field_by_number(&mut self, number: u32) {
        if let Some(field_desc) = self.desc.get_field(number) {
            self.clear_field(&field_desc);
        }
    }

    /// Returns `true` if this message has a field set with the given name.
    ///
    /// See [`has_field`][Self::has_field] for more details.
    pub fn has_field_by_name(&self, name: &str) -> bool {
        self.desc
            .get_field_by_name(name)
            .map_or(false, |field_desc| self.has_field(&field_desc))
    }

    /// Gets the value of the field with the given name, or the default value if it is unset.
    ///
    /// If the message has no field with the given name, `None` is returned.
    ///
    /// See [`get_field`][Self::get_field] for more details.
    pub fn get_field_by_name(&self, name: &str) -> Option<Cow<'_, Value>> {
        self.desc
            .get_field_by_name(name)
            .map(|field_desc| self.get_field(&field_desc))
    }

    /// Gets a mutable reference to the value of the field with the given name. If the field
    /// is not set, it is inserted with its default value.
    ///
    /// If the message has no field with the given name, `None` is returned.
    ///
    /// See [`get_field_mut`][Self::get_field_mut] for more details.
    pub fn get_field_by_name_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.desc
            .get_field_by_name(name)
            .map(move |field_desc| self.get_field_mut(&field_desc))
    }

    /// Sets the value of the field with name `name`.
    ///
    /// If no field with the given name exists, this method does nothing.
    ///
    /// See [`set_field`][Self::set_field] for more details.
    pub fn set_field_by_name(&mut self, name: &str, value: Value) {
        if let Some(field_desc) = self.desc.get_field_by_name(name) {
            self.set_field(&field_desc, value)
        }
    }

    /// Tries to set the value of the field with name `name`, returning an error if the value is an invalid type or does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_types::FileDescriptorSet;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value, SetFieldError};
    /// # let pool = DescriptorPool::decode(include_bytes!("../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("package.MyMessage").unwrap();
    /// let mut dynamic_message = DynamicMessage::new(message_descriptor.clone());
    /// assert_eq!(dynamic_message.try_set_field_by_name("foo", Value::I32(5)), Ok(()));
    /// assert_eq!(dynamic_message.try_set_field_by_name("foo", Value::String("bar".to_owned())), Err(SetFieldError::InvalidType {
    ///     field: message_descriptor.get_field_by_name("foo").unwrap(),
    ///     value: Value::String("bar".to_owned()),
    /// }));
    /// assert_eq!(dynamic_message.try_set_field_by_name("notfound", Value::I32(5)), Err(SetFieldError::NotFound));
    /// ```
    pub fn try_set_field_by_name(&mut self, name: &str, value: Value) -> Result<(), SetFieldError> {
        if let Some(field_desc) = self.desc.get_field_by_name(name) {
            self.try_set_field(&field_desc, value)
        } else {
            Err(SetFieldError::NotFound)
        }
    }

    /// Clears the field with the given name.
    ///
    /// If no field with the given name exists, this method does nothing.
    ///
    /// See [`clear_field`][Self::clear_field] for more details.
    pub fn clear_field_by_name(&mut self, name: &str) {
        if let Some(field_desc) = self.desc.get_field_by_name(name) {
            self.clear_field(&field_desc);
        }
    }

    /// Clears the value for the given field, and returns it.
    ///
    /// Returns the value if [`has_field`](Self::has_field) was `true`, or `None` otherwise.
    pub fn take_field(&mut self, field_desc: &FieldDescriptor) -> Option<Value> {
        self.fields.take(field_desc)
    }

    /// Clears the value for the field with the given name, and returns it.
    ///
    /// Returns the value if [`has_field_by_name`](Self::has_field_by_name) was `true`, or `None` otherwise.
    pub fn take_field_by_name(&mut self, name: &str) -> Option<Value> {
        if let Some(field_desc) = self.desc.get_field_by_name(name) {
            self.fields.take(&field_desc)
        } else {
            None
        }
    }

    /// Clears the value for the field with the given number, and returns it.
    ///
    /// Returns the value if [`has_field_by_number`](Self::has_field_by_number) was `true`, or `None` otherwise.
    pub fn take_field_by_number(&mut self, number: u32) -> Option<Value> {
        if let Some(field_desc) = self.desc.get_field(number) {
            self.fields.take(&field_desc)
        } else {
            None
        }
    }

    /// Returns `true` if this message has the given extension field set.
    ///
    /// See [`has_field`][Self::has_field] for more details.
    pub fn has_extension(&self, extension_desc: &ExtensionDescriptor) -> bool {
        self.fields.has(extension_desc)
    }

    /// Gets the value of the given extension field, or the default value if it is unset.
    ///
    /// See [`get_field`][Self::get_field] for more details.
    pub fn get_extension(&self, extension_desc: &ExtensionDescriptor) -> Cow<'_, Value> {
        self.fields.get(extension_desc)
    }

    /// Gets a mutable reference to the value of the given extension field. If the
    /// field is not set, it is inserted with its default value.
    ///
    /// See [`get_field_mut`][Self::get_field_mut] for more details.
    pub fn get_extension_mut(&mut self, extension_desc: &ExtensionDescriptor) -> &mut Value {
        self.fields.get_mut(extension_desc)
    }

    /// Sets the value of the given extension field.
    ///
    /// See [`set_field`][Self::set_field] for more details.
    pub fn set_extension(&mut self, extension_desc: &ExtensionDescriptor, value: Value) {
        self.fields.set(extension_desc, value)
    }

    /// Clears the given extension field.
    ///
    /// See [`clear_field`][Self::clear_field] for more details.
    pub fn clear_extension(&mut self, extension_desc: &ExtensionDescriptor) {
        self.fields.clear(extension_desc)
    }

    /// Clears the value for the given extension field, and returns it.
    ///
    /// Returns the value if [`has_extension`](Self::has_extension) was `true`, or `None` otherwise.
    pub fn take_extension(&mut self, extension_desc: &ExtensionDescriptor) -> Option<Value> {
        self.fields.take(extension_desc)
    }

    /// Gets an iterator over all fields of this message.
    ///
    /// The iterator will yield all fields for which [`has_field`](Self::has_field) returns `true`.
    pub fn fields(&self) -> impl Iterator<Item = (FieldDescriptor, &'_ Value)> {
        self.fields.iter_fields(&self.desc)
    }

    /// Gets an iterator over all extension fields of this message.
    ///
    /// The iterator will yield all extension fields for which [`has_extension`](Self::has_extension) returns `true`.
    pub fn extensions(&self) -> impl Iterator<Item = (ExtensionDescriptor, &'_ Value)> {
        self.fields.iter_extensions(&self.desc)
    }

    /// Gets an iterator over unknown fields for this message.
    ///
    /// A field is unknown if the message descriptor does not contain a field with the given number. This is often the
    /// result of a new field being added to the message definition.
    ///
    /// Unknown fields are preserved when decoding and re-encoding a message.
    pub fn unknown_fields(&self) -> impl Iterator<Item = &'_ UnknownField> {
        self.fields.iter_unknown()
    }

    /// Merge a strongly-typed message into this one.
    ///
    /// The message should be compatible with the type specified by
    /// [`descriptor`][Self::descriptor], or the merge will likely fail with
    /// a [`DecodeError`].
    pub fn transcode_from<T>(&mut self, value: &T) -> Result<(), DecodeError>
    where
        T: Message,
    {
        let buf = value.encode_to_vec();
        self.merge(buf.as_slice())
    }

    /// Convert this dynamic message into a strongly typed value.
    ///
    /// The message should be compatible with the type specified by
    /// [`descriptor`][Self::descriptor], or the conversion will likely fail with
    /// a [`DecodeError`].
    pub fn transcode_to<T>(&self) -> Result<T, DecodeError>
    where
        T: Message + Default,
    {
        let buf = self.encode_to_vec();
        T::decode(buf.as_slice())
    }
}

impl ReflectMessage for DynamicMessage {
    fn descriptor(&self) -> MessageDescriptor {
        self.desc.clone()
    }

    fn transcode_to_dynamic(&self) -> DynamicMessage
    where
        Self: Sized,
    {
        self.clone()
    }
}

impl Value {
    /// Returns the default value for the given protobuf field.
    ///
    /// This is equivalent to [`default_value`][Value::default_value] except for the following cases:
    ///
    /// * If the field is a map, an empty map is returned.
    /// * If the field is `repeated`, an empty list is returned.
    /// * If the field has a custom default value specified, that is returned (proto2 only).
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

    /// Returns the default value for the given protobuf extension field.
    ///
    /// See [`default_value_for_field`][Value::default_value_for_field] for more details.
    pub fn default_value_for_extension(extension_desc: &ExtensionDescriptor) -> Self {
        if extension_desc.is_list() {
            Value::List(Vec::default())
        } else if extension_desc.is_map() {
            Value::Map(HashMap::default())
        } else if let Some(default_value) = extension_desc.default_value() {
            default_value.clone()
        } else {
            Self::default_value(&extension_desc.kind())
        }
    }

    /// Returns the default value for the given protobuf type `kind`.
    ///
    /// Unlike [`default_value_for_field`](Value::default_value_for_field), this method does not
    /// look at field cardinality, so it will never return a list or map.
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

    /// Returns `true` if this is the default value for the given protobuf extension field.
    pub fn is_default_for_extension(&self, extension_desc: &ExtensionDescriptor) -> bool {
        *self == Value::default_value_for_extension(extension_desc)
    }

    /// Returns `true` if this is the default value for the given protobuf type `kind`.
    pub fn is_default(&self, kind: &Kind) -> bool {
        *self == Value::default_value(kind)
    }

    /// Returns `true` if this value can be set for a given field.
    ///
    /// Note this only checks if the value can be successfully encoded. It doesn't
    /// check, for example, that enum values are in the defined range.
    pub fn is_valid_for_field(&self, field_desc: &FieldDescriptor) -> bool {
        match (self, field_desc.kind()) {
            (Value::List(list), kind) if field_desc.is_list() => {
                list.iter().all(|value| value.is_valid(&kind))
            }
            (Value::Map(map), Kind::Message(message_desc)) if field_desc.is_map() => {
                let key_desc = message_desc.map_entry_key_field().kind();
                let value_desc = message_desc.map_entry_value_field();
                map.iter().all(|(key, value)| {
                    key.is_valid(&key_desc) && value.is_valid_for_field(&value_desc)
                })
            }
            (value, kind) => value.is_valid(&kind),
        }
    }

    /// Returns `true` if this value can be set for a given extension field.
    ///
    /// See [`is_valid_for_field`][Value::is_valid_for_field] for more details.
    pub fn is_valid_for_extension(&self, extension_desc: &ExtensionDescriptor) -> bool {
        match (self, extension_desc.kind()) {
            (Value::List(list), kind) if extension_desc.is_list() => {
                list.iter().all(|value| value.is_valid(&kind))
            }
            (Value::Map(map), Kind::Message(message_desc)) if extension_desc.is_map() => {
                let key_desc = message_desc.map_entry_key_field().kind();
                let value_desc = message_desc.map_entry_value_field();
                map.iter().all(|(key, value)| {
                    key.is_valid(&key_desc) && value.is_valid_for_field(&value_desc)
                })
            }
            (value, kind) => value.is_valid(&kind),
        }
    }

    /// Returns `true` if this value can be encoded as the given [`Kind`].
    ///
    /// Unlike [`is_valid_for_field`](Value::is_valid_for_field), this method does not
    /// look at field cardinality, so it will never return `true` for lists or maps.
    pub fn is_valid(&self, kind: &Kind) -> bool {
        matches!(
            (self, kind),
            (Value::Bool(_), Kind::Bool)
                | (Value::I32(_), Kind::Int32 | Kind::Sint32 | Kind::Sfixed32)
                | (Value::I64(_), Kind::Int64 | Kind::Sint64 | Kind::Sfixed64)
                | (Value::U32(_), Kind::Uint32 | Kind::Fixed32)
                | (Value::U64(_), Kind::Uint64 | Kind::Fixed64)
                | (Value::F32(_), Kind::Float)
                | (Value::F64(_), Kind::Double)
                | (Value::String(_), Kind::String)
                | (Value::Bytes(_), Kind::Bytes)
                | (Value::EnumNumber(_), Kind::Enum(_))
                | (Value::Message(_), Kind::Message(_))
        )
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

    /// Returns a a reference to the value if it is a `Value::Message`, or `None` if it is any other type.
    pub fn as_message(&self) -> Option<&DynamicMessage> {
        match self {
            Value::Message(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::Message`, or `None` if it is any other type.
    pub fn as_message_mut(&mut self) -> Option<&mut DynamicMessage> {
        match self {
            Value::Message(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a a reference to the value if it is a `Value::List`, or `None` if it is any other type.
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::List`, or `None` if it is any other type.
    pub fn as_list_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::List(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a a reference to the value if it is a `Value::Map`, or `None` if it is any other type.
    pub fn as_map(&self) -> Option<&HashMap<MapKey, Value>> {
        match self {
            Value::Map(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value if it is a `Value::Map`, or `None` if it is any other type.
    pub fn as_map_mut(&mut self) -> Option<&mut HashMap<MapKey, Value>> {
        match self {
            Value::Map(value) => Some(value),
            _ => None,
        }
    }

    /// Converts this value into a [`MapKey`], or `None` if it is not a valid map key type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost_reflect::{Value, MapKey, bytes::Bytes};
    /// assert_eq!(Value::I32(5).into_map_key(), Some(MapKey::I32(5)));
    /// assert_eq!(Value::String("foo".to_owned()).into_map_key(), Some(MapKey::String("foo".to_owned())));
    /// assert_eq!(Value::Bytes(Bytes::from_static(b"bytes")).into_map_key(), None);
    /// ```
    pub fn into_map_key(self) -> Option<MapKey> {
        match self {
            Value::Bool(value) => Some(MapKey::Bool(value)),
            Value::I32(value) => Some(MapKey::I32(value)),
            Value::I64(value) => Some(MapKey::I64(value)),
            Value::U32(value) => Some(MapKey::U32(value)),
            Value::U64(value) => Some(MapKey::U64(value)),
            Value::String(value) => Some(MapKey::String(value)),
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

    /// Returns `true` if this map key can be encoded as the given [`Kind`].
    pub fn is_valid(&self, kind: &Kind) -> bool {
        matches!(
            (self, kind),
            (MapKey::Bool(_), Kind::Bool)
                | (MapKey::I32(_), Kind::Int32 | Kind::Sint32 | Kind::Sfixed32)
                | (MapKey::I64(_), Kind::Int64 | Kind::Sint64 | Kind::Sfixed64)
                | (MapKey::U32(_), Kind::Uint32 | Kind::Fixed32)
                | (MapKey::U64(_), Kind::Uint64 | Kind::Fixed64)
                | (MapKey::String(_), Kind::String)
        )
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

impl fmt::Display for SetFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SetFieldError::NotFound => write!(f, "field not found"),
            SetFieldError::InvalidType { field, value } => {
                write!(f, "expected a value of type '")?;
                if field.is_map() {
                    let entry = field.kind();
                    let entry = entry.as_message().unwrap();
                    write!(
                        f,
                        "map<{:?}, {:?}>",
                        entry.map_entry_key_field().kind(),
                        entry.map_entry_value_field().kind()
                    )?;
                } else if field.is_list() {
                    write!(f, "repeated {:?}", field.kind())?;
                } else {
                    write!(f, "{:?}", field.kind())?;
                }
                write!(f, "', but found '{}'", value)
            }
        }
    }
}

impl Error for SetFieldError {}

pub(crate) fn fmt_string(f: &mut impl fmt::Write, bytes: &[u8]) -> fmt::Result {
    f.write_char('"')?;
    for &ch in bytes {
        match ch {
            b'\t' => f.write_str("\\t")?,
            b'\r' => f.write_str("\\r")?,
            b'\n' => f.write_str("\\n")?,
            b'\\' => f.write_str("\\\\")?,
            b'\'' => f.write_str("\\'")?,
            b'"' => f.write_str("\\\"")?,
            b'\x20'..=b'\x7e' => f.write_char(ch as char)?,
            _ => {
                write!(f, "\\{:03o}", ch)?;
            }
        }
    }
    f.write_char('"')
}

impl fmt::Display for DynamicMessage {
    /// Formats this message using the protobuf text format.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_types::FileDescriptorSet;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value};
    /// # let pool = DescriptorPool::decode(include_bytes!("../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("package.MyMessage").unwrap();
    /// let dynamic_message = DynamicMessage::decode(message_descriptor, b"\x08\x96\x01\x1a\x02\x10\x42".as_ref()).unwrap();
    /// assert_eq!(format!("{}", dynamic_message), "foo:150,nested{bar:66}");
    /// // The alternate format specifier may be used to pretty-print the output
    /// assert_eq!(format!("{:#}", dynamic_message), "foo: 150\nnested {\n  bar: 66\n}");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        text_format::Writer::new(text_format::FormatOptions::new().pretty(f.alternate()), f)
            .fmt_message(self)
    }
}

impl fmt::Display for Value {
    /// Formats this value using the protobuf text format.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::{collections::HashMap, iter::FromIterator};
    /// # use prost_reflect::{MapKey, Value};
    /// assert_eq!(format!("{}", Value::String("hello".to_owned())), "\"hello\"");
    /// assert_eq!(format!("{}", Value::List(vec![Value::I32(1), Value::I32(2)])), "[1,2]");
    /// assert_eq!(format!("{}", Value::Map(HashMap::from_iter([(MapKey::I32(1), Value::U32(2))]))), "[{key:1,value:2}]");
    /// // The alternate format specifier may be used to indent the output
    /// assert_eq!(format!("{:#}", Value::Map(HashMap::from_iter([(MapKey::I32(1), Value::U32(2))]))), "[{\n  key: 1\n  value: 2\n}]");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        text_format::Writer::new(text_format::FormatOptions::new().pretty(f.alternate()), f)
            .fmt_value(self, None)
    }
}

#[test]
fn type_sizes() {
    assert_eq!(std::mem::size_of::<DynamicMessage>(), 40);
    assert_eq!(std::mem::size_of::<Value>(), 56);
}
