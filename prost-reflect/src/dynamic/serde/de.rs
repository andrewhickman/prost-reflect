use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    convert::TryInto,
    fmt,
    str::FromStr,
};

use chrono::{DateTime, Utc};
use prost::{bytes::Bytes, Message};
use serde::de::{
    DeserializeSeed, Deserializer, Error, IgnoredAny, IntoDeserializer, MapAccess, SeqAccess,
    Visitor,
};

use crate::{
    dynamic::{
        serde::{is_well_known_type, DeserializeOptions},
        DynamicMessage, MapKey, Value,
    },
    EnumDescriptor, FieldDescriptor, FileDescriptor, Kind, MessageDescriptor, ReflectMessage,
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
            .deserialize_any(GoogleProtobufAnyVisitor(desc.parent_file(), options))
            .and_then(|timestamp| make_message(desc, timestamp)),
        "google.protobuf.Timestamp" => deserializer
            .deserialize_str(GoogleProtobufTimestampVisitor)
            .and_then(|timestamp| make_message(desc, timestamp)),
        "google.protobuf.Duration" => deserializer
            .deserialize_str(GoogleProtobufDurationVisitor)
            .and_then(|duration| make_message(desc, duration)),
        "google.protobuf.FloatValue" => deserializer
            .deserialize_any(FloatVisitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.DoubleValue" => deserializer
            .deserialize_any(DoubleVisitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.Int32Value" => deserializer
            .deserialize_any(Int32Visitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.Int64Value" => deserializer
            .deserialize_any(Int64Visitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.UInt32Value" => deserializer
            .deserialize_any(Uint32Visitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.UInt64Value" => deserializer
            .deserialize_any(Uint64Visitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.BoolValue" => deserializer
            .deserialize_any(BoolVisitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.StringValue" => deserializer
            .deserialize_any(StringVisitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.BytesValue" => deserializer
            .deserialize_any(BytesVisitor)
            .and_then(|v| make_message(desc, v)),
        "google.protobuf.FieldMask" => deserializer
            .deserialize_str(GoogleProtobufFieldMaskVisitor)
            .and_then(|field_mask| make_message(desc, field_mask)),
        "google.protobuf.Struct" => deserializer
            .deserialize_map(GoogleProtobufStructVisitor)
            .and_then(|value| make_message(desc, value)),
        "google.protobuf.ListValue" => deserializer
            .deserialize_seq(GoogleProtobufListVisitor)
            .and_then(|list| make_message(desc, list)),
        "google.protobuf.Value" => deserializer
            .deserialize_any(GoogleProtobufValueVisitor)
            .and_then(|value| make_message(desc, value)),
        "google.protobuf.Empty" => deserializer
            .deserialize_map(GoogleProtobufEmptyVisitor)
            .and_then(|empty| make_message(desc, empty)),
        _ => deserializer.deserialize_map(MessageVisitor(desc, options)),
    }
}

fn deserialize_enum<'de, D>(desc: &EnumDescriptor, deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    match desc.full_name() {
        "google.protobuf.NullValue" => deserializer.deserialize_unit(GoogleProtobufNullVisitor),
        _ => deserializer.deserialize_any(EnumVisitor(desc)),
    }
}

struct MessageSeed<'a>(&'a MessageDescriptor, &'a DeserializeOptions);

impl<'a, 'de> DeserializeSeed<'de> for MessageSeed<'a> {
    type Value = DynamicMessage;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_message(self.0, deserializer, self.1)
    }
}

struct OptionalFieldDescriptorSeed<'a>(&'a FieldDescriptor, &'a DeserializeOptions);

impl<'a, 'de> DeserializeSeed<'de> for OptionalFieldDescriptorSeed<'a> {
    type Value = Option<Value>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(self)
    }
}

impl<'a, 'de> Visitor<'de> for OptionalFieldDescriptorSeed<'a> {
    type Value = Option<Value>;

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(None)
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(None)
    }

    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        FieldDescriptorSeed(self.0, self.1)
            .deserialize(deserializer)
            .map(Some)
    }

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "option")
    }
}

