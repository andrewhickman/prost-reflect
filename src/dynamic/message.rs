use prost::{
    bytes::{Buf, BufMut},
    encoding::{DecodeContext, WireType},
    DecodeError, Message,
};

use crate::{
    descriptor::{FieldDescriptorKind, MAP_ENTRY_KEY_TAG, MAP_ENTRY_VALUE_TAG},
    DynamicMessage, FieldDescriptor, MapKey, Value,
};

impl Message for DynamicMessage {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        for field in self.fields.values() {
            field.value.encode_field(&field.desc, buf);
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
            field.value.merge_field(&field.desc, wire_type, buf, ctx)
        } else {
            prost::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        for field in self.fields.values() {
            len += field.value.encoded_len(&field.desc);
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
        let tag = field_desc.tag();
        match (self, field_desc.kind()) {
            (Value::Bool(value), FieldDescriptorKind::Bool) => {
                prost::encoding::bool::encode(tag, value, buf)
            }
            (Value::I32(value), FieldDescriptorKind::Int32) => {
                prost::encoding::int32::encode(tag, value, buf)
            }
            (Value::I32(value), FieldDescriptorKind::Sint32) => {
                prost::encoding::sint32::encode(tag, value, buf)
            }
            (Value::I32(value), FieldDescriptorKind::Sfixed32) => {
                prost::encoding::sfixed32::encode(tag, value, buf)
            }
            (Value::I64(value), FieldDescriptorKind::Int64) => {
                prost::encoding::int64::encode(tag, value, buf)
            }
            (Value::I64(value), FieldDescriptorKind::Sint64) => {
                prost::encoding::sint64::encode(tag, value, buf)
            }
            (Value::I64(value), FieldDescriptorKind::Sfixed64) => {
                prost::encoding::sfixed64::encode(tag, value, buf)
            }
            (Value::U32(value), FieldDescriptorKind::Uint32) => {
                prost::encoding::uint32::encode(tag, value, buf)
            }
            (Value::U32(value), FieldDescriptorKind::Fixed32) => {
                prost::encoding::fixed32::encode(tag, value, buf)
            }
            (Value::U64(value), FieldDescriptorKind::Uint64) => {
                prost::encoding::uint64::encode(tag, value, buf)
            }
            (Value::U64(value), FieldDescriptorKind::Fixed64) => {
                prost::encoding::fixed64::encode(tag, value, buf)
            }
            (Value::F32(value), FieldDescriptorKind::Float) => {
                prost::encoding::float::encode(tag, value, buf)
            }
            (Value::F64(value), FieldDescriptorKind::Double) => {
                prost::encoding::double::encode(tag, value, buf)
            }
            (Value::String(value), FieldDescriptorKind::String) => {
                prost::encoding::string::encode(tag, value, buf)
            }
            (Value::Bytes(value), FieldDescriptorKind::Bytes) => {
                prost::encoding::bytes::encode(tag, value, buf)
            }
            (Value::EnumNumber(value), FieldDescriptorKind::Enum(_)) => {
                prost::encoding::int32::encode(tag, value, buf)
            }
            (Value::Message(Some(message)), FieldDescriptorKind::Message(_)) => {
                if field_desc.is_group() {
                    prost::encoding::group::encode(tag, message, buf)
                } else {
                    prost::encoding::message::encode(tag, message, buf)
                }
            }
            (Value::Message(None), FieldDescriptorKind::Message(_)) => {}
            (Value::List(values), _) if field_desc.is_list() => {
                if field_desc.is_packed() {
                    match field_desc.kind() {
                        FieldDescriptorKind::Enum(_) => encode_packed_list(
                            tag,
                            values
                                .iter()
                                .map(|v| v.as_enum_number().expect("expected enum number")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        FieldDescriptorKind::Double => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_f64().expect("expected double")),
                            buf,
                            |v, b| b.put_f64_le(v),
                            |_| 8,
                        ),
                        FieldDescriptorKind::Float => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_f32().expect("expected float")),
                            buf,
                            |v, b| b.put_f32_le(v),
                            |_| 4,
                        ),
                        FieldDescriptorKind::Int32 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        FieldDescriptorKind::Int64 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        FieldDescriptorKind::Uint32 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        FieldDescriptorKind::Uint64 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        FieldDescriptorKind::Sint32 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            buf,
                            |v, b| prost::encoding::encode_varint(from_sint32(v) as u64, b),
                            |v| prost::encoding::encoded_len_varint(from_sint32(v) as u64),
                        ),
                        FieldDescriptorKind::Sint64 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            buf,
                            |v, b| prost::encoding::encode_varint(from_sint64(v) as u64, b),
                            |v| prost::encoding::encoded_len_varint(from_sint64(v) as u64),
                        ),
                        FieldDescriptorKind::Fixed32 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            buf,
                            |v, b| b.put_u32_le(v),
                            |_| 4,
                        ),
                        FieldDescriptorKind::Fixed64 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            buf,
                            |v, b| b.put_u64_le(v),
                            |_| 8,
                        ),
                        FieldDescriptorKind::Sfixed32 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            buf,
                            |v, b| b.put_i32_le(v),
                            |_| 4,
                        ),
                        FieldDescriptorKind::Sfixed64 => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            buf,
                            |v, b| b.put_i64_le(v),
                            |_| 8,
                        ),
                        FieldDescriptorKind::Bool => encode_packed_list(
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
            (Value::Map(values), FieldDescriptorKind::Message(map_entry))
                if field_desc.is_map() =>
            {
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
            _ => unreachable!("mismatch between DynamicMessage value and type"),
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
            (Value::Bool(value), FieldDescriptorKind::Bool) => {
                prost::encoding::bool::merge(wire_type, value, buf, ctx)
            }
            (Value::I32(value), FieldDescriptorKind::Int32) => {
                prost::encoding::int32::merge(wire_type, value, buf, ctx)
            }
            (Value::I32(value), FieldDescriptorKind::Sint32) => {
                prost::encoding::sint32::merge(wire_type, value, buf, ctx)
            }
            (Value::I32(value), FieldDescriptorKind::Sfixed32) => {
                prost::encoding::sfixed32::merge(wire_type, value, buf, ctx)
            }
            (Value::I64(value), FieldDescriptorKind::Int64) => {
                prost::encoding::int64::merge(wire_type, value, buf, ctx)
            }
            (Value::I64(value), FieldDescriptorKind::Sint64) => {
                prost::encoding::sint64::merge(wire_type, value, buf, ctx)
            }
            (Value::I64(value), FieldDescriptorKind::Sfixed64) => {
                prost::encoding::sfixed64::merge(wire_type, value, buf, ctx)
            }
            (Value::U32(value), FieldDescriptorKind::Uint32) => {
                prost::encoding::uint32::merge(wire_type, value, buf, ctx)
            }
            (Value::U32(value), FieldDescriptorKind::Fixed32) => {
                prost::encoding::fixed32::merge(wire_type, value, buf, ctx)
            }
            (Value::U64(value), FieldDescriptorKind::Uint64) => {
                prost::encoding::uint64::merge(wire_type, value, buf, ctx)
            }
            (Value::U64(value), FieldDescriptorKind::Fixed64) => {
                prost::encoding::fixed64::merge(wire_type, value, buf, ctx)
            }
            (Value::F32(value), FieldDescriptorKind::Float) => {
                prost::encoding::float::merge(wire_type, value, buf, ctx)
            }
            (Value::F64(value), FieldDescriptorKind::Double) => {
                prost::encoding::double::merge(wire_type, value, buf, ctx)
            }
            (Value::String(value), FieldDescriptorKind::String) => {
                prost::encoding::string::merge(wire_type, value, buf, ctx)
            }
            (Value::Bytes(value), FieldDescriptorKind::Bytes) => {
                prost::encoding::bytes::merge(wire_type, value, buf, ctx)
            }
            (Value::EnumNumber(value), FieldDescriptorKind::Enum(_)) => {
                prost::encoding::int32::merge(wire_type, value, buf, ctx)
            }
            (Value::Message(message), FieldDescriptorKind::Message(message_desc)) => {
                let message = message.get_or_insert_with(|| DynamicMessage::new(message_desc));
                if field_desc.is_group() {
                    prost::encoding::group::merge(field_desc.tag(), wire_type, message, buf, ctx)
                } else {
                    prost::encoding::message::merge(wire_type, message, buf, ctx)
                }
            }
            (Value::List(values), field_kind) if field_desc.is_list() => {
                if field_desc.is_packed() && wire_type == WireType::LengthDelimited {
                    let packed_wire_type = match field_desc.kind() {
                        FieldDescriptorKind::Double
                        | FieldDescriptorKind::Fixed64
                        | FieldDescriptorKind::Sfixed64 => WireType::SixtyFourBit,
                        FieldDescriptorKind::Float
                        | FieldDescriptorKind::Fixed32
                        | FieldDescriptorKind::Sfixed32 => WireType::ThirtyTwoBit,
                        FieldDescriptorKind::Enum(_)
                        | FieldDescriptorKind::Int32
                        | FieldDescriptorKind::Int64
                        | FieldDescriptorKind::Uint32
                        | FieldDescriptorKind::Uint64
                        | FieldDescriptorKind::Sint32
                        | FieldDescriptorKind::Sint64
                        | FieldDescriptorKind::Bool => WireType::Varint,
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
            (Value::Map(values), FieldDescriptorKind::Message(map_entry))
                if field_desc.is_map() =>
            {
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
            _ => unreachable!("invalid type for field in DynamicMessage"),
        }
    }

    fn encoded_len(&self, field_desc: &FieldDescriptor) -> usize {
        let tag = field_desc.tag();
        match (self, field_desc.kind()) {
            (Value::Bool(value), FieldDescriptorKind::Bool) => {
                prost::encoding::bool::encoded_len(tag, value)
            }
            (Value::I32(value), FieldDescriptorKind::Int32) => {
                prost::encoding::int32::encoded_len(tag, value)
            }
            (Value::I32(value), FieldDescriptorKind::Sint32) => {
                prost::encoding::sint32::encoded_len(tag, value)
            }
            (Value::I32(value), FieldDescriptorKind::Sfixed32) => {
                prost::encoding::sfixed32::encoded_len(tag, value)
            }
            (Value::I64(value), FieldDescriptorKind::Int64) => {
                prost::encoding::int64::encoded_len(tag, value)
            }
            (Value::I64(value), FieldDescriptorKind::Sint64) => {
                prost::encoding::sint64::encoded_len(tag, value)
            }
            (Value::I64(value), FieldDescriptorKind::Sfixed64) => {
                prost::encoding::sfixed64::encoded_len(tag, value)
            }
            (Value::U32(value), FieldDescriptorKind::Uint32) => {
                prost::encoding::uint32::encoded_len(tag, value)
            }
            (Value::U32(value), FieldDescriptorKind::Fixed32) => {
                prost::encoding::fixed32::encoded_len(tag, value)
            }
            (Value::U64(value), FieldDescriptorKind::Uint64) => {
                prost::encoding::uint64::encoded_len(tag, value)
            }
            (Value::U64(value), FieldDescriptorKind::Fixed64) => {
                prost::encoding::fixed64::encoded_len(tag, value)
            }
            (Value::F32(value), FieldDescriptorKind::Float) => {
                prost::encoding::float::encoded_len(tag, value)
            }
            (Value::F64(value), FieldDescriptorKind::Double) => {
                prost::encoding::double::encoded_len(tag, value)
            }
            (Value::String(value), FieldDescriptorKind::String) => {
                prost::encoding::string::encoded_len(tag, value)
            }
            (Value::Bytes(value), FieldDescriptorKind::Bytes) => {
                prost::encoding::bytes::encoded_len(tag, value)
            }
            (Value::EnumNumber(value), FieldDescriptorKind::Enum(_)) => {
                prost::encoding::int32::encoded_len(tag, value)
            }
            (Value::Message(Some(message)), FieldDescriptorKind::Message(_)) => {
                if field_desc.is_group() {
                    prost::encoding::group::encoded_len(tag, message)
                } else {
                    prost::encoding::message::encoded_len(tag, message)
                }
            }
            (Value::Message(None), FieldDescriptorKind::Message(_)) => 0,
            (Value::List(values), _) if field_desc.is_list() => {
                if field_desc.is_packed() {
                    match field_desc.kind() {
                        FieldDescriptorKind::Enum(_) => packed_list_encoded_len(
                            tag,
                            values
                                .iter()
                                .map(|v| v.as_enum_number().expect("expected enum number")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        FieldDescriptorKind::Double => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_f64().expect("expected double")),
                            |_| 8,
                        ),
                        FieldDescriptorKind::Float => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_f32().expect("expected float")),
                            |_| 4,
                        ),
                        FieldDescriptorKind::Int32 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        FieldDescriptorKind::Int64 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        FieldDescriptorKind::Uint32 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        FieldDescriptorKind::Uint64 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        FieldDescriptorKind::Sint32 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            |v| prost::encoding::encoded_len_varint(from_sint32(v) as u64),
                        ),
                        FieldDescriptorKind::Sint64 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            |v| prost::encoding::encoded_len_varint(from_sint64(v) as u64),
                        ),
                        FieldDescriptorKind::Fixed32 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            |_| 4,
                        ),
                        FieldDescriptorKind::Fixed64 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            |_| 8,
                        ),
                        FieldDescriptorKind::Sfixed32 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            |_| 4,
                        ),
                        FieldDescriptorKind::Sfixed64 => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            |_| 8,
                        ),
                        FieldDescriptorKind::Bool => packed_list_encoded_len(
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
            (Value::Map(values), FieldDescriptorKind::Message(map_entry))
                if field_desc.is_map() =>
            {
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
            _ => unreachable!("mismatch between DynamicMessage value and type"),
        }
    }
}

