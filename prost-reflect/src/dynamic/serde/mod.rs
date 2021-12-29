mod de;
mod ser;

use serde::{
    de::{DeserializeSeed, Deserializer},
    ser::{Serialize, Serializer},
};

use crate::{DynamicMessage, MessageDescriptor};

/// Options to control serialization of messages.
#[derive(Debug, Clone)]
pub struct SerializeOptions {
    stringify_64_bit_integers: bool,
    use_enum_numbers: bool,
    use_proto_field_name: bool,
    emit_unpopulated_fields: bool,
}

/// Options to control deserialization of messages.
#[derive(Debug, Clone)]
pub struct DeserializeOptions {
    deny_unknown_fields: bool,
}

impl Serialize for DynamicMessage {
    /// Serialize this message into `serializer` using the [canonical JSON encoding](https://developers.google.com/protocol-buffers/docs/proto3#json).
    ///
    /// This method is only available if the `serde` feature is enabled.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.serialize_with_options(serializer, &Default::default())
    }
}

impl<'de> DeserializeSeed<'de> for MessageDescriptor {
    type Value = DynamicMessage;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        DynamicMessage::deserialize(self, deserializer)
    }
}

impl DynamicMessage {
    /// Serialize this message into `serializer` using the encoding specified by `options`.
    ///
    /// This method is only available if the `serde` feature is enabled.
    pub fn serialize_with_options<S>(
        &self,
        serializer: S,
        options: &SerializeOptions,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ser::serialize_message(self, serializer, options)
    }

    /// Deserialize an instance of the message type described by `desc` from `deserializer`.
    ///
    /// This method is only available if the `serde` feature is enabled.
    pub fn deserialize<'de, D>(desc: MessageDescriptor, deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::deserialize_with_options(desc, deserializer, &Default::default())
    }

    /// Deserialize an instance of the message type described by `desc` from `deserializer`, using
    /// the encoding specified by `options`.
    ///
    /// This method is only available if the `serde` feature is enabled.
    pub fn deserialize_with_options<'de, D>(
        desc: MessageDescriptor,
        deserializer: D,
        options: &DeserializeOptions,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        de::deserialize_message(&desc, deserializer, options)
    }
}

impl DeserializeOptions {
    /// Creates a new instance of [`DeserializeOptions`], with the default options chosen to conform to
    /// the standard JSON mapping.
    pub const fn new() -> Self {
        DeserializeOptions {
            deny_unknown_fields: true,
        }
    }

    /// Whether to error during deserialization when encountering unknown message fields.
    ///
    /// The default value is `true`.
    pub const fn deny_unknown_fields(mut self, yes: bool) -> Self {
        self.deny_unknown_fields = yes;
        self
    }
}

impl Default for DeserializeOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl SerializeOptions {
    /// Creates a new instance of [`SerializeOptions`], with the default options chosen to conform to
    /// the standard JSON mapping.
    pub const fn new() -> Self {
        SerializeOptions {
            stringify_64_bit_integers: true,
            use_enum_numbers: false,
            use_proto_field_name: false,
            emit_unpopulated_fields: false,
        }
    }

    /// Whether to encode 64-bit integral types as strings.
    ///
    /// The spec requires encoding 64-bit integers as strings, to prevent loss of precision in JSON
    /// when the value cannot be represented exactly by a double. If this option is disabled, all
    /// numbers will be serialized as their corresponding serde types instead.
    ///
    /// The default value is `true`.
    pub const fn stringify_64_bit_integers(mut self, yes: bool) -> Self {
        self.stringify_64_bit_integers = yes;
        self
    }

    /// Whether to encode enum values as their numeric value.
    ///
    /// If `true`, enum values will be serialized as their integer values. Otherwise, they will be
    /// serialized as the string value specified in the proto file.
    ///
    /// The default value is `false`.
    pub const fn use_enum_numbers(mut self, yes: bool) -> Self {
        self.use_enum_numbers = yes;
        self
    }

    /// Whether to use the proto field name instead of the lowerCamelCase name in JSON field names.
    ///
    /// The default value is `false`.
    pub const fn use_proto_field_name(mut self, yes: bool) -> Self {
        self.use_proto_field_name = yes;
        self
    }

    /// Whether to emit unpopulated fields.
    ///
    /// If `false`, any fields for which [`has_field`][DynamicMessage::has_field] returns `true` will
    /// not be serialized. If `true`, they will be serialized with their default value.
    ///
    /// The default value is `false`.
    pub const fn emit_unpopulated_fields(mut self, yes: bool) -> Self {
        self.emit_unpopulated_fields = yes;
        self
    }
}

impl Default for SerializeOptions {
    fn default() -> Self {
        Self::new()
    }
}
