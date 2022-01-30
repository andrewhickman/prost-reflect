use std::cmp::Ordering;

use base64::display::Base64Display;
use prost::{DecodeError, Message};
use serde::ser::{Error, Serialize, SerializeMap, SerializeSeq, Serializer};
use time::{Duration, OffsetDateTime, UtcOffset};

use crate::{
    dynamic::{
        serde::{
            case::snake_case_to_camel_case, is_well_known_type, SerializeOptions,
            MAX_DURATION_NANOS, MAX_DURATION_SECONDS, MAX_TIMESTAMP_SECONDS, MIN_TIMESTAMP_SECONDS,
        },
        DynamicMessage,
    },
    ReflectMessage,
};

use super::{serialize_dynamic_message_fields, SerializeWrapper};

#[allow(type_alias_bounds)]
type WellKnownTypeSerializer<S: Serializer> =
    fn(&DynamicMessage, S, &SerializeOptions) -> Result<S::Ok, S::Error>;

pub fn get_well_known_type_serializer<S>(full_name: &str) -> Option<WellKnownTypeSerializer<S>>
where
    S: Serializer,
{
    match full_name {
        "google.protobuf.Any" => Some(serialize_any),
        "google.protobuf.Timestamp" => Some(serialize_timestamp),
        "google.protobuf.Duration" => Some(serialize_duration),
        "google.protobuf.Struct" => Some(serialize_struct),
        "google.protobuf.FloatValue" => Some(serialize_float),
        "google.protobuf.DoubleValue" => Some(serialize_double),
        "google.protobuf.Int32Value" => Some(serialize_int32),
        "google.protobuf.Int64Value" => Some(serialize_int64),
        "google.protobuf.UInt32Value" => Some(serialize_uint32),
        "google.protobuf.UInt64Value" => Some(serialize_uint64),
        "google.protobuf.BoolValue" => Some(serialize_bool),
        "google.protobuf.StringValue" => Some(serialize_string),
        "google.protobuf.BytesValue" => Some(serialize_bytes),
        "google.protobuf.FieldMask" => Some(serialize_field_mask),
        "google.protobuf.ListValue" => Some(serialize_list),
        "google.protobuf.Value" => Some(serialize_value),
        "google.protobuf.Empty" => Some(serialize_empty),
        _ => {
            debug_assert!(!is_well_known_type(full_name));
            None
        }
    }
}

fn serialize_any<S>(
    msg: &DynamicMessage,
    serializer: S,
    options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: prost_types::Any = msg.transcode_to().map_err(decode_to_ser_err)?;

    if let Some(message_name) = raw.type_url.strip_prefix("type.googleapis.com/") {
        let message_desc = msg
            .descriptor()
            .parent_file()
            .get_message_by_name(message_name)
            .ok_or_else(|| Error::custom(format!("message '{}' not found", message_name)))?;

        let mut payload_message = DynamicMessage::new(message_desc);
        payload_message
            .merge(raw.value.as_ref())
            .map_err(decode_to_ser_err)?;

        if is_well_known_type(message_name) {
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry("@type", &raw.type_url)?;
            map.serialize_entry(
                "value",
                &SerializeWrapper {
                    value: &payload_message,
                    options,
                },
            )?;
            map.end()
        } else {
            let mut map = serializer.serialize_map(None)?;
            map.serialize_entry("@type", &raw.type_url)?;
            serialize_dynamic_message_fields(&mut map, &payload_message, options)?;
            map.end()
        }
    } else {
        Err(Error::custom(format!(
            "unsupported type url '{}'",
            raw.type_url
        )))
    }
}

fn serialize_timestamp<S>(
    msg: &DynamicMessage,
    serializer: S,
    _options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: prost_types::Timestamp = msg.transcode_to().map_err(decode_to_ser_err)?;

    if raw.seconds < MIN_TIMESTAMP_SECONDS || MAX_TIMESTAMP_SECONDS < raw.seconds {
        return Err(Error::custom("timestamp out of range"));
    }

    let datetime = OffsetDateTime::from_unix_timestamp(raw.seconds)
        .map_err(|_| Error::custom("invalid timestamp"))?
        + Duration::nanoseconds(raw.nanos.into());
    let rfc3339 = format_rfc3339(&datetime).map_err(|_| Error::custom("invalid timestamp"))?;

    serializer.serialize_str(&rfc3339)
}