struct FieldDescriptorSeed<'a>(&'a FieldDescriptor, &'a DeserializeOptions);

impl<'a, 'de> DeserializeSeed<'de> for FieldDescriptorSeed<'a> {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        if self.0.is_list() {
            deserializer
                .deserialize_any(ListVisitor(&self.0.kind(), self.1))
                .map(Value::List)
        } else if self.0.is_map() {
            deserializer
                .deserialize_any(MapVisitor(&self.0.kind(), self.1))
                .map(Value::Map)
        } else {
            KindSeed(&self.0.kind(), self.1).deserialize(deserializer)
        }
    }
}

struct KindSeed<'a>(&'a Kind, &'a DeserializeOptions);

impl<'a, 'de> DeserializeSeed<'de> for KindSeed<'a> {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        match self.0 {
            Kind::Double => deserializer.deserialize_any(DoubleVisitor).map(Value::F64),
            Kind::Float => deserializer.deserialize_any(FloatVisitor).map(Value::F32),
            Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => {
                deserializer.deserialize_any(Int32Visitor).map(Value::I32)
            }
            Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => {
                deserializer.deserialize_any(Int64Visitor).map(Value::I64)
            }
            Kind::Uint32 | Kind::Fixed32 => {
                deserializer.deserialize_any(Uint32Visitor).map(Value::U32)
            }
            Kind::Uint64 | Kind::Fixed64 => {
                deserializer.deserialize_any(Uint64Visitor).map(Value::U64)
            }
            Kind::Bool => deserializer.deserialize_any(BoolVisitor).map(Value::Bool),
            Kind::String => deserializer
                .deserialize_string(StringVisitor)
                .map(Value::String),
            Kind::Bytes => deserializer.deserialize_str(BytesVisitor).map(Value::Bytes),
            Kind::Message(desc) => {
                deserialize_message(desc, deserializer, self.1).map(Value::Message)
            }
            Kind::Enum(desc) => deserialize_enum(desc, deserializer).map(Value::EnumNumber),
        }
    }
}

struct ListVisitor<'a>(&'a Kind, &'a DeserializeOptions);
struct MapVisitor<'a>(&'a Kind, &'a DeserializeOptions);
struct DoubleVisitor;
struct FloatVisitor;
struct Int32Visitor;
struct Uint32Visitor;
struct Int64Visitor;
struct Uint64Visitor;
struct StringVisitor;
struct BoolVisitor;
struct BytesVisitor;
struct MessageVisitor<'a>(&'a MessageDescriptor, &'a DeserializeOptions);
struct MessageVisitorInner<'a>(&'a mut DynamicMessage, &'a DeserializeOptions);
struct EnumVisitor<'a>(&'a EnumDescriptor);

impl<'a, 'de> Visitor<'de> for ListVisitor<'a> {
    type Value = Vec<Value>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a list")
    }

    #[inline]
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut result = Vec::with_capacity(seq.size_hint().unwrap_or(0));

        while let Some(value) = seq.next_element_seed(KindSeed(self.0, self.1))? {
            result.push(value)
        }

        Ok(result)
    }
}

impl<'a, 'de> Visitor<'de> for MapVisitor<'a> {
    type Value = HashMap<MapKey, Value>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut result = HashMap::with_capacity(map.size_hint().unwrap_or(0));

        let map_entry_message = self.0.as_message().unwrap();
        let key_kind = map_entry_message.map_entry_key_field().kind();
        let value_desc = map_entry_message.map_entry_value_field();

