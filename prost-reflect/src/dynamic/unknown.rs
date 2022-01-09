use std::collections::BTreeMap;

use prost::{
    bytes::{Buf, BufMut, Bytes},
    encoding::{self, DecodeContext, WireType},
    DecodeError, Message,
};

/// An unknown field in a protobuf message.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UnknownField {
    /// An unknown field with the `Varint` wire type.
    Varint(u64),
    /// An unknown field with the `SixtyFourBit` wire type.
    SixtyFourBit([u8; 8]),
    /// An unknown field with the `LengthDelimited` wire type.
    LengthDelimited(Bytes),
    /// An unknown field with the group wire type.
    Group(UnknownFieldSet),
    /// An unknown field with the `ThirtyTwoBit` wire type.
    ThirtyTwoBit([u8; 4]),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct UnknownFieldSet {
    fields: BTreeMap<u32, Vec<UnknownField>>,
}

impl Message for UnknownFieldSet {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        for (&number, fields) in &self.fields {
            for field in fields {
                field.encode_field(number, buf)
            }
        }
    }

    fn merge_field<B>(
        &mut self,
        number: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        let field = UnknownField::decode(number, wire_type, buf, ctx)?;
        self.fields.entry(number).or_default().push(field);
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        for (&number, fields) in &self.fields {
            for field in fields {
                len += field.encoded_len(number);
            }
        }
        len
    }

    fn clear(&mut self) {
        self.fields.clear();
    }
}

impl UnknownField {
    pub fn encode_field<B>(&self, number: u32, buf: &mut B)
    where
        B: BufMut,
    {
        match self {
            UnknownField::Varint(value) => {
                encoding::encode_key(number, WireType::Varint, buf);
                encoding::encode_varint(*value, buf);
            }
            UnknownField::SixtyFourBit(value) => {
                encoding::encode_key(number, WireType::SixtyFourBit, buf);
                buf.put_slice(value);
            }
            UnknownField::LengthDelimited(value) => {
                encoding::bytes::encode(number, value, buf);
            }
            UnknownField::Group(value) => {
                encoding::group::encode(number, value, buf);
            }
            UnknownField::ThirtyTwoBit(value) => {
                encoding::encode_key(number, WireType::ThirtyTwoBit, buf);
                buf.put_slice(value);
            }
        }
    }

    pub fn decode<B>(
        number: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<Self, DecodeError>
    where
        B: Buf,
    {
        match wire_type {
            WireType::Varint => {
                let value = encoding::decode_varint(buf)?;
                Ok(UnknownField::Varint(value))
            }
            WireType::SixtyFourBit => {
                let mut value = [0; 8];
                if buf.remaining() < value.len() {
                    return Err(DecodeError::new("buffer underflow"));
                }
                buf.copy_to_slice(&mut value);
                Ok(UnknownField::SixtyFourBit(value))
            }
            WireType::LengthDelimited => {
                let mut value = Bytes::default();
                encoding::bytes::merge(wire_type, &mut value, buf, ctx)?;
                Ok(UnknownField::LengthDelimited(value))
            }
            WireType::StartGroup => {
                let mut value = UnknownFieldSet::default();
                encoding::group::merge(number, wire_type, &mut value, buf, ctx)?;
                Ok(UnknownField::Group(value))
            }
            WireType::EndGroup => Err(DecodeError::new("unexpected end group tag")),
            WireType::ThirtyTwoBit => {
                let mut value = [0; 4];
                if buf.remaining() < value.len() {
                    return Err(DecodeError::new("buffer underflow"));
                }
                buf.copy_to_slice(&mut value);
                Ok(UnknownField::ThirtyTwoBit(value))
            }
        }
    }

    pub fn encoded_len(&self, number: u32) -> usize {
        match self {
            UnknownField::Varint(value) => {
                encoding::key_len(number) + encoding::encoded_len_varint(*value)
            }
            UnknownField::SixtyFourBit(value) => encoding::key_len(number) + value.len(),
            UnknownField::LengthDelimited(value) => encoding::bytes::encoded_len(number, value),
            UnknownField::Group(value) => encoding::group::encoded_len(number, value),
            UnknownField::ThirtyTwoBit(value) => encoding::key_len(number) + value.len(),
        }
    }
}
