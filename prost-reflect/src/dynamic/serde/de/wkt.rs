use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    fmt,
    str::FromStr,
};

use chrono::{DateTime, Utc};
use prost::Message;
use serde::de::{
    DeserializeSeed, Deserializer, Error, IgnoredAny, IntoDeserializer, MapAccess, SeqAccess,
    Visitor,
};

use crate::{
    dynamic::{
        serde::{is_well_known_type, DeserializeOptions},
        DynamicMessage,
    },
    FileDescriptor,
};

use super::{deserialize_message, kind::MessageVisitorInner, MessageSeed};

pub struct GoogleProtobufAnyVisitor<'a>(pub &'a FileDescriptor, pub &'a DeserializeOptions);
pub struct GoogleProtobufNullVisitor;
pub struct GoogleProtobufTimestampVisitor;
pub struct GoogleProtobufDurationVisitor;
pub struct GoogleProtobufFieldMaskVisitor;
pub struct GoogleProtobufListVisitor;
pub struct GoogleProtobufStructVisitor;
pub struct GoogleProtobufValueVisitor;
pub struct GoogleProtobufEmptyVisitor;

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