        while let Some(key_str) = map.next_key::<Cow<str>>()? {
            let key = match key_kind {
                Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => {
                    MapKey::I32(i32::from_str(key_str.as_ref()).map_err(Error::custom)?)
                }
                Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => {
                    MapKey::I64(i64::from_str(key_str.as_ref()).map_err(Error::custom)?)
                }
                Kind::Uint32 | Kind::Fixed32 => {
                    MapKey::U32(u32::from_str(key_str.as_ref()).map_err(Error::custom)?)
                }
                Kind::Uint64 | Kind::Fixed64 => {
                    MapKey::U64(u64::from_str(key_str.as_ref()).map_err(Error::custom)?)
                }
                Kind::Bool => {
                    MapKey::Bool(bool::from_str(key_str.as_ref()).map_err(Error::custom)?)
                }
                Kind::String => MapKey::String(key_str.into_owned()),
                _ => unreachable!("invalid type for map key"),
            };

            let value = map.next_value_seed(FieldDescriptorSeed(&value_desc, self.1))?;

            result.insert(key, value);
        }

        Ok(result)
    }
}

impl<'de> Visitor<'de> for DoubleVisitor {
    type Value = f64;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 64-bit floating point value")
    }

    #[inline]
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v)
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v as Self::Value)
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v as Self::Value)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match f64::from_str(v) {
            Ok(value) => Ok(value),
            Err(_) if v == "Infinity" => Ok(f64::INFINITY),
            Err(_) if v == "-Infinity" => Ok(f64::NEG_INFINITY),
            Err(_) if v == "NaN" => Ok(f64::NAN),
            Err(err) => Err(Error::custom(err)),
        }
    }
}

impl<'de> Visitor<'de> for FloatVisitor {
    type Value = f32;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 32-bit floating point value")
    }

    #[inline]
    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v)
    }

    #[inline]
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v as f32)
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v as Self::Value)
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v as Self::Value)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match f32::from_str(v) {
            Ok(value) => Ok(value),
            Err(_) if v == "Infinity" => Ok(f32::INFINITY),
            Err(_) if v == "-Infinity" => Ok(f32::NEG_INFINITY),
            Err(_) if v == "NaN" => Ok(f32::NAN),
            Err(err) => Err(Error::custom(err)),
        }
    }
}

impl<'de> Visitor<'de> for Int32Visitor {
    type Value = i32;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 32-bit signed integer")
    }

    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.parse().map_err(Error::custom)
    }

    #[inline]
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v)
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.try_into().map_err(Error::custom)
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.try_into().map_err(Error::custom)
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if v.fract() != 0.0 {
            return Err(Error::custom("expected integer value"));
        }

        if v < (i32::MIN as f64) || v > (i32::MAX as f64) {
            return Err(Error::custom("float value out of range"));
        }

        Ok(v as i32)
    }
}

impl<'de> Visitor<'de> for Uint32Visitor {
    type Value = u32;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 32-bit unsigned integer or decimal string")
    }

    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.parse().map_err(Error::custom)
    }

    #[inline]
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v)
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.try_into().map_err(Error::custom)
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.try_into().map_err(Error::custom)
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if v.fract() != 0.0 {
            return Err(Error::custom("expected integer value"));
        }

        if v < (u32::MIN as f64) || v > (u32::MAX as f64) {
            return Err(Error::custom("float value out of range"));
        }

        Ok(v as u32)
    }
}

impl<'de> Visitor<'de> for Int64Visitor {
    type Value = i64;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 64-bit signed integer or decimal string")
    }

    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.parse().map_err(Error::custom)
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v)
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.try_into().map_err(Error::custom)
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if v.fract() != 0.0 {
            return Err(Error::custom("expected integer value"));
        }

        if v < (i64::MIN as f64) || v > (i64::MAX as f64) {
            return Err(Error::custom("float value out of range"));
        }

        Ok(v as i64)
    }
}

