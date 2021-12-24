use std::collections::{BTreeMap, HashMap};

use prost::{
    bytes::{Buf, BufMut, Bytes},
    encoding::{DecodeContext, WireType},
    DecodeError, Message,
};

use crate::{
    descriptor::{
        ty::{self, TypeMap},
        FieldDescriptor,
    },
    Descriptor,
};

#[derive(Debug, Clone)]
pub struct DynamicMessage {
    desc: Descriptor,
    fields: BTreeMap<u32, DynamicValue>,
}

/// A dynamically-typed protobuf value.
///
/// Note this type may map to multiple possible protobuf wire formats, so it must be
/// serialized as part of a DynamicMessage.
#[derive(Debug, Clone)]
pub enum DynamicValue {
    Bool(bool),
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Bytes),
    EnumNumber(i32),
    Message(DynamicMessage),
    List(Vec<DynamicValue>),
    Map(HashMap<MapKey, DynamicValue>),
}

/// A dynamically-typed key for a protobuf map.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MapKey {
    Bool(bool),
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    String(String),
}

impl DynamicMessage {
    pub fn new(desc: Descriptor) -> Self {
        DynamicMessage {
            desc,
            fields: BTreeMap::new(),
        }
    }
}

impl Message for DynamicMessage {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        for (&tag, value) in &self.fields {
            let field_desc = self
                .desc
                .get_field(tag)
                .expect("unexpected field in DynamicMessage");
            value.encode_field(
                self.desc.type_map(),
                field_desc.tag(),
                field_desc.ty(),
                field_desc.is_group(),
                buf,
            );
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
        if let Some(field_desc) = self.desc.get_field(tag) {
            self.fields
                .entry(tag)
                .or_insert_with(|| DynamicValue::default_value(&field_desc))
                .merge_field(self.desc.type_map(), wire_type, &field_desc, buf, ctx)
        } else {
            prost::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        for (&tag, value) in &self.fields {
            let field_desc = self
                .desc
                .get_field(tag)
                .expect("unexpected field in DynamicMessage");
            len += value.encoded_len(
                self.desc.type_map(),
                field_desc.tag(),
                field_desc.ty(),
                field_desc.is_group(),
            );
        }
        len
    }