fn serialize_duration<S>(
    msg: &DynamicMessage,
    serializer: S,
    _options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut raw: prost_types::Duration = msg.transcode_to().map_err(decode_to_ser_err)?;

    raw.normalize();

    let abs_seconds = raw.seconds.unsigned_abs();
    let mut abs_nanos = raw.nanos.unsigned_abs();

    if abs_seconds > MAX_DURATION_SECONDS {
        return Err(Error::custom("duration out of range"));
    }
    debug_assert!(abs_nanos <= MAX_DURATION_NANOS);

    let mut nanos_fract_digits: usize = 9;
    while nanos_fract_digits != 0 && abs_nanos % 1000 == 0 {
        abs_nanos /= 1000;
        nanos_fract_digits -= 3;
    }

    match (raw.seconds.cmp(&0), raw.nanos.cmp(&0)) {
        (_, Ordering::Equal) => serializer.collect_str(&format_args!("{}s", raw.seconds)),
        (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less) => {
            Err(Error::custom("inconsistent signs for duration"))
        }
        (Ordering::Equal | Ordering::Less, Ordering::Less) => {
            serializer.collect_str(&format_args!(
                "-{}.{:0>digits$}s",
                abs_seconds,
                abs_nanos,
                digits = nanos_fract_digits,
            ))
        }
        (Ordering::Equal | Ordering::Greater, Ordering::Greater) => {
            serializer.collect_str(&format_args!(
                "{}.{:0>digits$}s",
                abs_seconds,
                abs_nanos,
                digits = nanos_fract_digits,
            ))
        }
    }
}

fn serialize_float<S>(
    msg: &DynamicMessage,
    serializer: S,
    _options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: f32 = msg.transcode_to().map_err(decode_to_ser_err)?;

    serializer.serialize_f32(raw)
}

fn serialize_double<S>(
    msg: &DynamicMessage,
    serializer: S,
    _options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: f64 = msg.transcode_to().map_err(decode_to_ser_err)?;

    serializer.serialize_f64(raw)
}

fn serialize_int32<S>(
    msg: &DynamicMessage,
    serializer: S,
    _options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: i32 = msg.transcode_to().map_err(decode_to_ser_err)?;

    serializer.serialize_i32(raw)
}

fn serialize_int64<S>(
    msg: &DynamicMessage,
    serializer: S,
    options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: i64 = msg.transcode_to().map_err(decode_to_ser_err)?;

    if options.stringify_64_bit_integers {
        serializer.collect_str(&raw)
    } else {
        serializer.serialize_i64(raw)
    }
}

fn serialize_uint32<S>(
    msg: &DynamicMessage,
    serializer: S,
    _options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: u32 = msg.transcode_to().map_err(decode_to_ser_err)?;

    serializer.serialize_u32(raw)
}

fn serialize_uint64<S>(
    msg: &DynamicMessage,
    serializer: S,
    options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: u64 = msg.transcode_to().map_err(decode_to_ser_err)?;

    if options.stringify_64_bit_integers {
        serializer.collect_str(&raw)
    } else {
        serializer.serialize_u64(raw)
    }
}

fn serialize_bool<S>(
    msg: &DynamicMessage,
    serializer: S,
    _options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: bool = msg.transcode_to().map_err(decode_to_ser_err)?;

    serializer.serialize_bool(raw)
}

fn serialize_string<S>(
    msg: &DynamicMessage,
    serializer: S,
    _options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: String = msg.transcode_to().map_err(decode_to_ser_err)?;

    serializer.serialize_str(&raw)
}

fn serialize_bytes<S>(
    msg: &DynamicMessage,
    serializer: S,
    _options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: Vec<u8> = msg.transcode_to().map_err(decode_to_ser_err)?;

    serializer.collect_str(&Base64Display::with_config(&raw, base64::STANDARD))
}

fn serialize_field_mask<S>(
    msg: &DynamicMessage,
    serializer: S,
    _options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: prost_types::FieldMask = msg.transcode_to().map_err(decode_to_ser_err)?;

    let mut result = String::new();
    for path in raw.paths {
        if !result.is_empty() {
            result.push(',');
        }

        let mut first = true;
        for part in path.split('.') {
            if !first {
                result.push('.');
            }
            snake_case_to_camel_case(&mut result, part)
                .map_err(|()| Error::custom("cannot roundtrip field name through camelcase"))?;
            first = false;
        }
    }

    serializer.serialize_str(&result)
}

fn serialize_empty<S>(
    _: &DynamicMessage,
    serializer: S,
    _options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_map(std::iter::empty::<((), ())>())
}

fn serialize_value<S>(
    msg: &DynamicMessage,
    serializer: S,
    options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: prost_types::Value = msg.transcode_to().map_err(decode_to_ser_err)?;

    serialize_value_inner(&raw, serializer, options)
}