impl<'de> Visitor<'de> for Uint64Visitor {
    type Value = u64;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 64-bit unsigned integer or decimal string")
    }

    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.parse().map_err(Error::custom)
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v)
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.try_into().map_err(Error::custom)
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if v.fract() != 0.0 {
            return Err(Error::custom("expected integer value"));
        }

        if v < (u64::MIN as f64) || v > (u64::MAX as f64) {
            return Err(Error::custom("float value out of range"));
        }

        Ok(v as u64)
    }
}

impl<'de> Visitor<'de> for StringVisitor {
    type Value = String;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a string")
    }

    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v.to_owned())
    }

    #[inline]
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v)
    }
}

impl<'de> Visitor<'de> for BoolVisitor {
    type Value = bool;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a boolean")
    }

    #[inline]
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v)
    }
}

impl<'de> Visitor<'de> for BytesVisitor {
    type Value = Bytes;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a base64-encoded string")
    }

    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        use base64::{decode_config_buf, DecodeError, STANDARD, URL_SAFE};

        let mut buf = Vec::new();
        match decode_config_buf(v, STANDARD, &mut buf) {
            Ok(()) => Ok(buf.into()),
            Err(DecodeError::InvalidByte(_, b'-')) | Err(DecodeError::InvalidByte(_, b'_')) => {
                buf.clear();
                match decode_config_buf(v, URL_SAFE, &mut buf) {
                    Ok(()) => Ok(buf.into()),
                    Err(err) => Err(Error::custom(format!("invalid base64: {}", err))),
                }
            }
            Err(err) => Err(Error::custom(format!("invalid base64: {}", err))),
        }
    }
}

impl<'a, 'de> Visitor<'de> for MessageVisitor<'a> {
    type Value = DynamicMessage;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a map")
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut message = DynamicMessage::new(self.0.clone());

        MessageVisitorInner(&mut message, self.1).visit_map(map)?;

        Ok(message)
    }
}

impl<'a, 'de> Visitor<'de> for MessageVisitorInner<'a> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let desc = self.0.descriptor();
        while let Some(key) = map.next_key::<Cow<str>>()? {
            if let Some(field) = desc
                .get_field_by_json_name(key.as_ref())
                .or_else(|| desc.get_field_by_name(key.as_ref()))
            {
                if let Some(value) =
                    map.next_value_seed(OptionalFieldDescriptorSeed(&field, self.1))?
                {
                    self.0.set_field(field.number(), value);
                }
            } else if self.1.deny_unknown_fields {
                return Err(Error::custom(format!("unrecognized field name '{}'", key)));
            } else {
                let _ = map.next_value::<IgnoredAny>()?;
            }
        }

        Ok(())
    }
}

impl<'a, 'de> Visitor<'de> for EnumVisitor<'a> {
    type Value = i32;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a string or integer")
    }

    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match self.0.get_value_by_name(v) {
            Some(e) => Ok(e.number()),
            None => Err(Error::custom(format!("unrecognized enum value '{}'", v))),
        }
    }

    #[inline]
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v)
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_i32(v.try_into().map_err(Error::custom)?)
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_i32(v.try_into().map_err(Error::custom)?)
    }
}

struct GoogleProtobufAnyVisitor<'a>(&'a FileDescriptor, &'a DeserializeOptions);
struct GoogleProtobufNullVisitor;
struct GoogleProtobufTimestampVisitor;
struct GoogleProtobufDurationVisitor;
struct GoogleProtobufFieldMaskVisitor;
struct GoogleProtobufListVisitor;
struct GoogleProtobufStructVisitor;
struct GoogleProtobufValueVisitor;
struct GoogleProtobufEmptyVisitor;

impl<'a, 'de> Visitor<'de> for GoogleProtobufAnyVisitor<'a> {
    type Value = prost_types::Any;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut buffered_entries = HashMap::new();

        let type_url = loop {
            match map.next_key::<Cow<str>>()? {
                Some(key) if key == "@type" => {
                    break map.next_value::<String>()?;
                }
                Some(key) => {
                    let value: serde_value::Value = map.next_value()?;
                    buffered_entries.insert(key, value);
                }
                None => return Err(Error::custom("expected '@type' field")),
            }
        };

