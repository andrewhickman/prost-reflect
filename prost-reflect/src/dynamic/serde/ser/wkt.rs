use base64::{display::Base64Display, prelude::BASE64_STANDARD};
use prost::{DecodeError, Message};
use serde::ser::{Error, Serialize, SerializeMap, SerializeSeq, Serializer};

use crate::{
    dynamic::{
        serde::{
            case::snake_case_to_camel_case, check_duration, check_timestamp, is_well_known_type,
            SerializeOptions,
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
            .parent_pool()
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
    let timestamp: prost_types::Timestamp = msg.transcode_to().map_err(decode_to_ser_err)?;

    check_timestamp(&timestamp).map_err(Error::custom)?;

    serializer.collect_str(&timestamp)
}

fn serialize_duration<S>(
    msg: &DynamicMessage,
    serializer: S,
    _options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let duration: prost_types::Duration = msg.transcode_to().map_err(decode_to_ser_err)?;

    check_duration(&duration).map_err(Error::custom)?;

    serializer.collect_str(&duration)
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

    serializer.collect_str(&Base64Display::new(&raw, &BASE64_STANDARD))
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