    fn clear(&mut self) {
        self.fields.clear()
    }
}

impl DynamicValue {
    pub fn default_value(desc: &FieldDescriptor) -> Self {
        match desc.ty() {
            ty::Type::Message(_) => {
                DynamicValue::Message(DynamicMessage::new(desc.message_descriptor().unwrap()))
            }
            ty::Type::Enum(_) => DynamicValue::EnumNumber(0),
            ty::Type::Scalar(scalar) => match scalar {
                ty::Scalar::Double => DynamicValue::F64(0.0),
                ty::Scalar::Float => DynamicValue::F32(0.0),
                ty::Scalar::Int32 | ty::Scalar::Sint32 | ty::Scalar::Sfixed32 => {
                    DynamicValue::I32(0)
                }
                ty::Scalar::Int64 | ty::Scalar::Sint64 | ty::Scalar::Sfixed64 => {
                    DynamicValue::I64(0)
                }
                ty::Scalar::Uint32 | ty::Scalar::Fixed32 => DynamicValue::U32(0),
                ty::Scalar::Uint64 | ty::Scalar::Fixed64 => DynamicValue::U64(0),
                ty::Scalar::Bool => DynamicValue::Bool(false),
                ty::Scalar::String => DynamicValue::String(String::default()),
                ty::Scalar::Bytes => DynamicValue::Bytes(Bytes::default()),
            },
            ty::Type::List(_) => DynamicValue::List(Vec::default()),
            ty::Type::Map(_) => DynamicValue::Map(HashMap::default()),
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            DynamicValue::Bool(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match *self {
            DynamicValue::U32(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match *self {
            DynamicValue::U64(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            DynamicValue::I64(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            DynamicValue::I32(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match *self {
            DynamicValue::F32(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            DynamicValue::F64(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_enum_number(&self) -> Option<i32> {
        match *self {
            DynamicValue::EnumNumber(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            DynamicValue::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&Bytes> {
        match self {
            DynamicValue::Bytes(value) => Some(value),
            _ => None,
        }
    }

    fn encode_field<B>(
        &self,
        type_map: &TypeMap,
        tag: u32,
        ty: &ty::Type,
        is_group: bool,
        buf: &mut B,
    ) where
        B: BufMut,
    {
        match (self, ty) {
            (DynamicValue::Bool(value), ty::Type::Scalar(ty::Scalar::Bool)) => {
                prost::encoding::bool::encode(tag, value, buf)
            }
            (DynamicValue::I32(value), ty::Type::Scalar(ty::Scalar::Int32)) => {
                prost::encoding::int32::encode(tag, value, buf)
            }
            (DynamicValue::I32(value), ty::Type::Scalar(ty::Scalar::Sint32)) => {
                prost::encoding::sint32::encode(tag, value, buf)
            }
            (DynamicValue::I32(value), ty::Type::Scalar(ty::Scalar::Sfixed32)) => {
                prost::encoding::sfixed32::encode(tag, value, buf)
            }
            (DynamicValue::I64(value), ty::Type::Scalar(ty::Scalar::Int64)) => {
                prost::encoding::int64::encode(tag, value, buf)
            }
            (DynamicValue::I64(value), ty::Type::Scalar(ty::Scalar::Sint64)) => {
                prost::encoding::sint64::encode(tag, value, buf)
            }
            (DynamicValue::I64(value), ty::Type::Scalar(ty::Scalar::Sfixed64)) => {
                prost::encoding::sfixed64::encode(tag, value, buf)
            }
            (DynamicValue::U32(value), ty::Type::Scalar(ty::Scalar::Uint32)) => {
                prost::encoding::uint32::encode(tag, value, buf)
            }
            (DynamicValue::U32(value), ty::Type::Scalar(ty::Scalar::Fixed32)) => {
                prost::encoding::fixed32::encode(tag, value, buf)
            }
            (DynamicValue::U64(value), ty::Type::Scalar(ty::Scalar::Uint64)) => {
                prost::encoding::uint64::encode(tag, value, buf)
            }
            (DynamicValue::U64(value), ty::Type::Scalar(ty::Scalar::Fixed64)) => {
                prost::encoding::fixed64::encode(tag, value, buf)
            }
            (DynamicValue::F32(value), ty::Type::Scalar(ty::Scalar::Float)) => {
                prost::encoding::float::encode(tag, value, buf)
            }
            (DynamicValue::F64(value), ty::Type::Scalar(ty::Scalar::Double)) => {
                prost::encoding::double::encode(tag, value, buf)
            }
            (DynamicValue::String(value), ty::Type::Scalar(ty::Scalar::String)) => {
                prost::encoding::string::encode(tag, value, buf)
            }
            (DynamicValue::Bytes(value), ty::Type::Scalar(ty::Scalar::Bytes)) => {
                prost::encoding::bytes::encode(tag, value, buf)
            }
            (DynamicValue::EnumNumber(value), ty::Type::Enum(_)) => {
                prost::encoding::int32::encode(tag, value, buf)
            }
            (DynamicValue::Message(message), ty::Type::Message(_)) => {
                if is_group {
                    prost::encoding::group::encode(tag, message, buf)
                } else {
                    prost::encoding::message::encode(tag, message, buf)
                }
            }
            (DynamicValue::List(values), ty::Type::List(list)) => {
                if list.packed {
                    match &type_map[list.ty] {
                        ty::Type::Enum(_) => encode_packed_list(
                            tag,
                            values
                                .iter()
                                .map(|v| v.as_enum_number().expect("expected enum number")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Double) => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_f64().expect("expected double")),
                            buf,
                            |v, b| b.put_f64_le(v),
                            |_| 8,
                        ),
                        ty::Type::Scalar(ty::Scalar::Float) => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_f32().expect("expected float")),
                            buf,
                            |v, b| b.put_f32_le(v),
                            |_| 4,
                        ),
                        ty::Type::Scalar(ty::Scalar::Int32) => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Int64) => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Uint32) => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Uint64) => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            buf,
                            |v, b| prost::encoding::encode_varint(v as u64, b),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Sint32) => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            buf,
                            |v, b| prost::encoding::encode_varint(from_sint32(v) as u64, b),
                            |v| prost::encoding::encoded_len_varint(from_sint32(v) as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Sint64) => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            buf,
                            |v, b| prost::encoding::encode_varint(from_sint64(v) as u64, b),
                            |v| prost::encoding::encoded_len_varint(from_sint64(v) as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Fixed32) => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            buf,
                            |v, b| b.put_u32_le(v),
                            |_| 4,
                        ),
                        ty::Type::Scalar(ty::Scalar::Fixed64) => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            buf,
                            |v, b| b.put_u64_le(v),
                            |_| 8,
                        ),
                        ty::Type::Scalar(ty::Scalar::Sfixed32) => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            buf,
                            |v, b| b.put_i32_le(v),
                            |_| 4,
                        ),
                        ty::Type::Scalar(ty::Scalar::Sfixed64) => encode_packed_list(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            buf,
                            |v, b| b.put_i64_le(v),
                            |_| 8,
                        ),
                        ty::Type::Scalar(ty::Scalar::Bool) => encode_packed_list(
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
                        value.encode_field(type_map, tag, &type_map[list.ty], is_group, buf);
                    }
                }
            }
            (DynamicValue::Map(values), ty::Type::Map(map)) => {
                let value_ty = &type_map[map.value_ty];
                for (key, value) in values {
                    let len = key.encoded_len(1, map.key_ty)
                        + value.encoded_len(type_map, 2, value_ty, false);

                    prost::encoding::encode_key(tag, WireType::LengthDelimited, buf);
                    prost::encoding::encode_varint(len as u64, buf);

                    key.encode_field(1, map.key_ty, buf);
                    value.encode_field(type_map, 2, value_ty, false, buf);
                }
            }
            _ => unreachable!("mismatch between DynamicMessage value and type"),
        }
    }

    fn merge_field<B>(
        &mut self,
        type_map: &TypeMap,
        wire_type: WireType,
        field_desc: &FieldDescriptor,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
    {
        match (self, field_desc.ty()) {
            (DynamicValue::Bool(value), ty::Type::Scalar(ty::Scalar::Bool)) => {
                prost::encoding::bool::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::I32(value), ty::Type::Scalar(ty::Scalar::Int32)) => {
                prost::encoding::int32::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::I32(value), ty::Type::Scalar(ty::Scalar::Sint32)) => {
                prost::encoding::sint32::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::I32(value), ty::Type::Scalar(ty::Scalar::Sfixed32)) => {
                prost::encoding::sfixed32::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::I64(value), ty::Type::Scalar(ty::Scalar::Int64)) => {
                prost::encoding::int64::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::I64(value), ty::Type::Scalar(ty::Scalar::Sint64)) => {
                prost::encoding::sint64::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::I64(value), ty::Type::Scalar(ty::Scalar::Sfixed64)) => {
                prost::encoding::sfixed64::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::U32(value), ty::Type::Scalar(ty::Scalar::Uint32)) => {
                prost::encoding::uint32::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::U32(value), ty::Type::Scalar(ty::Scalar::Fixed32)) => {
                prost::encoding::fixed32::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::U64(value), ty::Type::Scalar(ty::Scalar::Uint64)) => {
                prost::encoding::uint64::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::U64(value), ty::Type::Scalar(ty::Scalar::Fixed64)) => {
                prost::encoding::fixed64::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::F32(value), ty::Type::Scalar(ty::Scalar::Float)) => {
                prost::encoding::float::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::F64(value), ty::Type::Scalar(ty::Scalar::Double)) => {
                prost::encoding::double::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::String(value), ty::Type::Scalar(ty::Scalar::String)) => {
                prost::encoding::string::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::Bytes(value), ty::Type::Scalar(ty::Scalar::Bytes)) => {
                prost::encoding::bytes::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::EnumNumber(value), ty::Type::Enum(_)) => {
                prost::encoding::int32::merge(wire_type, value, buf, ctx)
            }
            (DynamicValue::Message(message), ty::Type::Message(_)) => {
                if field_desc.is_group() {
                    prost::encoding::group::merge(field_desc.tag(), wire_type, message, buf, ctx)
                } else {
                    prost::encoding::message::merge(wire_type, message, buf, ctx)
                }
            }
            (DynamicValue::List(values), ty::Type::List(list)) => {
                if list.packed && wire_type == WireType::LengthDelimited {
                    let packed_wire_type = match &type_map[list.ty] {
                        ty::Type::Enum(_) => WireType::Varint,
                        ty::Type::Scalar(scalar) => scalar.wire_type(),
                        _ => unreachable!("invalid entry type for packed list"),
                    };
                    prost::encoding::merge_loop(values, buf, ctx, |values, buf, ctx| {
                        let mut value = DynamicValue::default_value(field_desc);
                        value.merge_field(type_map, packed_wire_type, field_desc, buf, ctx)?;
                        values.push(value);
                        Ok(())
                    })
                } else {
                    let mut value = DynamicValue::default_value(field_desc);
                    value.merge_field(type_map, wire_type, field_desc, buf, ctx)?;
                    values.push(value);
                    Ok(())
                }
            }
            (DynamicValue::Map(values), ty::Type::Map(map)) => {
                let map_entry_desc = field_desc.map_entry_descriptor().unwrap();
                let key_desc = map_entry_desc.get_field(1).unwrap();
                let value_desc = map_entry_desc.get_field(2).unwrap();

                let mut key = MapKey::default_value(&key_desc);
                let mut value = DynamicValue::default_value(&value_desc);
                prost::encoding::merge_loop(
                    &mut (&mut key, &mut value),
                    buf,
                    ctx,
                    |(key, value), buf, ctx| {
                        let (tag, wire_type) = prost::encoding::decode_key(buf)?;
                        match tag {
                            1 => key.merge_field(wire_type, map.key_ty, buf, ctx),
                            2 => value.merge_field(type_map, wire_type, &value_desc, buf, ctx),
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

    fn encoded_len(&self, type_map: &TypeMap, tag: u32, ty: &ty::Type, is_group: bool) -> usize {
        match (self, ty) {
            (DynamicValue::Bool(value), ty::Type::Scalar(ty::Scalar::Bool)) => {
                prost::encoding::bool::encoded_len(tag, value)
            }
            (DynamicValue::I32(value), ty::Type::Scalar(ty::Scalar::Int32)) => {
                prost::encoding::int32::encoded_len(tag, value)
            }
            (DynamicValue::I32(value), ty::Type::Scalar(ty::Scalar::Sint32)) => {
                prost::encoding::sint32::encoded_len(tag, value)
            }
            (DynamicValue::I32(value), ty::Type::Scalar(ty::Scalar::Sfixed32)) => {
                prost::encoding::sfixed32::encoded_len(tag, value)
            }
            (DynamicValue::I64(value), ty::Type::Scalar(ty::Scalar::Int64)) => {
                prost::encoding::int64::encoded_len(tag, value)
            }
            (DynamicValue::I64(value), ty::Type::Scalar(ty::Scalar::Sint64)) => {
                prost::encoding::sint64::encoded_len(tag, value)
            }
            (DynamicValue::I64(value), ty::Type::Scalar(ty::Scalar::Sfixed64)) => {
                prost::encoding::sfixed64::encoded_len(tag, value)
            }
            (DynamicValue::U32(value), ty::Type::Scalar(ty::Scalar::Uint32)) => {
                prost::encoding::uint32::encoded_len(tag, value)
            }
            (DynamicValue::U32(value), ty::Type::Scalar(ty::Scalar::Fixed32)) => {
                prost::encoding::fixed32::encoded_len(tag, value)
            }
            (DynamicValue::U64(value), ty::Type::Scalar(ty::Scalar::Uint64)) => {
                prost::encoding::uint64::encoded_len(tag, value)
            }
            (DynamicValue::U64(value), ty::Type::Scalar(ty::Scalar::Fixed64)) => {
                prost::encoding::fixed64::encoded_len(tag, value)
            }
            (DynamicValue::F32(value), ty::Type::Scalar(ty::Scalar::Float)) => {
                prost::encoding::float::encoded_len(tag, value)
            }
            (DynamicValue::F64(value), ty::Type::Scalar(ty::Scalar::Double)) => {
                prost::encoding::double::encoded_len(tag, value)
            }
            (DynamicValue::String(value), ty::Type::Scalar(ty::Scalar::String)) => {
                prost::encoding::string::encoded_len(tag, value)
            }
            (DynamicValue::Bytes(value), ty::Type::Scalar(ty::Scalar::Bytes)) => {
                prost::encoding::bytes::encoded_len(tag, value)
            }
            (DynamicValue::EnumNumber(value), ty::Type::Enum(_)) => {
                prost::encoding::int32::encoded_len(tag, value)
            }
            (DynamicValue::Message(message), ty::Type::Message(_)) => {
                if is_group {
                    prost::encoding::group::encoded_len(tag, message)
                } else {
                    prost::encoding::message::encoded_len(tag, message)
                }
            }
            (DynamicValue::List(values), ty::Type::List(list)) => {
                if list.packed {
                    match &type_map[list.ty] {
                        ty::Type::Enum(_) => packed_list_encoded_len(
                            tag,
                            values
                                .iter()
                                .map(|v| v.as_enum_number().expect("expected enum number")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Double) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_f64().expect("expected double")),
                            |_| 8,
                        ),
                        ty::Type::Scalar(ty::Scalar::Float) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_f32().expect("expected float")),
                            |_| 4,
                        ),
                        ty::Type::Scalar(ty::Scalar::Int32) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Int64) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Uint32) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Uint64) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Sint32) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            |v| prost::encoding::encoded_len_varint(from_sint32(v) as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Sint64) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            |v| prost::encoding::encoded_len_varint(from_sint64(v) as u64),
                        ),
                        ty::Type::Scalar(ty::Scalar::Fixed32) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_u32().expect("expected u32")),
                            |_| 4,
                        ),
                        ty::Type::Scalar(ty::Scalar::Fixed64) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_u64().expect("expected u64")),
                            |_| 8,
                        ),
                        ty::Type::Scalar(ty::Scalar::Sfixed32) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i32().expect("expected i32")),
                            |_| 4,
                        ),
                        ty::Type::Scalar(ty::Scalar::Sfixed64) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_i64().expect("expected i64")),
                            |_| 8,
                        ),
                        ty::Type::Scalar(ty::Scalar::Bool) => packed_list_encoded_len(
                            tag,
                            values.iter().map(|v| v.as_bool().expect("expected bool")),
                            |v| prost::encoding::encoded_len_varint(v as u64),
                        ),
                        _ => panic!("invalid type for packed field in DynamicMessage"),
                    }
                } else {
                    values
                        .iter()
                        .map(|value| value.encoded_len(type_map, tag, &type_map[list.ty], is_group))
                        .sum()
                }
            }
            (DynamicValue::Map(values), ty::Type::Map(map)) => {
                let value_ty = &type_map[map.value_ty];
                let key_len = prost::encoding::key_len(tag);
                values
                    .iter()
                    .map(|(key, value)| {
                        let len = key.encoded_len(1, map.key_ty)
                            + value.encoded_len(type_map, 2, value_ty, false);

                        key_len + prost::encoding::encoded_len_varint(len as u64) + len
                    })
                    .sum::<usize>()
            }
            _ => unreachable!("mismatch between DynamicMessage value and type"),
        }
    }
}

