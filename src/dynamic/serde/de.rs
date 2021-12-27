use std::{borrow::Cow, collections::HashMap, convert::TryInto, fmt, str::FromStr};

use prost::bytes::Bytes;
use serde::de::{DeserializeSeed, Deserializer, Error, IgnoredAny, MapAccess, SeqAccess, Visitor};

use crate::{
    descriptor::{MAP_ENTRY_KEY_NUMBER, MAP_ENTRY_VALUE_NUMBER},
    dynamic::{DynamicMessage, MapKey, Value},
    EnumDescriptor, FieldDescriptor, Kind, MessageDescriptor,
};

impl<'a, 'de> DeserializeSeed<'de> for &'a MessageDescriptor {
    type Value = DynamicMessage;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(MessageVisitor(self))
    }
}

struct OptionalFieldDescriptorSeed<'a>(&'a FieldDescriptor);

impl<'a, 'de: 'a> DeserializeSeed<'de> for OptionalFieldDescriptorSeed<'a> {
    type Value = Option<Value>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(self)
    }
}

impl<'a, 'de: 'a> Visitor<'de> for OptionalFieldDescriptorSeed<'a> {
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
        FieldDescriptorSeed(self.0)
            .deserialize(deserializer)
            .map(Some)
    }

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "option")
    }
}

struct FieldDescriptorSeed<'a>(&'a FieldDescriptor);

impl<'a, 'de: 'a> DeserializeSeed<'de> for FieldDescriptorSeed<'a> {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        if self.0.is_list() {
            deserializer
                .deserialize_any(ListVisitor(self.0))
                .map(Value::List)
        } else if self.0.is_map() {
            deserializer
                .deserialize_any(MapVisitor(self.0))
                .map(Value::Map)
        } else {
            match self.0.kind() {
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
                Kind::Message(desc) => deserializer
                    .deserialize_map(MessageVisitor(&desc))
                    .map(Value::Message),
                Kind::Enum(desc) => deserializer
                    .deserialize_any(EnumVisitor(&desc))
                    .map(Value::EnumNumber),
            }
        }
    }
}

struct ListVisitor<'a>(&'a FieldDescriptor);
struct MapVisitor<'a>(&'a FieldDescriptor);
struct DoubleVisitor;
struct FloatVisitor;
struct Int32Visitor;
struct Uint32Visitor;
struct Int64Visitor;
struct Uint64Visitor;
struct StringVisitor;
struct BoolVisitor;
struct BytesVisitor;
struct MessageVisitor<'a>(&'a MessageDescriptor);
struct EnumVisitor<'a>(&'a EnumDescriptor);

impl<'a, 'de: 'a> Visitor<'de> for ListVisitor<'a> {
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

        while let Some(value) = seq.next_element_seed(FieldDescriptorSeed(self.0))? {
            result.push(value)
        }

        Ok(result)
    }
}

impl<'a, 'de: 'a> Visitor<'de> for MapVisitor<'a> {
    type Value = HashMap<MapKey, Value>;

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut result = HashMap::with_capacity(map.size_hint().unwrap_or(0));

        let kind = self.0.kind();
        let map_entry_message = kind.as_message().unwrap();
        let key_kind = map_entry_message
            .get_field(MAP_ENTRY_KEY_NUMBER)
            .unwrap()
            .kind();
        let value_desc = map_entry_message.get_field(MAP_ENTRY_VALUE_NUMBER).unwrap();

        while let Some(key_str) = map.next_key::<Cow<'de, str>>()? {
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

            let value = map.next_value_seed(FieldDescriptorSeed(&value_desc))?;

            result.insert(key, value);
        }

        Ok(result)
    }

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a map")
    }
}

impl<'de> Visitor<'de> for DoubleVisitor {
    type Value = f64;

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

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 64-bit floating point value")
    }
}

impl<'de> Visitor<'de> for FloatVisitor {
    type Value = f32;

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

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 32-bit floating point value")
    }
}

impl<'de> Visitor<'de> for Int32Visitor {
    type Value = i32;

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

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 32-bit signed integer")
    }
}

impl<'de> Visitor<'de> for Uint32Visitor {
    type Value = u32;

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

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 32-bit unsigned integer or decimal string")
    }
}

impl<'de> Visitor<'de> for Int64Visitor {
    type Value = i64;

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

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 64-bit signed integer or decimal string")
    }
}

impl<'de> Visitor<'de> for Uint64Visitor {
    type Value = u64;

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

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 64-bit unsigned integer or decimal string")
    }
}

impl<'de> Visitor<'de> for StringVisitor {
    type Value = String;

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

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a string")
    }
}

impl<'de> Visitor<'de> for BoolVisitor {
    type Value = bool;

    #[inline]
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v)
    }

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a boolean")
    }
}

impl<'de> Visitor<'de> for BytesVisitor {
    type Value = Bytes;

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

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a base64-encoded string")
    }
}

impl<'a, 'de: 'a> Visitor<'de> for MessageVisitor<'a> {
    type Value = DynamicMessage;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut message = DynamicMessage::new(self.0.clone());

        while let Some(key) = map.next_key::<&'de str>()? {
            if let Some(field) = self
                .0
                .get_field_by_json_name(key)
                .or_else(|| self.0.get_field_by_name(key))
            {
                if let Some(value) = map.next_value_seed(OptionalFieldDescriptorSeed(&field))? {
                    message.set_field(field.number(), value);
                }
            } else {
                let _ = map.next_value::<IgnoredAny>()?;
            }
        }

        Ok(message)
    }
}

impl<'a, 'de: 'a> Visitor<'de> for EnumVisitor<'a> {
    type Value = i32;

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

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a string or integer")
    }
}
