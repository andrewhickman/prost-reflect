use prost::{
    bytes::{Buf, BufMut},
    encoding::{DecodeContext, WireType},
    DecodeError, Message,
};

use crate::{
    descriptor::{FieldDescriptor, Kind, MAP_ENTRY_KEY_NUMBER, MAP_ENTRY_VALUE_NUMBER},
    DynamicMessage, MapKey, Value,
};

use super::{
    fields::{FieldDescriptorLike, ValueAndDescriptor},
    unknown::UnknownField,
};

impl Message for DynamicMessage {
    fn encode_raw(&self, buf: &mut impl BufMut)
    where
        Self: Sized,
    {
        for field in self.fields.iter(&self.desc, false, false) {
            match field {
                ValueAndDescriptor::Field(value, field_desc) => {
                    value.encode_field(&field_desc, buf)
                }
                ValueAndDescriptor::Extension(value, extension_desc) => {
                    value.encode_field(&extension_desc, buf)
                }
                ValueAndDescriptor::Unknown(unknowns) => unknowns.encode_raw(buf),
            }
        }
    }

    fn merge_field(
        &mut self,
        number: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        Self: Sized,
    {
        if let Some(field_desc) = self.desc.get_field(number) {
            self.get_field_mut(&field_desc)
                .merge_field(&field_desc, wire_type, buf, ctx)
        } else if let Some(extension_desc) = self.desc.get_extension(number) {
            self.get_extension_mut(&extension_desc).merge_field(
                &extension_desc,
                wire_type,
                buf,
                ctx,
            )
        } else {
            let field = UnknownField::decode_value(number, wire_type, buf, ctx)?;
            self.fields.add_unknown(number, field);
            Ok(())
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        for field in self.fields.iter(&self.desc, false, false) {
            match field {
                ValueAndDescriptor::Field(value, field_desc) => {
                    len += value.encoded_len(&field_desc);
                }
                ValueAndDescriptor::Extension(value, extension_desc) => {
                    len += value.encoded_len(&extension_desc);
                }
                ValueAndDescriptor::Unknown(unknowns) => len += unknowns.encoded_len(),
            }
        }
        len
    }

    fn clear(&mut self) {
        self.fields.clear_all();
    }
}

impl Value {
    pub(super) fn encode_field<B>(&self, field_desc: &impl FieldDescriptorLike, buf: &mut B)
    where
        B: BufMut,
    {
        if !field_desc.supports_presence() && field_desc.is_default_value(self) {
            return;
        }

        let number = field_desc.number();
        match (self, field_desc.kind()) {
            (Value::Bool(value), Kind::Bool) => prost::encoding::bool::encode(number, value, buf),
            (Value::I32(value), Kind::Int32) => prost::encoding::int32::encode(number, value, buf),
            (Value::I32(value), Kind::Sint32) => {
                prost::encoding::sint32::encode(number, value, buf)
            }
            (Value::I32(value), Kind::Sfixed32) => {
                prost::encoding::sfixed32::encode(number, value, buf)
            }
            (Value::I64(value), Kind::Int64) => prost::encoding::int64::encode(number, value, buf),
            (Value::I64(value), Kind::Sint64) => {
                prost::encoding::sint64::encode(number, value, buf)
            }
            (Value::I64(value), Kind::Sfixed64) => {
                prost::encoding::sfixed64::encode(number, value, buf)
            }
            (Value::U32(value), Kind::Uint32) => {
                prost::encoding::uint32::encode(number, value, buf)
            }
            (Value::U32(value), Kind::Fixed32) => {
                prost::encoding::fixed32::encode(number, value, buf)
            }
            (Value::U64(value), Kind::Uint64) => {
                prost::encoding::uint64::encode(number, value, buf)
            }
            (Value::U64(value), Kind::Fixed64) => {
                prost::encoding::fixed64::encode(number, value, buf)
            }
            (Value::F32(value), Kind::Float) => prost::encoding::float::encode(number, value, buf),
            (Value::F64(value), Kind::Double) => {
                prost::encoding::double::encode(number, value, buf)
            }
            (Value::String(value), Kind::String) => {
                prost::encoding::string::encode(number, value, buf)
            }
            (Value::Bytes(value), Kind::Bytes) => {
                prost::encoding::bytes::encode(number, value, buf)
            }
            (Value::EnumNumber(value), Kind::Enum(_)) => {
                prost::encoding::int32::encode(number, value, buf)
            }
            (Value::Message(message), Kind::Message(_)) => {
                if field_desc.is_group() {
                    prost::encoding::group::encode(number, message, buf)
                } else {
                    prost::encoding::message::encode(number, message, buf)
                }
            }
            (Value::List(values), _) if field_desc.is_list() => {
                if field_desc.is_packed() {
                    match field_desc.kind() {
                        Kind::Enum(_) => encode_packed_list(
                            number,
                            values
                                .iter()
                                .map(|v| v.as_enum_number().expect("expected enum number")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Double => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_f64().expect("expected double")),
                            buf,
                            |v, b| b.put_f64_le(v),
                            |_| 8,
                        ),
                        Kind::Float => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_f32().expect("expected float")),
                            buf,
                            |v, b| b.put_f32_le(v),
                            |_| 4,
                        ),
                        Kind::Int32 => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Int64 => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Uint32 => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Uint64 => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v, b),
                            prost::encoding::encoded_len_varint,
                        ),
                        Kind::Sint32 => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            buf,
                            |v, b| prost::encoding::encode_varint(from_sint32(v) as u64, b),
                            |v| prost::encoding::encoded_len_varint(from_sint32(v) as u64),
                        ),
                        Kind::Sint64 => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            buf,
                            |v, b| prost::encoding::encode_varint(from_sint64(v), b),
                            |v| prost::encoding::encoded_len_varint(from_sint64(v)),
                        ),
                        Kind::Fixed32 => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            buf,
                            |v, b| b.put_u32_le(v),
                            |_| 4,
                        ),
                        Kind::Fixed64 => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            buf,
                            |v, b| b.put_u64_le(v),
                            |_| 8,
                        ),
                        Kind::Sfixed32 => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            buf,
                            |v, b| b.put_i32_le(v),
                            |_| 4,
                        ),
                        Kind::Sfixed64 => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            buf,
                            |v, b| b.put_i64_le(v),
                            |_| 8,
                        ),
                        Kind::Bool => encode_packed_list(
                            number,
                            values.iter().map(|v| v.as_bool().expect("expected bool")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        _ => panic!("invalid type for packed field in DynamicMessage"),
                    }
                } else {
                    for value in values {
                        value.encode_field(field_desc, buf);
                    }
                }
            }
            (Value::Map(values), Kind::Message(map_entry)) if field_desc.is_map() => {
                let key_desc = map_entry.get_field(MAP_ENTRY_KEY_NUMBER).unwrap();
                let value_desc = map_entry.get_field(MAP_ENTRY_VALUE_NUMBER).unwrap();

                for (key, value) in values {
                    let len = key.encoded_len(&key_desc) + value.encoded_len(&value_desc);

                    prost::encoding::encode_key(number, WireType::LengthDelimited, buf);
                    prost::encoding::encode_varint(len as u64, buf);

                    key.encode_field(&key_desc, buf);
                    value.encode_field(&value_desc, buf);
                }
            }
            (value, ty) => panic!(
                "mismatch between DynamicMessage value {:?} and type {:?}",
                value, ty
            ),
        }
    }

    pub(super) fn merge_field<B>(
        &mut self,
        field_desc: &impl FieldDescriptorLike,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
    {
        match (self, field_desc.kind()) {
            (Value::Bool(value), Kind::Bool) => {
                prost::encoding::bool::merge(wire_type, value, buf, ctx)
            }
            (Value::I32(value), Kind::Int32) => {
                prost::encoding::int32::merge(wire_type, value, buf, ctx)
            }
            (Value::I32(value), Kind::Sint32) => {
                prost::encoding::sint32::merge(wire_type, value, buf, ctx)
            }
            (Value::I32(value), Kind::Sfixed32) => {
                prost::encoding::sfixed32::merge(wire_type, value, buf, ctx)
            }
            (Value::I64(value), Kind::Int64) => {
                prost::encoding::int64::merge(wire_type, value, buf, ctx)
            }
            (Value::I64(value), Kind::Sint64) => {
                prost::encoding::sint64::merge(wire_type, value, buf, ctx)
            }
            (Value::I64(value), Kind::Sfixed64) => {
                prost::encoding::sfixed64::merge(wire_type, value, buf, ctx)
            }
            (Value::U32(value), Kind::Uint32) => {
                prost::encoding::uint32::merge(wire_type, value, buf, ctx)
            }
            (Value::U32(value), Kind::Fixed32) => {
                prost::encoding::fixed32::merge(wire_type, value, buf, ctx)
            }
            (Value::U64(value), Kind::Uint64) => {
                prost::encoding::uint64::merge(wire_type, value, buf, ctx)
            }
            (Value::U64(value), Kind::Fixed64) => {
                prost::encoding::fixed64::merge(wire_type, value, buf, ctx)
            }
            (Value::F32(value), Kind::Float) => {
                prost::encoding::float::merge(wire_type, value, buf, ctx)
            }
            (Value::F64(value), Kind::Double) => {
                prost::encoding::double::merge(wire_type, value, buf, ctx)
            }
            (Value::String(value), Kind::String) => {
                prost::encoding::string::merge(wire_type, value, buf, ctx)
            }
            (Value::Bytes(value), Kind::Bytes) => {
                prost::encoding::bytes::merge(wire_type, value, buf, ctx)
            }
            (Value::EnumNumber(value), Kind::Enum(_)) => {
                prost::encoding::int32::merge(wire_type, value, buf, ctx)
            }
            (Value::Message(message), Kind::Message(_)) => {
                if field_desc.is_group() {
                    prost::encoding::group::merge(field_desc.number(), wire_type, message, buf, ctx)
                } else {
                    prost::encoding::message::merge(wire_type, message, buf, ctx)
                }
            }
            (Value::List(values), field_kind) if field_desc.is_list() => {
                if wire_type == WireType::LengthDelimited && field_desc.is_packable() {
                    prost::encoding::merge_loop(values, buf, ctx, |values, buf, ctx| {
                        let mut value = Value::default_value(&field_kind);
                        value.merge_field(field_desc, field_kind.wire_type(), buf, ctx)?;
                        values.push(value);
                        Ok(())
                    })
                } else {
                    let mut value = Value::default_value(&field_kind);
                    value.merge_field(field_desc, wire_type, buf, ctx)?;
                    values.push(value);
                    Ok(())
                }
            }
            (Value::Map(values), Kind::Message(map_entry)) if field_desc.is_map() => {
                let key_desc = map_entry.get_field(MAP_ENTRY_KEY_NUMBER).unwrap();
                let value_desc = map_entry.get_field(MAP_ENTRY_VALUE_NUMBER).unwrap();

                let mut key = MapKey::default_value(&key_desc.kind());
                let mut value = Value::default_value_for_field(&value_desc);
                prost::encoding::merge_loop(
                    &mut (&mut key, &mut value),
                    buf,
                    ctx,
                    |(key, value), buf, ctx| {
                        let (number, wire_type) = prost::encoding::decode_key(buf)?;
                        match number {
                            MAP_ENTRY_KEY_NUMBER => key.merge_field(&key_desc, wire_type, buf, ctx),
                            MAP_ENTRY_VALUE_NUMBER => {
                                value.merge_field(&value_desc, wire_type, buf, ctx)
                            }
                            _ => prost::encoding::skip_field(wire_type, number, buf, ctx),
                        }
                    },
                )?;
                values.insert(key, value);

                Ok(())
            }
            (value, ty) => panic!(
                "mismatch between DynamicMessage value {:?} and type {:?}",
                value, ty
            ),
        }
    }

    pub(super) fn encoded_len(&self, field_desc: &impl FieldDescriptorLike) -> usize {
        if !field_desc.supports_presence() && field_desc.is_default_value(self) {
            return 0;
        }

        let number = field_desc.number();
        match (self, field_desc.kind()) {
            (Value::Bool(value), Kind::Bool) => prost::encoding::bool::encoded_len(number, value),
            (Value::I32(value), Kind::Int32) => prost::encoding::int32::encoded_len(number, value),
            (Value::I32(value), Kind::Sint32) => {
                prost::encoding::sint32::encoded_len(number, value)
            }
            (Value::I32(value), Kind::Sfixed32) => {
                prost::encoding::sfixed32::encoded_len(number, value)
            }
            (Value::I64(value), Kind::Int64) => prost::encoding::int64::encoded_len(number, value),
            (Value::I64(value), Kind::Sint64) => {
                prost::encoding::sint64::encoded_len(number, value)
            }
            (Value::I64(value), Kind::Sfixed64) => {
                prost::encoding::sfixed64::encoded_len(number, value)
            }
            (Value::U32(value), Kind::Uint32) => {
                prost::encoding::uint32::encoded_len(number, value)
            }
            (Value::U32(value), Kind::Fixed32) => {
                prost::encoding::fixed32::encoded_len(number, value)
            }
            (Value::U64(value), Kind::Uint64) => {
                prost::encoding::uint64::encoded_len(number, value)
            }
            (Value::U64(value), Kind::Fixed64) => {
                prost::encoding::fixed64::encoded_len(number, value)
            }
            (Value::F32(value), Kind::Float) => prost::encoding::float::encoded_len(number, value),
            (Value::F64(value), Kind::Double) => {
                prost::encoding::double::encoded_len(number, value)
            }
            (Value::String(value), Kind::String) => {
                prost::encoding::string::encoded_len(number, value)
            }
            (Value::Bytes(value), Kind::Bytes) => {
                prost::encoding::bytes::encoded_len(number, value)
            }
            (Value::EnumNumber(value), Kind::Enum(_)) => {
                prost::encoding::int32::encoded_len(number, value)
            }
            (Value::Message(message), Kind::Message(_)) => {
                if field_desc.is_group() {
                    prost::encoding::group::encoded_len(number, message)
                } else {
                    prost::encoding::message::encoded_len(number, message)
                }
            }
            (Value::List(values), _) if field_desc.is_list() => {
                if field_desc.is_packed() {
                    match field_desc.kind() {
                        Kind::Enum(_) => packed_list_encoded_len(
                            number,
                            values
                                .iter()
                                .map(|v| v.as_enum_number().expect("expected enum number")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Double => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_f64().expect("expected double")),
                            |_| 8,
                        ),
                        Kind::Float => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_f32().expect("expected float")),
                            |_| 4,
                        ),
                        Kind::Int32 => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Int64 => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Uint32 => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Uint64 => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            prost::encoding::encoded_len_varint,
                        ),
                        Kind::Sint32 => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            |v| prost::encoding::encoded_len_varint(from_sint32(v) as u64),
                        ),
                        Kind::Sint64 => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            |v| prost::encoding::encoded_len_varint(from_sint64(v)),
                        ),
                        Kind::Fixed32 => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            |_| 4,
                        ),
                        Kind::Fixed64 => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            |_| 8,
                        ),
                        Kind::Sfixed32 => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            |_| 4,
                        ),
                        Kind::Sfixed64 => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            |_| 8,
                        ),
                        Kind::Bool => packed_list_encoded_len(
                            number,
                            values.iter().map(|v| v.as_bool().expect("expected bool")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        _ => panic!("invalid type for packed field in DynamicMessage"),
                    }
                } else {
                    values
                        .iter()
                        .map(|value| value.encoded_len(field_desc))
                        .sum()
                }
            }
            (Value::Map(values), Kind::Message(map_entry)) if field_desc.is_map() => {
                let key_desc = map_entry.map_entry_key_field();
                let value_desc = map_entry.map_entry_value_field();

                let key_len = prost::encoding::key_len(number);
                values
                    .iter()
                    .map(|(key, value)| {
                        let len = key.encoded_len(&key_desc) + value.encoded_len(&value_desc);

                        key_len + prost::encoding::encoded_len_varint(len as u64) + len
                    })
                    .sum::<usize>()
            }
            (value, ty) => panic!(
                "mismatch between DynamicMessage value {:?} and type {:?}",
                value, ty
            ),
        }
    }
}