impl MapKey {
    fn default_value(desc: &FieldDescriptor) -> Self {
        match desc.ty() {
            ty::Type::Scalar(scalar) => match scalar {
                ty::Scalar::Int32 | ty::Scalar::Sint32 | ty::Scalar::Sfixed32 => MapKey::I32(0),
                ty::Scalar::Int64 | ty::Scalar::Sint64 | ty::Scalar::Sfixed64 => MapKey::I64(0),
                ty::Scalar::Uint32 | ty::Scalar::Fixed32 => MapKey::U32(0),
                ty::Scalar::Uint64 | ty::Scalar::Fixed64 => MapKey::U64(0),
                ty::Scalar::Bool => MapKey::Bool(false),
                _ => panic!("invalid type for map key"),
            },
            _ => panic!("invalid type for map key"),
        }
    }

    fn encode_field<B>(&self, tag: u32, ty: ty::TypeId, buf: &mut B)
    where
        B: BufMut,
    {
        match (self, ty) {
            (MapKey::Bool(value), ty::TypeId::BOOL) => {
                prost::encoding::bool::encode(tag, value, buf)
            }
            (MapKey::I32(value), ty::TypeId::INT32) => {
                prost::encoding::int32::encode(tag, value, buf)
            }
            (MapKey::I32(value), ty::TypeId::SINT32) => {
                prost::encoding::sint32::encode(tag, value, buf)
            }
            (MapKey::I32(value), ty::TypeId::SFIXED32) => {
                prost::encoding::sfixed32::encode(tag, value, buf)
            }
            (MapKey::I64(value), ty::TypeId::INT64) => {
                prost::encoding::int64::encode(tag, value, buf)
            }
            (MapKey::I64(value), ty::TypeId::SINT64) => {
                prost::encoding::sint64::encode(tag, value, buf)
            }
            (MapKey::I64(value), ty::TypeId::SFIXED64) => {
                prost::encoding::sfixed64::encode(tag, value, buf)
            }
            (MapKey::U32(value), ty::TypeId::UINT32) => {
                prost::encoding::uint32::encode(tag, value, buf)
            }
            (MapKey::U32(value), ty::TypeId::FIXED32) => {
                prost::encoding::fixed32::encode(tag, value, buf)
            }
            (MapKey::U64(value), ty::TypeId::UINT64) => {
                prost::encoding::uint64::encode(tag, value, buf)
            }
            (MapKey::U64(value), ty::TypeId::FIXED64) => {
                prost::encoding::fixed64::encode(tag, value, buf)
            }
            (MapKey::String(value), ty::TypeId::STRING) => {
                prost::encoding::string::encode(tag, value, buf)
            }
            _ => unreachable!("mismatch between DynamicMessage value and type"),
        }
    }