fn serialize_struct<S>(
    msg: &DynamicMessage,
    serializer: S,
    options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: prost_types::Struct = msg.transcode_to().map_err(decode_to_ser_err)?;

    serialize_struct_inner(&raw, serializer, options)
}

fn serialize_list<S>(
    msg: &DynamicMessage,
    serializer: S,
    options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: prost_types::ListValue = msg.transcode_to().map_err(decode_to_ser_err)?;

    serialize_list_inner(&raw, serializer, options)
}

impl<'a> Serialize for SerializeWrapper<'a, prost_types::Value> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_value_inner(self.value, serializer, self.options)
    }
}

fn serialize_value_inner<S>(
    raw: &prost_types::Value,
    serializer: S,
    options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match &raw.kind {
        None | Some(prost_types::value::Kind::NullValue(_)) => serializer.serialize_none(),
        Some(prost_types::value::Kind::BoolValue(value)) => serializer.serialize_bool(*value),
        Some(prost_types::value::Kind::NumberValue(number)) => {
            if number.is_finite() {
                serializer.serialize_f64(*number)
            } else {
                Err(Error::custom(
                    "cannot serialize non-finite double in google.protobuf.Value",
                ))
            }
        }
        Some(prost_types::value::Kind::StringValue(value)) => serializer.serialize_str(value),
        Some(prost_types::value::Kind::ListValue(value)) => {
            serialize_list_inner(value, serializer, options)
        }
        Some(prost_types::value::Kind::StructValue(value)) => {
            serialize_struct_inner(value, serializer, options)
        }
    }
}

fn serialize_struct_inner<S>(
    raw: &prost_types::Struct,
    serializer: S,
    options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(raw.fields.len()))?;
    for (key, value) in &raw.fields {
        map.serialize_entry(key, &SerializeWrapper { value, options })?;
    }
    map.end()
}

fn serialize_list_inner<S>(
    raw: &prost_types::ListValue,
    serializer: S,
    options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut list = serializer.serialize_seq(Some(raw.values.len()))?;
    for value in &raw.values {
        list.serialize_element(&SerializeWrapper { value, options })?;
    }
    list.end()
}

fn decode_to_ser_err<E>(err: DecodeError) -> E
where
    E: Error,
{
    Error::custom(format!("error decoding: {}", err))
}

fn format_rfc3339(date_time: &OffsetDateTime) -> Result<String, time::error::Format> {
    use time::format_description::{modifier::*, Component, FormatItem};

    debug_assert_eq!(date_time.offset(), UtcOffset::UTC);

    static PREFIX: &[FormatItem] = &[
        FormatItem::Component(Component::Year(Year::default())),
        FormatItem::Literal(b"-"),
        FormatItem::Component(Component::Month(Month::default())),
        FormatItem::Literal(b"-"),
        FormatItem::Component(Component::Day(Day::default())),
        FormatItem::Literal(b"T"),
        FormatItem::Component(Component::Hour(Hour::default())),
        FormatItem::Literal(b":"),
        FormatItem::Component(Component::Minute(Minute::default())),
        FormatItem::Literal(b":"),
        FormatItem::Component(Component::Second(Second::default())),
    ];

    let nanos = date_time.nanosecond();
    if nanos == 0 {
        let format_desc = [FormatItem::Compound(PREFIX), FormatItem::Literal(b"Z")];
        date_time.format(format_desc.as_ref())
    } else if nanos % 1_000_000 == 0 {
        let format_desc = [
            FormatItem::Compound(PREFIX),
            FormatItem::Literal(b"."),
            FormatItem::Component(Component::Subsecond({
                let mut subsec = Subsecond::default();
                subsec.digits = SubsecondDigits::Three;
                subsec
            })),
            FormatItem::Literal(b"Z"),
        ];
        date_time.format(format_desc.as_ref())
    } else if nanos % 1_000 == 0 {
        let format_desc = [
            FormatItem::Compound(PREFIX),
            FormatItem::Literal(b"."),
            FormatItem::Component(Component::Subsecond({
                let mut subsec = Subsecond::default();
                subsec.digits = SubsecondDigits::Six;
                subsec
            })),
            FormatItem::Literal(b"Z"),
        ];
        date_time.format(format_desc.as_ref())
    } else {
        let format_desc = [
            FormatItem::Compound(PREFIX),
            FormatItem::Literal(b"."),
            FormatItem::Component(Component::Subsecond({
                let mut subsec = Subsecond::default();
                subsec.digits = SubsecondDigits::Nine;
                subsec
            })),
            FormatItem::Literal(b"Z"),
        ];
        date_time.format(format_desc.as_ref())
    }
}
