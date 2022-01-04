mod wkt;

use std::borrow::Cow;

use base64::display::Base64Display;

use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

use crate::{
    descriptor::Kind,
    dynamic::{
        field::{DynamicMessageField, FieldDescriptorLike},
        serde::SerializeOptions,
        DynamicMessage, MapKey, Value,
    },
    ReflectMessage,
};

struct SerializeWrapper<'a, T> {
    value: &'a T,
    options: &'a SerializeOptions,
}

pub(super) fn serialize_message<S>(
    message: &DynamicMessage,
    serializer: S,
    options: &SerializeOptions,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    SerializeWrapper {
        value: message,
        options,
    }
    .serialize(serializer)
}

impl<'a> Serialize for SerializeWrapper<'a, DynamicMessage> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let message_desc = self.value.descriptor();
        if let Some(serialize) = wkt::get_well_known_type_serializer(message_desc.full_name()) {
            serialize(self.value, serializer, self.options)
        } else {
            let mut map = serializer.serialize_map(None)?;
            serialize_dynamic_message_fields(&mut map, self.value, self.options)?;
            map.end()
        }
    }
}

fn serialize_dynamic_message_fields<S>(
    map: &mut S,
    value: &DynamicMessage,
    options: &SerializeOptions,
) -> Result<(), S::Error>
where
    S: SerializeMap,
{
    if options.skip_default_fields {
        for field in value.fields.fields.values() {
            if field.has() {
                let name = if options.use_proto_field_name {
                    field.desc.name()
                } else {
                    field.desc.json_name()
                };

                map.serialize_entry(
                    name,
                    &SerializeWrapper {
                        value: field,
                        options,
                    },
                )?;
            }
        }
    } else {
        for field_desc in value.desc.fields() {
            let field = value
                .fields
                .fields
                .get(&field_desc.number())
                .map(Cow::Borrowed)
                .unwrap_or_else(|| Cow::Owned(DynamicMessageField::new(field_desc.clone())));

            if field.value.is_some() {
                let name = if options.use_proto_field_name {
                    field_desc.name()
                } else {
                    field_desc.json_name()
                };

                map.serialize_entry(
                    name,
                    &SerializeWrapper {
                        value: field.as_ref(),
                        options,
                    },
                )?;
            }
        }
    }

    if let Some(extensions) = value.extension_fields() {
        for field in extensions.fields.values() {
            if field.value.is_some() {
                let name = field.desc.json_name();

                map.serialize_entry(
                    name,
                    &SerializeWrapper {
                        value: field,
                        options,
                    },
                )?;
            }
        }
    }
    Ok(())
}

impl<'a, T> Serialize for SerializeWrapper<'a, DynamicMessageField<T>>
where
    T: FieldDescriptorLike,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = self
            .value
            .value
            .as_ref()
            .expect("cannot serialize unpopulated field");

        SerializeWrapper {
            value: &ValueAndKind {
                value,
                kind: &self.value.desc.kind(),
            },
            options: self.options,
        }
        .serialize(serializer)
    }
}

struct ValueAndKind<'a> {
    value: &'a Value,
    kind: &'a Kind,
}

impl<'a> Serialize for SerializeWrapper<'a, ValueAndKind<'a>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Special cased well known types
        match self.value.value {
            Value::Bool(value) => serializer.serialize_bool(*value),
            Value::I32(value) => serializer.serialize_i32(*value),
            Value::I64(value) => {
                if self.options.stringify_64_bit_integers {
                    serializer.collect_str(value)
                } else {
                    serializer.serialize_i64(*value)
                }
            }
            Value::U32(value) => serializer.serialize_u32(*value),
            Value::U64(value) => {
                if self.options.stringify_64_bit_integers {
                    serializer.collect_str(value)
                } else {
                    serializer.serialize_u64(*value)
                }
            }
            Value::F32(value) => {
                if value.is_finite() {
                    serializer.serialize_f32(*value)
                } else if *value == f32::INFINITY {
                    serializer.serialize_str("Infinity")
                } else if *value == f32::NEG_INFINITY {
                    serializer.serialize_str("-Infinity")
                } else {
                    debug_assert!(value.is_nan());
                    serializer.serialize_str("NaN")
                }
            }
            Value::F64(value) => {
                if value.is_finite() {
                    serializer.serialize_f64(*value)
                } else if *value == f64::INFINITY {
                    serializer.serialize_str("Infinity")
                } else if *value == f64::NEG_INFINITY {
                    serializer.serialize_str("-Infinity")
                } else {
                    debug_assert!(value.is_nan());
                    serializer.serialize_str("NaN")
                }
            }
            Value::String(value) => serializer.serialize_str(value),
            Value::Bytes(value) => {
                serializer.collect_str(&Base64Display::with_config(value, base64::STANDARD))
            }
            Value::EnumNumber(number) => {
                let enum_ty = match self.value.kind {
                    Kind::Enum(enum_ty) => enum_ty,
                    _ => panic!(
                        "mismatch between DynamicMessage value {:?} and type {:?}",
                        self.value.value, self.value.kind
                    ),
                };

                if enum_ty.full_name() == "google.protobuf.NullValue" {
                    serializer.serialize_none()
                } else if self.options.use_enum_numbers {
                    serializer.serialize_i32(*number)
                } else if let Some(enum_value) = enum_ty.get_value(*number) {
                    serializer.serialize_str(enum_value.name())
                } else {
                    serializer.serialize_i32(*number)
                }
            }
            Value::Message(message) => message.serialize_with_options(serializer, self.options),
            Value::List(values) => {
                let mut list = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    list.serialize_element(&SerializeWrapper {
                        value: &ValueAndKind {
                            value,
                            kind: self.value.kind,
                        },
                        options: self.options,
                    })?;
                }
                list.end()
            }
            Value::Map(values) => {
                let value_kind = match self.value.kind {
                    Kind::Message(message) if message.is_map_entry() => {
                        message.map_entry_value_field().kind()
                    }
                    _ => panic!(
                        "mismatch between DynamicMessage value {:?} and type {:?}",
                        self.value.value, self.value.kind
                    ),
                };

                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (key, value) in values {
                    map.serialize_entry(
                        &SerializeWrapper {
                            value: key,
                            options: self.options,
                        },
                        &SerializeWrapper {
                            value: &ValueAndKind {
                                value,
                                kind: &value_kind,
                            },
                            options: self.options,
                        },
                    )?;
                }
                map.end()
            }
        }
    }
}

impl<'a> Serialize for SerializeWrapper<'a, MapKey> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.value {
            MapKey::Bool(value) => serializer.collect_str(value),
            MapKey::I32(value) => serializer.collect_str(value),
            MapKey::I64(value) => serializer.collect_str(value),
            MapKey::U32(value) => serializer.collect_str(value),
            MapKey::U64(value) => serializer.collect_str(value),
            MapKey::String(value) => serializer.serialize_str(value),
        }
    }
}