    fn merge_field<B>(
        &mut self,
        wire_type: WireType,
        ty: ty::TypeId,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
    {
        match (self, ty) {
            (MapKey::Bool(value), ty::TypeId::BOOL) => {
                prost::encoding::bool::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I32(value), ty::TypeId::INT32) => {
                prost::encoding::int32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I32(value), ty::TypeId::SINT32) => {
                prost::encoding::sint32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I32(value), ty::TypeId::SFIXED32) => {
                prost::encoding::sfixed32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I64(value), ty::TypeId::INT64) => {
                prost::encoding::int64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I64(value), ty::TypeId::SINT64) => {
                prost::encoding::sint64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::I64(value), ty::TypeId::SFIXED64) => {
                prost::encoding::sfixed64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::U32(value), ty::TypeId::UINT32) => {
                prost::encoding::uint32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::U32(value), ty::TypeId::FIXED32) => {
                prost::encoding::fixed32::merge(wire_type, value, buf, ctx)
            }
            (MapKey::U64(value), ty::TypeId::UINT64) => {
                prost::encoding::uint64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::U64(value), ty::TypeId::FIXED64) => {
                prost::encoding::fixed64::merge(wire_type, value, buf, ctx)
            }
            (MapKey::String(value), ty::TypeId::STRING) => {
                prost::encoding::string::merge(wire_type, value, buf, ctx)
            }
            _ => unreachable!("mismatch between DynamicMessage value and type"),
        }
    }

    fn encoded_len(&self, tag: u32, ty: ty::TypeId) -> usize {
        match (self, ty) {
            (MapKey::Bool(value), ty::TypeId::BOOL) => {
                prost::encoding::bool::encoded_len(tag, value)
            }
            (MapKey::I32(value), ty::TypeId::INT32) => {
                prost::encoding::int32::encoded_len(tag, value)
            }
            (MapKey::I32(value), ty::TypeId::SINT32) => {
                prost::encoding::sint32::encoded_len(tag, value)
            }
            (MapKey::I32(value), ty::TypeId::SFIXED32) => {
                prost::encoding::sfixed32::encoded_len(tag, value)
            }
            (MapKey::I64(value), ty::TypeId::INT64) => {
                prost::encoding::int64::encoded_len(tag, value)
            }
            (MapKey::I64(value), ty::TypeId::SINT64) => {
                prost::encoding::sint64::encoded_len(tag, value)
            }
            (MapKey::I64(value), ty::TypeId::SFIXED64) => {
                prost::encoding::sfixed64::encoded_len(tag, value)
            }
            (MapKey::U32(value), ty::TypeId::UINT32) => {
                prost::encoding::uint32::encoded_len(tag, value)
            }
            (MapKey::U32(value), ty::TypeId::FIXED32) => {
                prost::encoding::fixed32::encoded_len(tag, value)
            }
            (MapKey::U64(value), ty::TypeId::UINT64) => {
                prost::encoding::uint64::encoded_len(tag, value)
            }
            (MapKey::U64(value), ty::TypeId::FIXED64) => {
                prost::encoding::fixed64::encoded_len(tag, value)
            }
            (MapKey::String(value), ty::TypeId::STRING) => {
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
