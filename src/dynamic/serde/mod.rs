mod de;
mod ser;

use serde::{
    de::{DeserializeSeed, Deserializer},
    ser::{Serialize, Serializer},
};

use crate::{DynamicMessage, MessageDescriptor};

/// Options to control serialization of messages.
#[derive(Default, Debug, Clone)]
pub struct SerializeOptions {}

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
    /// Creates a new instance of [`DeserializeOptions`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether to error during deserialization when encountering unknown message fields.
    ///
    /// The default value for this field is `true`.
    pub fn deny_unknown_fields(mut self, yes: bool) -> Self {
        self.deny_unknown_fields = yes;
        self
    }
}

impl Default for DeserializeOptions {
    fn default() -> Self {
        DeserializeOptions {
            deny_unknown_fields: true,
        }
    }
}
