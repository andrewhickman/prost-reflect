mod de;
mod ser;

use serde::{
    de::{DeserializeSeed, Deserializer},
    ser::{Serialize, Serializer},
};

use crate::{DynamicMessage, MessageDescriptor};

#[derive(Default, Debug, Clone)]
pub struct SerializeOptions {}

#[derive(Default, Debug, Clone)]
pub struct DeserializeOptions {}

impl Serialize for DynamicMessage {
    /// Serialize this message into `serializer` using the [canonical JSON encoding](https://developers.google.com/protocol-buffers/docs/proto3#json).
    ///
    /// This method is only available if the `serde` feature is enabled.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.serialize_with_config(serializer, &Default::default())
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
    /// Serialize this message into `serializer` using the encoding specified by `config`.
    ///
    /// This method is only available if the `serde` feature is enabled.
    pub fn serialize_with_config<S>(
        &self,
        serializer: S,
        config: &SerializeOptions,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ser::serialize_message(self, serializer, config)
    }

    /// Deserialize an instance of the message type described by `desc` from `deserializer`.
    ///
    /// This method is only available if the `serde` feature is enabled.
    pub fn deserialize<'de, D>(desc: MessageDescriptor, deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::deserialize_with_config(desc, deserializer, &Default::default())
    }

    /// Deserialize an instance of the message type described by `desc` from `deserializer`, using
    /// the encoding specified by `config`.
    ///
    /// This method is only available if the `serde` feature is enabled.
    pub fn deserialize_with_config<'de, D>(
        desc: MessageDescriptor,
        deserializer: D,
        config: &DeserializeOptions,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        de::deserialize_message(&desc, deserializer, config)
    }
}