impl MapKey {
    fn encode_field<B>(&self, field_desc: &FieldDescriptor, buf: &mut B)
    where
        B: BufMut,
    {
        if !field_desc.supports_presence() && self.is_default(&field_desc.kind()) {
            return;
        }

        let number = field_desc.number();
        match (self, field_desc.kind()) {
            (MapKey::Bool(value), Kind::Bool) => prost::encoding::bool::encode(number, value, buf),
            (MapKey::I32(value), Kind::Int32) => prost::encoding::int32::encode(number, value, buf),
            (MapKey::I32(value), Kind::Sint32) => {
                prost::encoding::sint32::encode(number, value, buf)
            }
            (MapKey::I32(value), Kind::Sfixed32) => {
                prost::encoding::sfixed32::encode(number, value, buf)
            }
            (MapKey::I64(value), Kind::Int64) => prost::encoding::int64::encode(number, value, buf),
            (MapKey::I64(value), Kind::Sint64) => {
                prost::encoding::sint64::encode(number, value, buf)
            }
            (MapKey::I64(value), Kind::Sfixed64) => {
                prost::encoding::sfixed64::encode(number, value, buf)
            }
            (MapKey::U32(value), Kind::Uint32) => {
                prost::encoding::uint32::encode(number, value, buf)
            }
            (MapKey::U32(value), Kind::Fixed32) => {
                prost::encoding::fixed32::encode(number, value, buf)
            }
            (MapKey::U64(value), Kind::Uint64) => {
                prost::encoding::uint64::encode(number, value, buf)
            }
            (MapKey::U64(value), Kind::Fixed64) => {
                prost::encoding::fixed64::encode(number, value, buf)
            }
            (MapKey::String(value), Kind::String) => {
                prost::encoding::string::encode(number, value, buf)
            }
            (value, ty) => panic!(
                "mismatch between DynamicMessage value {:?} and type {:?}",
                value, ty
            ),
        }
    }