        if let Some(message_name) = type_url.strip_prefix("type.googleapis.com/") {
            let message_desc = self
                .0
                .get_message_by_name(message_name)
                .ok_or_else(|| Error::custom(format!("message '{}' not found", message_name)))?;

            let payload_message = if is_well_known_type(message_name) {
                let payload_message = match buffered_entries.remove("value") {
                    Some(value) => {
                        deserialize_message(&message_desc, value, self.1).map_err(Error::custom)?
                    }
                    None => loop {
                        match map.next_key::<Cow<str>>()? {
                            Some(key) if key == "value" => {
                                break map.next_value_seed(MessageSeed(&message_desc, self.1))?
                            }
                            Some(key) => {
                                if self.1.deny_unknown_fields {
                                    return Err(Error::custom(format!(
                                        "unrecognized field name '{}'",
                                        key
                                    )));
                                } else {
                                    let _ = map.next_value::<IgnoredAny>()?;
                                }
                            }
                            None => return Err(Error::custom("expected '@type' field")),
                        }
                    },
                };

                if self.1.deny_unknown_fields {
                    if let Some(key) = buffered_entries.keys().next() {
                        return Err(Error::custom(format!("unrecognized field name '{}'", key)));
                    }
                    if let Some(key) = map.next_key::<Cow<str>>()? {
                        return Err(Error::custom(format!("unrecognized field name '{}'", key)));
                    }
                } else {
                    drop(buffered_entries);
                    while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                }

                payload_message
            } else {
                let mut payload_message = DynamicMessage::new(message_desc);

                buffered_entries
                    .into_deserializer()
                    .deserialize_map(MessageVisitorInner(&mut payload_message, self.1))
                    .map_err(Error::custom)?;

                MessageVisitorInner(&mut payload_message, self.1).visit_map(map)?;

                payload_message
            };

            let value = payload_message.encode_to_vec();
            Ok(prost_types::Any { type_url, value })
        } else {
            Err(Error::custom(format!(
                "unsupported type url '{}'",
                type_url
            )))
        }
    }
}

impl<'de> Visitor<'de> for GoogleProtobufNullVisitor {
    type Value = i32;

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(0)
    }

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "null")
    }
}

impl<'de> Visitor<'de> for GoogleProtobufTimestampVisitor {
    type Value = prost_types::Timestamp;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a rfc3339 timestamp string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let fixed_offset = DateTime::parse_from_rfc3339(v).map_err(Error::custom)?;

        let utc: DateTime<Utc> = fixed_offset.into();

        let mut timestamp = prost_types::Timestamp {
            seconds: utc.timestamp(),
            nanos: utc.timestamp_subsec_nanos() as i32,
        };
        timestamp.normalize();
        Ok(timestamp)
    }
}

impl<'de> Visitor<'de> for GoogleProtobufDurationVisitor {
    type Value = prost_types::Duration;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a duration string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let v = v
            .strip_suffix('s')
            .ok_or_else(|| Error::custom("invalid duration string"))?;

        let (negative, v) = match v.strip_prefix('-') {
            Some(v) => (true, v),
            None => (false, v),
        };

        let (seconds, nanos) = if let Some((seconds_str, nanos_str)) = v.split_once('.') {
            let seconds = u64::from_str(seconds_str).map_err(Error::custom)?;
            let nanos = match nanos_str.len() {
                0 => 0,
                len @ 1..=9 => {
                    let mut nanos = u32::from_str(nanos_str).map_err(Error::custom)?;
                    for _ in 0..9 - len {
                        nanos *= 10;
                    }
                    nanos
                }
                _ => return Err(Error::custom("too many fractional digits for duration")),
            };

            (seconds, nanos)
        } else {
            let seconds = u64::from_str(v).map_err(Error::custom)?;

            (seconds, 0)
        };