impl MapKey {
    fn encode_field<B>(&self, field_desc: &FieldDescriptor, buf: &mut B)
    where
        B: BufMut,
    {
        let tag = field_desc.tag();
        match (self, field_desc.kind()) {
            (MapKey::Bool(value), FieldDescriptorKind::Bool) => {
                prost::encoding::bool::encode(tag, value, buf)
            }
            (MapKey::I32(value), FieldDescriptorKind::Int32) => {
                prost::encoding::int32::encode(tag, value, buf)
            }
            (MapKey::I32(value), FieldDescriptorKind::Sint32) => {
                prost::encoding::sint32::encode(tag, value, buf)
            }
            (MapKey::I32(value), FieldDescriptorKind::Sfixed32) => {
                prost::encoding::sfixed32::encode(tag, value, buf)
            }
            (MapKey::I64(value), FieldDescriptorKind::Int64) => {
                prost::encoding::int64::encode(tag, value, buf)
            }
            (MapKey::I64(value), FieldDescriptorKind::Sint64) => {
                prost::encoding::sint64::encode(tag, value, buf)
            }
            (MapKey::I64(value), FieldDescriptorKind::Sfixed64) => {
                prost::encoding::sfixed64::encode(tag, value, buf)
            }
            (MapKey::U32(value), FieldDescriptorKind::Uint32) => {
                prost::encoding::uint32::encode(tag, value, buf)
            }
            (MapKey::U32(value), FieldDescriptorKind::Fixed32) => {
                prost::encoding::fixed32::encode(tag, value, buf)
            }
            (MapKey::U64(value), FieldDescriptorKind::Uint64) => {
                prost::encoding::uint64::encode(tag, value, buf)
            }
            (MapKey::U64(value), FieldDescriptorKind::Fixed64) => {
                prost::encoding::fixed64::encode(tag, value, buf)
            }
            (MapKey::String(value), FieldDescriptorKind::String) => {
                prost::encoding::string::encode(tag, value, buf)
            }
            _ => unreachable!("mismatch between DynamicMessage value and type"),
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
            (MapKey::Bool(value), FieldDescriptorKind::Bool) => {
                prost::encoding::bool::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I32(value), FieldDescriptorKind::Int32) => {
                prost::encoding::int32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I32(value), FieldDescriptorKind::Sint32) => {
                prost::encoding::sint32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I32(value), FieldDescriptorKind::Sfixed32) => {
                prost::encoding::sfixed32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I64(value), FieldDescriptorKind::Int64) => {
                prost::encoding::int64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I64(value), FieldDescriptorKind::Sint64) => {
                prost::encoding::sint64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I64(value), FieldDescriptorKind::Sfixed64) => {
                prost::encoding::sfixed64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::U32(value), FieldDescriptorKind::Uint32) => {
                prost::encoding::uint32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::U32(value), FieldDescriptorKind::Fixed32) => {
                prost::encoding::fixed32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::U64(value), FieldDescriptorKind::Uint64) => {
                prost::encoding::uint64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::U64(value), FieldDescriptorKind::Fixed64) => {
                prost::encoding::fixed64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::String(value), FieldDescriptorKind::String) => {
                prost::encoding::string::merge(wire_type, value, buf, ctx)
            }
            _ => unreachable!("mismatch between DynamicMessage value and type"),
        }
    }