    fn merge_field<B>(
        &mut self,
        field_desc: &FieldDescriptor,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
    {
        match (self, field_desc.kind()) {
            (MapKey::Bool(value), Kind::Bool) => {
                prost::encoding::bool::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I32(value), Kind::Int32) => {
                prost::encoding::int32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I32(value), Kind::Sint32) => {
                prost::encoding::sint32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I32(value), Kind::Sfixed32) => {
                prost::encoding::sfixed32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I64(value), Kind::Int64) => {
                prost::encoding::int64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I64(value), Kind::Sint64) => {
                prost::encoding::sint64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I64(value), Kind::Sfixed64) => {
                prost::encoding::sfixed64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::U32(value), Kind::Uint32) => {
                prost::encoding::uint32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::U32(value), Kind::Fixed32) => {
                prost::encoding::fixed32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::U64(value), Kind::Uint64) => {
                prost::encoding::uint64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::U64(value), Kind::Fixed64) => {
                prost::encoding::fixed64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::String(value), Kind::String) => {
                prost::encoding::string::merge(wire_type, value, buf, ctx)
            }
            (value, ty) => panic!(
                "mismatch between DynamicMessage value {:?} and type {:?}",
                value, ty
            ),
        }
    }

    fn encoded_len(&self, field_desc: &FieldDescriptor) -> usize {
        if !field_desc.supports_presence() && self.is_default(&field_desc.kind()) {
            return 0;
        }

        let number = field_desc.number();
        match (self, field_desc.kind()) {
            (MapKey::Bool(value), Kind::Bool) => prost::encoding::bool::encoded_len(number, value),
            (MapKey::I32(value), Kind::Int32) => prost::encoding::int32::encoded_len(number, value),
            (MapKey::I32(value), Kind::Sint32) => {
                prost::encoding::sint32::encoded_len(number, value)
            }
            (MapKey::I32(value), Kind::Sfixed32) => {
                prost::encoding::sfixed32::encoded_len(number, value)
            }
            (MapKey::I64(value), Kind::Int64) => prost::encoding::int64::encoded_len(number, value),
            (MapKey::I64(value), Kind::Sint64) => {
                prost::encoding::sint64::encoded_len(number, value)
            }
            (MapKey::I64(value), Kind::Sfixed64) => {
                prost::encoding::sfixed64::encoded_len(number, value)
            }
            (MapKey::U32(value), Kind::Uint32) => {
                prost::encoding::uint32::encoded_len(number, value)
            }
            (MapKey::U32(value), Kind::Fixed32) => {
                prost::encoding::fixed32::encoded_len(number, value)
            }
            (MapKey::U64(value), Kind::Uint64) => {
                prost::encoding::uint64::encoded_len(number, value)
            }
            (MapKey::U64(value), Kind::Fixed64) => {
                prost::encoding::fixed64::encoded_len(number, value)
            }
            (MapKey::String(value), Kind::String) => {
                prost::encoding::string::encoded_len(number, value)
            }
            (value, ty) => panic!(
                "mismatch between DynamicMessage value {:?} and type {:?}",
                value, ty
            ),
        }
    }
}

fn encode_packed_list<T, I, B, E, L>(number: u32, iter: I, buf: &mut B, encode: E, encoded_len: L)
where
    I: IntoIterator<Item = T> + Clone,
    B: BufMut,
    E: Fn(T, &mut B),
    L: Fn(T) -> usize,
{
    prost::encoding::encode_key(number, WireType::LengthDelimited, buf);
    let len: usize = iter.clone().into_iter().map(encoded_len).sum();
    prost::encoding::encode_varint(len as u64, buf);

    for value in iter {
        encode(value, buf);
    }
}

fn packed_list_encoded_len<T, I, L>(number: u32, iter: I, encoded_len: L) -> usize
where
    I: IntoIterator<Item = T>,
    L: Fn(T) -> usize,
{
    let len: usize = iter.into_iter().map(encoded_len).sum();
    prost::encoding::key_len(number) + prost::encoding::encoded_len_varint(len as u64) + len
}

fn from_sint32(value: i32) -> u32 {
    ((value << 1) ^ (value >> 31)) as u32
}
// fn to_sint32(value: u32) -> i32 {
//     ((value >> 1) as i32) ^ (-((value & 1) as i32))
// }
fn from_sint64(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}
// fn to_sint64(value: u64) -> i64 {
//     ((value >> 1) as i64) ^ (-((value & 1) as i64))
// }