        if seconds > 315_576_000_000 {
            return Err(Error::custom("duration out of range"));
        }
        debug_assert!(nanos < 1_000_000_000);

        if negative {
            Ok(prost_types::Duration {
                seconds: -(seconds as i64),
                nanos: -(nanos as i32),
            })
        } else {
            Ok(prost_types::Duration {
                seconds: seconds as i64,
                nanos: nanos as i32,
            })
        }
    }
}

impl<'de> Visitor<'de> for GoogleProtobufFieldMaskVisitor {
    type Value = prost_types::FieldMask;

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let paths = v
            .split(',')
            .filter(|path| !path.is_empty())
            .map(|path| {
                let mut result = String::new();
                let mut parts = path.split('.');

                if let Some(part) = parts.next() {
                    camel_case_to_snake_case(&mut result, part)?;
                }
                for part in parts {
                    result.push('.');
                    camel_case_to_snake_case(&mut result, part)?;
                }

                Ok(result)
            })
            .collect::<Result<_, ()>>()
            .map_err(|()| Error::custom("invalid field mask"))?;

        Ok(prost_types::FieldMask { paths })
    }

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a field mask string")
    }
}

fn camel_case_to_snake_case(result: &mut String, part: &str) -> Result<(), ()> {
    for ch in part.chars() {
        if ch.is_ascii_uppercase() {
            result.push('_');
            result.push(ch.to_ascii_lowercase());
        } else if ch == '_' {
            return Err(());
        } else {
            result.push(ch);
        }
    }

    Ok(())
}

impl<'de> DeserializeSeed<'de> for GoogleProtobufValueVisitor {
    type Value = prost_types::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for GoogleProtobufListVisitor {
    type Value = prost_types::ListValue;

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(value) = seq.next_element_seed(GoogleProtobufValueVisitor)? {
            values.push(value);
        }
        Ok(prost_types::ListValue { values })
    }

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a list")
    }
}

impl<'de> Visitor<'de> for GoogleProtobufStructVisitor {
    type Value = prost_types::Struct;

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut fields = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value_seed(GoogleProtobufValueVisitor)?;
            fields.insert(key, value);
        }
        Ok(prost_types::Struct { fields })
    }

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a map")
    }
}

impl<'de> Visitor<'de> for GoogleProtobufValueVisitor {
    type Value = prost_types::Value;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(prost_types::Value {
            kind: Some(prost_types::value::Kind::BoolValue(v)),
        })
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v as f64)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v as f64)
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(prost_types::Value {
            kind: Some(prost_types::value::Kind::NumberValue(v)),
        })
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_string(v.to_owned())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(prost_types::Value {
            kind: Some(prost_types::value::Kind::StringValue(v)),
        })
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(prost_types::Value {
            kind: Some(prost_types::value::Kind::NullValue(0)),
        })
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        GoogleProtobufListVisitor
            .visit_seq(seq)
            .map(|l| prost_types::Value {
                kind: Some(prost_types::value::Kind::ListValue(l)),
            })
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        GoogleProtobufStructVisitor
            .visit_map(map)
            .map(|s| prost_types::Value {
                kind: Some(prost_types::value::Kind::StructValue(s)),
            })
    }
}

impl<'de> Visitor<'de> for GoogleProtobufEmptyVisitor {
    type Value = ();

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        if map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {
            return Err(Error::custom("unexpected value in map"));
        }

        Ok(())
    }

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "an empty map")
    }
}

fn make_message<E: Error, T: Message>(
    desc: &MessageDescriptor,
    message: T,
) -> Result<DynamicMessage, E> {
    let mut dynamic = DynamicMessage::new(desc.clone());
    dynamic
        .transcode_from(&message)
        .map_err(|err| Error::custom(format!("error decoding: {}", err)))?;
    Ok(dynamic)
}
