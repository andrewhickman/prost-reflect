mod kind;
mod wkt;

use std::fmt;

use prost::Message;
use serde::de::{DeserializeSeed, Deserializer, Error, Visitor};

use crate::{
    dynamic::{fields::FieldDescriptorLike, serde::DeserializeOptions, DynamicMessage, Value},
    EnumDescriptor, Kind, MessageDescriptor,
};

pub(super) fn deserialize_message<'de, D>(
    desc: &MessageDescriptor,
    deserializer: D,
    options: &DeserializeOptions,
) -> Result<DynamicMessage, D::Error>
where
    D: Deserializer<'de>,
{
    match desc.full_name() {
        "google.protobuf.Any" => deserializer
            .deserialize_any(wkt::GoogleProtobufAnyVisitor(desc.parent_pool(), options))
            .and_then(|timestamp| make_message(desc, timestamp)),
        "google.protobuf.Timestamp" => deserializer
            .deserialize_str(wkt::GoogleProtobufTimestampVisitor)
            .and_then(|timestamp| make_message(desc, timestamp)),
        "google.protobuf.Duration" => deserializer
            .deserialize_str(wkt::GoogleProtobufDurationVisitor)
            .and_then(|duration| make_message(desc, duration)),
        "google.protobuf.FloatValue" => deserializer
            .deserialize_any(kind::FloatVisitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.DoubleValue" => deserializer
            .deserialize_any(kind::DoubleVisitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.Int32Value" => deserializer
            .deserialize_any(kind::Int32Visitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.Int64Value" => deserializer
            .deserialize_any(kind::Int64Visitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.UInt32Value" => deserializer
            .deserialize_any(kind::Uint32Visitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.UInt64Value" => deserializer
            .deserialize_any(kind::Uint64Visitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.BoolValue" => deserializer
            .deserialize_any(kind::BoolVisitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.StringValue" => deserializer
            .deserialize_any(kind::StringVisitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.BytesValue" => deserializer
            .deserialize_any(kind::BytesVisitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.FieldMask" => deserializer
            .deserialize_str(wkt::GoogleProtobufFieldMaskVisitor)
            .and_then(|field_mask| make_message(desc, field_mask)),
        "google.protobuf.Struct" => deserializer
            .deserialize_map(wkt::GoogleProtobufStructVisitor)
            .and_then(|value| make_message(desc, value)),
        "google.protobuf.ListValue" => deserializer
            .deserialize_seq(wkt::GoogleProtobufListVisitor)
            .and_then(|list| make_message(desc, list)),
        "google.protobuf.Value" => deserializer
            .deserialize_any(wkt::GoogleProtobufValueVisitor)
            .and_then(|value| make_message(desc, value)),
        "google.protobuf.Empty" => deserializer
            .deserialize_map(wkt::GoogleProtobufEmptyVisitor)
            .and_then(|empty| make_message(desc, empty)),
        _ => deserializer.deserialize_map(kind::MessageVisitor(desc, options)),
    }
}

fn deserialize_enum<'de, D>(
    desc: &EnumDescriptor,
    deserializer: D,
    options: &DeserializeOptions,
) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    match desc.full_name() {
        "google.protobuf.NullValue" => {
            deserializer.deserialize_any(wkt::GoogleProtobufNullVisitor(options))
        }
        _ => deserializer.deserialize_any(kind::EnumVisitor(desc, options)),
    }
}

struct MessageSeed<'a>(&'a MessageDescriptor, &'a DeserializeOptions);

impl<'de> DeserializeSeed<'de> for MessageSeed<'_> {
    type Value = DynamicMessage;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_message(self.0, deserializer, self.1)
    }
}

struct FieldDescriptorSeed<'a, T>(&'a T, &'a DeserializeOptions);

impl<'de, T> DeserializeSeed<'de> for FieldDescriptorSeed<'_, T>
where
    T: FieldDescriptorLike,
{
    type Value = Option<Value>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        if self.0.is_list() {
            Ok(Some(Value::List(deserializer.deserialize_any(
                kind::ListVisitor(&self.0.kind(), self.1),
            )?)))
        } else if self.0.is_map() {
            Ok(Some(Value::Map(deserializer.deserialize_any(
                kind::MapVisitor(&self.0.kind(), self.1),
            )?)))
        } else {
            kind::KindSeed(&self.0.kind(), self.1).deserialize(deserializer)
        }
    }
}

struct OptionalFieldDescriptorSeed<'a, T>(&'a T, &'a DeserializeOptions);

impl<'de, T> DeserializeSeed<'de> for OptionalFieldDescriptorSeed<'_, T>
where
    T: FieldDescriptorLike,
{
    type Value = Option<Value>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(self)
    }
}

impl<'de, T> Visitor<'de> for OptionalFieldDescriptorSeed<'_, T>
where
    T: FieldDescriptorLike,
{
    type Value = Option<Value>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "option")
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_none()
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match self.0.kind() {
            Kind::Message(message_desc) if message_desc.full_name() == "google.protobuf.Value" => {
                make_message(
                    &message_desc,
                    prost_types::Value {
                        kind: Some(prost_types::value::Kind::NullValue(0)),
                    },
                )
                .map(|v| Some(Value::Message(v)))
            }
            Kind::Enum(enum_desc) if enum_desc.full_name() == "google.protobuf.NullValue" => {
                Ok(Some(Value::EnumNumber(0)))
            }
            _ => Ok(None),
        }
    }

    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        FieldDescriptorSeed(self.0, self.1).deserialize(deserializer)
    }
}

fn make_message<E: Error, T: Message>(
    desc: &MessageDescriptor,
    message: T,
) -> Result<DynamicMessage, E> {
    let mut dynamic = DynamicMessage::new(desc.clone());
    dynamic
        .transcode_from(&message)
        .map_err(|err| Error::custom(format!("error decoding: {err}")))?;
    Ok(dynamic)
}
