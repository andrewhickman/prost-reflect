use std::{borrow::Cow, collections::HashMap, convert::TryInto, fmt, str::FromStr};

use prost::bytes::Bytes;
use serde::de::{DeserializeSeed, Deserializer, Error, IgnoredAny, MapAccess, SeqAccess, Visitor};

use crate::{
    dynamic::{serde::DeserializeOptions, DynamicMessage, MapKey, Value},
    EnumDescriptor, Kind, MessageDescriptor, ReflectMessage,
};

use super::{
    deserialize_enum, deserialize_message, FieldDescriptorSeed, OptionalFieldDescriptorSeed,
};

pub struct KindSeed<'a>(pub &'a Kind, pub &'a DeserializeOptions);

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

pub struct ListVisitor<'a>(pub &'a Kind, pub &'a DeserializeOptions);
pub struct MapVisitor<'a>(pub &'a Kind, pub &'a DeserializeOptions);
pub struct DoubleVisitor;
pub struct FloatVisitor;
pub struct Int32Visitor;
pub struct Uint32Visitor;
pub struct Int64Visitor;
pub struct Uint64Visitor;
pub struct StringVisitor;
pub struct BoolVisitor;
pub struct BytesVisitor;
pub struct MessageVisitor<'a>(pub &'a MessageDescriptor, pub &'a DeserializeOptions);
pub struct MessageVisitorInner<'a>(pub &'a mut DynamicMessage, pub &'a DeserializeOptions);
pub struct EnumVisitor<'a>(pub &'a EnumDescriptor);

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
        if v < (f32::MIN as f64) || v > (f32::MAX as f64) {
            Err(Error::custom("float value out of range"))
        } else {
            Ok(v as f32)
        }
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
