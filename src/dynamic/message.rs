use prost::{
    bytes::{Buf, BufMut},
    encoding::{DecodeContext, WireType},
    DecodeError, Message,
};

use crate::{
    descriptor::{Kind, MAP_ENTRY_KEY_TAG, MAP_ENTRY_VALUE_TAG},
    DynamicMessage, FieldDescriptor, MapKey, Value,
};

impl Message for DynamicMessage {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        for field in self.fields.values() {
            if let Some(value) = &field.value {
                value.encode_field(&field.desc, buf);
            }
        }
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        if let Some(field) = self.fields.get_mut(&tag) {
            let field_value = &mut field.value;
            let field_desc = &field.desc;
            field_value
                .get_or_insert_with(|| Value::default_value(field_desc))
                .merge_field(field_desc, wire_type, buf, ctx)
        } else {
            prost::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        for field in self.fields.values() {
            if let Some(value) = &field.value {
                len += value.encoded_len(&field.desc);
            }
        }
        len
    }

    fn clear(&mut self) {
        for value in self.fields.values_mut() {
            value.clear();
        }
    }
}

impl Value {
    fn encode_field<B>(&self, field_desc: &FieldDescriptor, buf: &mut B)
    where
        B: BufMut,
    {
        if !field_desc.supports_presence() && self.is_default(field_desc) {
            return;
        }

        let tag = field_desc.tag();
        match (self, field_desc.kind()) {
            (Value::Bool(value), Kind::Bool) => prost::encoding::bool::encode(tag, value, buf),
            (Value::I32(value), Kind::Int32) => prost::encoding::int32::encode(tag, value, buf),
            (Value::I32(value), Kind::Sint32) => prost::encoding::sint32::encode(tag, value, buf),
            (Value::I32(value), Kind::Sfixed32) => {
                prost::encoding::sfixed32::encode(tag, value, buf)
            }
            (Value::I64(value), Kind::Int64) => prost::encoding::int64::encode(tag, value, buf),
            (Value::I64(value), Kind::Sint64) => prost::encoding::sint64::encode(tag, value, buf),
            (Value::I64(value), Kind::Sfixed64) => {
                prost::encoding::sfixed64::encode(tag, value, buf)
            }
            (Value::U32(value), Kind::Uint32) => prost::encoding::uint32::encode(tag, value, buf),
            (Value::U32(value), Kind::Fixed32) => prost::encoding::fixed32::encode(tag, value, buf),
            (Value::U64(value), Kind::Uint64) => prost::encoding::uint64::encode(tag, value, buf),
            (Value::U64(value), Kind::Fixed64) => prost::encoding::fixed64::encode(tag, value, buf),
            (Value::F32(value), Kind::Float) => prost::encoding::float::encode(tag, value, buf),
            (Value::F64(value), Kind::Double) => prost::encoding::double::encode(tag, value, buf),
            (Value::String(value), Kind::String) => {
                prost::encoding::string::encode(tag, value, buf)
            }
            (Value::Bytes(value), Kind::Bytes) => prost::encoding::bytes::encode(tag, value, buf),
            (Value::EnumNumber(value), Kind::Enum(_)) => {
                prost::encoding::int32::encode(tag, value, buf)
            }
            (Value::Message(message), Kind::Message(_)) => {
                if field_desc.is_group() {
                    prost::encoding::group::encode(tag, message, buf)
                } else {
                    prost::encoding::message::encode(tag, message, buf)
                }
            }
            (Value::List(values), _) if field_desc.is_list() => {
                if field_desc.is_packed() {
                    match field_desc.kind() {
                        Kind::Enum(_) => encode_packed_list(
                            tag,
                            values
                                .iter()
                                .map(|v| v.as_enum_number().expect("expected enum number")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Double => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_f64().expect("expected double")),
                            buf,
                            |v, b| b.put_f64_le(v),
                            |_| 8,
                        ),
                        Kind::Float => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_f32().expect("expected float")),
                            buf,
                            |v, b| b.put_f32_le(v),
                            |_| 4,
                        ),
                        Kind::Int32 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Int64 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Uint32 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Uint64 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Sint32 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            buf,
                            |v, b| prost::encoding::encode_varint(from_sint32(v) as u64, b),
                            |v| prost::encoding::encoded_len_varint(from_sint32(v) as u64),
                        ),
                        Kind::Sint64 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            buf,
                            |v, b| prost::encoding::encode_varint(from_sint64(v) as u64, b),
                            |v| prost::encoding::encoded_len_varint(from_sint64(v) as u64),
                        ),
                        Kind::Fixed32 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            buf,
                            |v, b| b.put_u32_le(v),
                            |_| 4,
                        ),
                        Kind::Fixed64 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            buf,
                            |v, b| b.put_u64_le(v),
                            |_| 8,
                        ),
                        Kind::Sfixed32 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            buf,
                            |v, b| b.put_i32_le(v),
                            |_| 4,
                        ),
                        Kind::Sfixed64 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            buf,
                            |v, b| b.put_i64_le(v),
                            |_| 8,
                        ),
                        Kind::Bool => encode_packed_list(
                            tag,
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
                let key_desc = map_entry.get_field(MAP_ENTRY_KEY_TAG).unwrap();
                let value_desc = map_entry.get_field(MAP_ENTRY_VALUE_TAG).unwrap();

                for (key, value) in values {
                    let len = key.encoded_len(&key_desc) + value.encoded_len(&value_desc);

                    prost::encoding::encode_key(tag, WireType::LengthDelimited, buf);
                    prost::encoding::encode_varint(len as u64, buf);

                    key.encode_field(&key_desc, buf);
                    value.encode_field(&value_desc, buf);
                }
            }
            (value, ty) => unreachable!(
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
                    prost::encoding::group::merge(field_desc.tag(), wire_type, message, buf, ctx)
                } else {
                    prost::encoding::message::merge(wire_type, message, buf, ctx)
                }
            }
            (Value::List(values), field_kind) if field_desc.is_list() => {
                if field_desc.is_packed() && wire_type == WireType::LengthDelimited {
                    let packed_wire_type = match field_desc.kind() {
                        Kind::Double | Kind::Fixed64 | Kind::Sfixed64 => WireType::SixtyFourBit,
                        Kind::Float | Kind::Fixed32 | Kind::Sfixed32 => WireType::ThirtyTwoBit,
                        Kind::Enum(_)
                        | Kind::Int32
                        | Kind::Int64
                        | Kind::Uint32
                        | Kind::Uint64
                        | Kind::Sint32
                        | Kind::Sint64
                        | Kind::Bool => WireType::Varint,
                        _ => unreachable!("invalid entry type for packed list"),
                    };
                    prost::encoding::merge_loop(values, buf, ctx, |values, buf, ctx| {
                        let mut value = Value::default_value_for_kind(&field_kind);
                        value.merge_field(field_desc, packed_wire_type, buf, ctx)?;
                        values.push(value);
                        Ok(())
                    })
                } else {
                    let mut value = Value::default_value_for_kind(&field_kind);
                    value.merge_field(field_desc, wire_type, buf, ctx)?;
                    values.push(value);
                    Ok(())
                }
            }
            (Value::Map(values), Kind::Message(map_entry)) if field_desc.is_map() => {
                let key_desc = map_entry.get_field(MAP_ENTRY_KEY_TAG).unwrap();
                let value_desc = map_entry.get_field(MAP_ENTRY_VALUE_TAG).unwrap();

                let mut key = MapKey::default_value(&key_desc.kind());
                let mut value = Value::default_value(&value_desc);
                prost::encoding::merge_loop(
                    &mut (&mut key, &mut value),
                    buf,
                    ctx,
                    |(key, value), buf, ctx| {
                        let (tag, wire_type) = prost::encoding::decode_key(buf)?;
                        match tag {
                            MAP_ENTRY_KEY_TAG => key.merge_field(&key_desc, wire_type, buf, ctx),
                            MAP_ENTRY_VALUE_TAG => {
                                value.merge_field(&value_desc, wire_type, buf, ctx)
                            }
                            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
                        }
                    },
                )?;
                values.insert(key, value);

                Ok(())
            }
            (value, ty) => unreachable!(
                "mismatch between DynamicMessage value {:?} and type {:?}",
                value, ty
            ),
        }
    }

    fn encoded_len(&self, field_desc: &FieldDescriptor) -> usize {
        if !field_desc.supports_presence() && self.is_default(field_desc) {
            return 0;
        }

        let tag = field_desc.tag();
        match (self, field_desc.kind()) {
            (Value::Bool(value), Kind::Bool) => prost::encoding::bool::encoded_len(tag, value),
            (Value::I32(value), Kind::Int32) => prost::encoding::int32::encoded_len(tag, value),
            (Value::I32(value), Kind::Sint32) => prost::encoding::sint32::encoded_len(tag, value),
            (Value::I32(value), Kind::Sfixed32) => {
                prost::encoding::sfixed32::encoded_len(tag, value)
            }
            (Value::I64(value), Kind::Int64) => prost::encoding::int64::encoded_len(tag, value),
            (Value::I64(value), Kind::Sint64) => prost::encoding::sint64::encoded_len(tag, value),
            (Value::I64(value), Kind::Sfixed64) => {
                prost::encoding::sfixed64::encoded_len(tag, value)
            }
            (Value::U32(value), Kind::Uint32) => prost::encoding::uint32::encoded_len(tag, value),
            (Value::U32(value), Kind::Fixed32) => prost::encoding::fixed32::encoded_len(tag, value),
            (Value::U64(value), Kind::Uint64) => prost::encoding::uint64::encoded_len(tag, value),
            (Value::U64(value), Kind::Fixed64) => prost::encoding::fixed64::encoded_len(tag, value),
            (Value::F32(value), Kind::Float) => prost::encoding::float::encoded_len(tag, value),
            (Value::F64(value), Kind::Double) => prost::encoding::double::encoded_len(tag, value),
            (Value::String(value), Kind::String) => {
                prost::encoding::string::encoded_len(tag, value)
            }
            (Value::Bytes(value), Kind::Bytes) => prost::encoding::bytes::encoded_len(tag, value),
            (Value::EnumNumber(value), Kind::Enum(_)) => {
                prost::encoding::int32::encoded_len(tag, value)
            }
            (Value::Message(message), Kind::Message(_)) => {
                if field_desc.is_group() {
                    prost::encoding::group::encoded_len(tag, message)
                } else {
                    prost::encoding::message::encoded_len(tag, message)
                }
            }
            (Value::List(values), _) if field_desc.is_list() => {
                if field_desc.is_packed() {
                    match field_desc.kind() {
                        Kind::Enum(_) => packed_list_encoded_len(
                            tag,
                            values
                                .iter()
                                .map(|v| v.as_enum_number().expect("expected enum number")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Double => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_f64().expect("expected double")),
                            |_| 8,
                        ),
                        Kind::Float => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_f32().expect("expected float")),
                            |_| 4,
                        ),
                        Kind::Int32 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Int64 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Uint32 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Uint64 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        Kind::Sint32 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            |v| prost::encoding::encoded_len_varint(from_sint32(v) as u64),
                        ),
                        Kind::Sint64 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            |v| prost::encoding::encoded_len_varint(from_sint64(v) as u64),
                        ),
                        Kind::Fixed32 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            |_| 4,
                        ),
                        Kind::Fixed64 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            |_| 8,
                        ),
                        Kind::Sfixed32 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            |_| 4,
                        ),
                        Kind::Sfixed64 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            |_| 8,
                        ),
                        Kind::Bool => packed_list_encoded_len(
                            tag,
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
                let key_desc = map_entry.get_field(MAP_ENTRY_KEY_TAG).unwrap();
                let value_desc = map_entry.get_field(MAP_ENTRY_VALUE_TAG).unwrap();

                let key_len = prost::encoding::key_len(tag);
                values
                    .iter()
                    .map(|(key, value)| {
                        let len = key.encoded_len(&key_desc) + value.encoded_len(&value_desc);

                        key_len + prost::encoding::encoded_len_varint(len as u64) + len
                    })
                    .sum::<usize>()
            }
            (value, ty) => unreachable!(
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

        let tag = field_desc.tag();
        match (self, field_desc.kind()) {
            (MapKey::Bool(value), Kind::Bool) => prost::encoding::bool::encode(tag, value, buf),
            (MapKey::I32(value), Kind::Int32) => prost::encoding::int32::encode(tag, value, buf),
            (MapKey::I32(value), Kind::Sint32) => prost::encoding::sint32::encode(tag, value, buf),
            (MapKey::I32(value), Kind::Sfixed32) => {
                prost::encoding::sfixed32::encode(tag, value, buf)
            }
            (MapKey::I64(value), Kind::Int64) => prost::encoding::int64::encode(tag, value, buf),
            (MapKey::I64(value), Kind::Sint64) => prost::encoding::sint64::encode(tag, value, buf),
            (MapKey::I64(value), Kind::Sfixed64) => {
                prost::encoding::sfixed64::encode(tag, value, buf)
            }
            (MapKey::U32(value), Kind::Uint32) => prost::encoding::uint32::encode(tag, value, buf),
            (MapKey::U32(value), Kind::Fixed32) => {
                prost::encoding::fixed32::encode(tag, value, buf)
            }
            (MapKey::U64(value), Kind::Uint64) => prost::encoding::uint64::encode(tag, value, buf),
            (MapKey::U64(value), Kind::Fixed64) => {
                prost::encoding::fixed64::encode(tag, value, buf)
            }
            (MapKey::String(value), Kind::String) => {
                prost::encoding::string::encode(tag, value, buf)
            }
            (value, ty) => unreachable!(
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
            (value, ty) => unreachable!(
                "mismatch between DynamicMessage value {:?} and type {:?}",
                value, ty
            ),
        }
    }

    fn encoded_len(&self, field_desc: &FieldDescriptor) -> usize {
        if !field_desc.supports_presence() && self.is_default(&field_desc.kind()) {
            return 0;
        }

        let tag = field_desc.tag();
        match (self, field_desc.kind()) {
            (MapKey::Bool(value), Kind::Bool) => prost::encoding::bool::encoded_len(tag, value),
            (MapKey::I32(value), Kind::Int32) => prost::encoding::int32::encoded_len(tag, value),
            (MapKey::I32(value), Kind::Sint32) => prost::encoding::sint32::encoded_len(tag, value),
            (MapKey::I32(value), Kind::Sfixed32) => {
                prost::encoding::sfixed32::encoded_len(tag, value)
            }
            (MapKey::I64(value), Kind::Int64) => prost::encoding::int64::encoded_len(tag, value),
            (MapKey::I64(value), Kind::Sint64) => prost::encoding::sint64::encoded_len(tag, value),
            (MapKey::I64(value), Kind::Sfixed64) => {
                prost::encoding::sfixed64::encoded_len(tag, value)
            }
            (MapKey::U32(value), Kind::Uint32) => prost::encoding::uint32::encoded_len(tag, value),
            (MapKey::U32(value), Kind::Fixed32) => {
                prost::encoding::fixed32::encoded_len(tag, value)
            }
            (MapKey::U64(value), Kind::Uint64) => prost::encoding::uint64::encoded_len(tag, value),
            (MapKey::U64(value), Kind::Fixed64) => {
                prost::encoding::fixed64::encoded_len(tag, value)
            }
            (MapKey::String(value), Kind::String) => {
                prost::encoding::string::encoded_len(tag, value)
            }
            (value, ty) => unreachable!(
                "mismatch between DynamicMessage value {:?} and type {:?}",
                value, ty
            ),
        }
    }
}

fn encode_packed_list<T, I, B, E, L>(tag: u32, iter: I, buf: &mut B, encode: E, encoded_len: L)
where
    I: IntoIterator<Item = T> + Clone,
    B: BufMut,
    E: Fn(T, &mut B),
    L: Fn(T) -> usize,
{
    prost::encoding::encode_key(tag, WireType::LengthDelimited, buf);
    let len: usize = iter.clone().into_iter().map(encoded_len).sum();
    prost::encoding::encode_varint(len as u64, buf);

    for value in iter {
        encode(value, buf);
    }
}

fn packed_list_encoded_len<T, I, L>(tag: u32, iter: I, encoded_len: L) -> usize
where
    I: IntoIterator<Item = T>,
    L: Fn(T) -> usize,
{
    let len: usize = iter.into_iter().map(encoded_len).sum();
    prost::encoding::key_len(tag) + prost::encoding::encoded_len_varint(len as u64) + len
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
