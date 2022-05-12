use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    fmt,
};

use prost::Message;
use serde::de::{
    DeserializeSeed, Deserializer, Error, IgnoredAny, IntoDeserializer, MapAccess, SeqAccess,
    Visitor,
};

use crate::{
    dynamic::{
        serde::{
            case::camel_case_to_snake_case, check_duration, check_timestamp, is_well_known_type,
            DeserializeOptions,
        },
        DynamicMessage,
    },
    DescriptorPool,
};

use super::{deserialize_message, kind::MessageVisitorInner, MessageSeed};

pub struct GoogleProtobufAnyVisitor<'a>(pub &'a DescriptorPool, pub &'a DeserializeOptions);
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

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "null")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if v == "NULL_VALUE" {
            Ok(0)
        } else {
            Err(Error::custom("expected null"))
        }
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(0)
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
        validate_strict_rfc3339(v).map_err(Error::custom)?;

        let timestamp: prost_types::Timestamp = v.parse().map_err(Error::custom)?;

        check_timestamp(&timestamp).map_err(Error::custom)?;

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
        let duration: prost_types::Duration = v.parse().map_err(Error::custom)?;

        check_duration(&duration).map_err(Error::custom)?;

        Ok(duration)
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

    #[inline]
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

/// Validates the string is a valid RFC3339 timestamp, requiring upper-case
/// 'T' and 'Z' characters as recommended by the conformance tests.
fn validate_strict_rfc3339(v: &str) -> Result<(), String> {
    use std::{ascii, iter::Peekable, str::Bytes};

    fn pop_digit(bytes: &mut Peekable<Bytes>) -> bool {
        bytes.next_if(u8::is_ascii_digit).is_some()
    }

    fn pop_digits(bytes: &mut Peekable<Bytes>, n: usize) -> bool {
        (0..n).all(|_| pop_digit(bytes))
    }

    fn pop_char(p: &mut Peekable<Bytes>, c: u8) -> bool {
        p.next_if_eq(&c).is_some()
    }

    fn fmt_next(p: &mut Peekable<Bytes>) -> String {
        match p.peek() {
            Some(&ch) => format!("'{}'", ascii::escape_default(ch)),
            None => "end of string".to_owned(),
        }
    }

    let mut v = v.bytes().peekable();

    if !(pop_digits(&mut v, 4)
        && pop_char(&mut v, b'-')
        && pop_digits(&mut v, 2)
        && pop_char(&mut v, b'-')
        && pop_digits(&mut v, 2))
    {
        return Err("invalid rfc3339 timestamp: invalid date".to_owned());
    }

    if !pop_char(&mut v, b'T') {
        return Err(format!(
            "invalid rfc3339 timestamp: expected 'T' but found {}",
            fmt_next(&mut v)
        ));
    }

    if !(pop_digits(&mut v, 2)
        && pop_char(&mut v, b':')
        && pop_digits(&mut v, 2)
        && pop_char(&mut v, b':')
        && pop_digits(&mut v, 2))
    {
        return Err("invalid rfc3339 timestamp: invalid time".to_owned());
    }

    if pop_char(&mut v, b'.') {
        if !pop_digit(&mut v) {
            return Err("invalid rfc3339 timestamp: empty fractional seconds".to_owned());
        }
        while pop_digit(&mut v) {}
    }

    if v.next_if(|&ch| ch == b'+' || ch == b'-').is_some() {
        if !(pop_digits(&mut v, 2) && pop_char(&mut v, b':') && pop_digits(&mut v, 2)) {
            return Err("invalid rfc3339 timestamp: invalid offset".to_owned());
        }
    } else if !pop_char(&mut v, b'Z') {
        return Err(format!(
            "invalid rfc3339 timestamp: expected 'Z', '+' or '-' but found {}",
            fmt_next(&mut v)
        ));
    }

    if v.peek().is_some() {
        return Err(format!(
            "invalid rfc3339 timestamp: expected end of string but found {}",
            fmt_next(&mut v)
        ));
    }

    Ok(())
}

#[test]
fn test_validate_strict_rfc3339() {
    macro_rules! case {
        ($s:expr => Ok) => {
            assert_eq!(validate_strict_rfc3339($s), Ok(()))
        };
        ($s:expr => Err($e:expr)) => {
            assert_eq!(validate_strict_rfc3339($s).unwrap_err().to_string(), $e)
        };
    }

    case!("1972-06-30T23:59:60Z" => Ok);
    case!("2019-03-26T14:00:00.9Z" => Ok);
    case!("2019-03-26T14:00:00.4999Z" => Ok);
    case!("2019-03-26T14:00:00.4999+10:00" => Ok);
    case!("2019-03-26t14:00Z" => Err("invalid rfc3339 timestamp: expected 'T' but found 't'"));
    case!("2019-03-26T14:00z" => Err("invalid rfc3339 timestamp: invalid time"));
    case!("2019-03-26T14:00:00,999Z" => Err("invalid rfc3339 timestamp: expected 'Z', '+' or '-' but found ','"));
    case!("2019-03-26T10:00-04" => Err("invalid rfc3339 timestamp: invalid time"));
    case!("2019-03-26T14:00.9Z" => Err("invalid rfc3339 timestamp: invalid time"));
    case!("20190326T1400Z" => Err("invalid rfc3339 timestamp: invalid date"));
    case!("2019-02-30" => Err("invalid rfc3339 timestamp: expected 'T' but found end of string"));
    case!("2019-03-25T24:01Z" => Err("invalid rfc3339 timestamp: invalid time"));
    case!("2019-03-26T14:00+24:00" => Err("invalid rfc3339 timestamp: invalid time"));
    case!("2019-03-26Z" => Err("invalid rfc3339 timestamp: expected 'T' but found 'Z'"));
    case!("2019-03-26+01:00" => Err("invalid rfc3339 timestamp: expected 'T' but found '+'"));
    case!("2019-03-26-04:00" => Err("invalid rfc3339 timestamp: expected 'T' but found '-'"));
    case!("2019-03-26T10:00-0400" => Err("invalid rfc3339 timestamp: invalid time"));
    case!("+0002019-03-26T14:00Z" => Err("invalid rfc3339 timestamp: invalid date"));
    case!("+2019-03-26T14:00Z" => Err("invalid rfc3339 timestamp: invalid date"));
    case!("002019-03-26T14:00Z" => Err("invalid rfc3339 timestamp: invalid date"));
    case!("019-03-26T14:00Z" => Err("invalid rfc3339 timestamp: invalid date"));
    case!("2019-03-26T10:00Q" => Err("invalid rfc3339 timestamp: invalid time"));
    case!("2019-03-26T10:00T" => Err("invalid rfc3339 timestamp: invalid time"));
    case!("2019-03-26Q" => Err("invalid rfc3339 timestamp: expected 'T' but found 'Q'"));
    case!("2019-03-26T" => Err("invalid rfc3339 timestamp: invalid time"));
    case!("2019-03-26 14:00Z" => Err("invalid rfc3339 timestamp: expected 'T' but found ' '"));
    case!("2019-03-26T14:00:00." => Err("invalid rfc3339 timestamp: empty fractional seconds"));
}
