use base64::display::Base64Display;
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

use crate::{
    descriptor::{Kind, MAP_ENTRY_VALUE_NUMBER},
    dynamic::{DynamicMessage, DynamicMessageField, MapKey, Value},
};

impl Serialize for DynamicMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = self.fields.values().filter(|v| v.is_populated()).count();
        let mut map = serializer.serialize_map(Some(len))?;
        for field in self.fields.values() {
            if field.is_populated() {
                map.serialize_entry(field.desc.json_name(), field)?;
            }
        }
        map.end()
    }
}

impl Serialize for DynamicMessageField {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // These null cases shouldn't be hit since we're only serializing populated fields currently,
        // but we may want an option to include unpopulated fields in future.
        let value = match &self.value {
            None => return serializer.serialize_none(),
            Some(value) => value,
        };

        SerializeValue(value, &self.desc.kind()).serialize(serializer)
    }
}

struct SerializeValue<'a>(&'a Value, &'a Kind);
struct SerializeMapKey<'a>(&'a MapKey);

impl<'a> Serialize for SerializeValue<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            Value::Bool(value) => serializer.serialize_bool(*value),
            Value::I32(value) => serializer.serialize_i32(*value),
            Value::I64(value) => serializer.collect_str(value),
            Value::U32(value) => serializer.serialize_u32(*value),
            Value::U64(value) => serializer.collect_str(value),
            Value::F32(value) => serializer.serialize_f32(*value),
            Value::F64(value) => serializer.serialize_f64(*value),
            Value::String(value) => serializer.serialize_str(value),
            Value::Bytes(value) => {
                serializer.collect_str(&Base64Display::with_config(value, base64::STANDARD))
            }
            Value::EnumNumber(number) => {
                if let Kind::Enum(enum_ty) = self.1 {
                    if let Some(enum_value) = enum_ty.get_value(*number) {
                        serializer.serialize_str(enum_value.name())
                    } else {
                        serializer.serialize_i32(*number)
                    }
                } else {
                    panic!(
                        "mismatch between DynamicMessage value {:?} and type {:?}",
                        self.0, self.1
                    )
                }
            }
            Value::Message(message) => message.serialize(serializer),
            Value::List(values) => {
                let mut list = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    list.serialize_element(&SerializeValue(value, self.1))?;
                }
                list.end()
            }
            Value::Map(values) => {
                let value_kind = match self.1 {
                    Kind::Message(message) if message.is_map_entry() => {
                        message.get_field(MAP_ENTRY_VALUE_NUMBER).unwrap().kind()
                    }
                    _ => panic!(
                        "mismatch between DynamicMessage value {:?} and type {:?}",
                        self.0, self.1
                    ),
                };

                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (key, value) in values {
                    map.serialize_entry(
                        &SerializeMapKey(key),
                        &SerializeValue(value, &value_kind),
                    )?;
                }
                map.end()
            }
        }
    }
}

impl<'a> Serialize for SerializeMapKey<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            MapKey::Bool(value) => serializer.collect_str(value),
            MapKey::I32(value) => serializer.collect_str(value),
            MapKey::I64(value) => serializer.collect_str(value),
            MapKey::U32(value) => serializer.collect_str(value),
            MapKey::U64(value) => serializer.collect_str(value),
            MapKey::String(value) => serializer.serialize_str(value),
        }
    }
}