    fn encoded_len(&self, field_desc: &FieldDescriptor) -> usize {
        let tag = field_desc.tag();
        match (self, field_desc.kind()) {
            (MapKey::Bool(value), FieldDescriptorKind::Bool) => {
                prost::encoding::bool::encoded_len(tag, value)
            }
            (MapKey::I32(value), FieldDescriptorKind::Int32) => {
                prost::encoding::int32::encoded_len(tag, value)
            }
            (MapKey::I32(value), FieldDescriptorKind::Sint32) => {
                prost::encoding::sint32::encoded_len(tag, value)
            }
            (MapKey::I32(value), FieldDescriptorKind::Sfixed32) => {
                prost::encoding::sfixed32::encoded_len(tag, value)
            }
            (MapKey::I64(value), FieldDescriptorKind::Int64) => {
                prost::encoding::int64::encoded_len(tag, value)
            }
            (MapKey::I64(value), FieldDescriptorKind::Sint64) => {
                prost::encoding::sint64::encoded_len(tag, value)
            }
            (MapKey::I64(value), FieldDescriptorKind::Sfixed64) => {
                prost::encoding::sfixed64::encoded_len(tag, value)
            }
            (MapKey::U32(value), FieldDescriptorKind::Uint32) => {
                prost::encoding::uint32::encoded_len(tag, value)
            }
            (MapKey::U32(value), FieldDescriptorKind::Fixed32) => {
                prost::encoding::fixed32::encoded_len(tag, value)
            }
            (MapKey::U64(value), FieldDescriptorKind::Uint64) => {
                prost::encoding::uint64::encoded_len(tag, value)
            }
            (MapKey::U64(value), FieldDescriptorKind::Fixed64) => {
                prost::encoding::fixed64::encoded_len(tag, value)
            }
            (MapKey::String(value), FieldDescriptorKind::String) => {
                prost::encoding::string::encoded_len(tag, value)
            }
            _ => unreachable!("mismatch between DynamicMessage value and type"),
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
