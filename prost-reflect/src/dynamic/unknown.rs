use std::{fmt, slice};

use prost::{
    bytes::{Buf, BufMut, Bytes},
    encoding::{self, DecodeContext, WireType},
    DecodeError, Message,
};

use crate::text_format;

/// An unknown field found when deserializing a protobuf message.
///
/// A field is unknown if the message descriptor does not contain a field with the given number. This is often the
/// result of a new field being added to the message definition.
///
/// The [`Message`](prost::Message) implementation of [`DynamicMessage`](crate::DynamicMessage) will preserve any unknown
/// fields.
#[derive(Debug, Clone, PartialEq)]
pub struct UnknownField {
    number: u32,
    value: UnknownFieldValue,
}

/// The vaalue of an unknown field in a protobuf message.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UnknownFieldValue {
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
    fields: Vec<UnknownField>,
}

impl UnknownField {
    /// The number of this field as found during decoding.
    pub fn number(&self) -> u32 {
        self.number
    }

    /// The wire type of this field as found during decoding.
    pub fn wire_type(&self) -> WireType {
        match &self.value {
            UnknownFieldValue::Varint(_) => WireType::Varint,
            UnknownFieldValue::SixtyFourBit(_) => WireType::SixtyFourBit,
            UnknownFieldValue::LengthDelimited(_) => WireType::LengthDelimited,
            UnknownFieldValue::Group(_) => WireType::StartGroup,
            UnknownFieldValue::ThirtyTwoBit(_) => WireType::ThirtyTwoBit,
        }
    }

    pub(crate) fn value(&self) -> &UnknownFieldValue {
        &self.value
    }

    /// Encodes this field into its byte representation.
    pub fn encode<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        match &self.value {
            UnknownFieldValue::Varint(value) => {
                encoding::encode_key(self.number, WireType::Varint, buf);
                encoding::encode_varint(*value, buf);
            }
            UnknownFieldValue::SixtyFourBit(value) => {
                encoding::encode_key(self.number, WireType::SixtyFourBit, buf);
                buf.put_slice(value);
            }
            UnknownFieldValue::LengthDelimited(value) => {
                encoding::bytes::encode(self.number, value, buf);
            }
            UnknownFieldValue::Group(value) => {
                encoding::group::encode(self.number, value, buf);
            }
            UnknownFieldValue::ThirtyTwoBit(value) => {
                encoding::encode_key(self.number, WireType::ThirtyTwoBit, buf);
                buf.put_slice(value);
            }
        }
    }

    /// Decodes an unknown field from the given buffer.
    ///
    /// This method will read the field number and wire type from the buffer. Normally, it is useful to know
    /// the field number before deciding whether to treat a field as unknown. See [`decode_value`](UnknownField::decode_value)
    /// if you have already read the number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost_reflect::{DescriptorPool, UnknownField};
    /// # use prost::encoding::{DecodeContext, WireType};
    /// # let pool = DescriptorPool::decode(include_bytes!("../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("google.protobuf.Empty").unwrap();
    /// let unknown_field = UnknownField::decode(&mut b"\x1a\x02\x10\x42".as_ref(), DecodeContext::default()).unwrap();
    /// assert_eq!(unknown_field.number(), 3);
    /// assert_eq!(unknown_field.wire_type(), WireType::LengthDelimited);
    /// ```
    pub fn decode<B>(buf: &mut B, ctx: DecodeContext) -> Result<Self, DecodeError>
    where
        B: Buf,
    {
        let (number, wire_type) = encoding::decode_key(buf)?;
        Self::decode_value(number, wire_type, buf, ctx)
    }

    /// Given a field number and wire type, decodes the value of an unknown field.
    ///
    /// This method assumes the field number and wire type have already been read from the buffer.
    /// See also [`decode`](UnknownField::decode).
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost_reflect::{DescriptorPool, UnknownField};
    /// # use prost::encoding::{DecodeContext, WireType};
    /// # let pool = DescriptorPool::decode(include_bytes!("../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("google.protobuf.Empty").unwrap();
    /// let unknown_field = UnknownField::decode_value(3, WireType::LengthDelimited, &mut b"\x02\x10\x42".as_ref(), DecodeContext::default()).unwrap();
    /// assert_eq!(unknown_field.number(), 3);
    /// assert_eq!(unknown_field.wire_type(), WireType::LengthDelimited);
    ///
    /// let mut buf = Vec::new();
    /// unknown_field.encode(&mut buf);
    /// assert_eq!(buf, b"\x1a\x02\x10\x42");
    /// ```
    pub fn decode_value<B>(
        number: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<Self, DecodeError>
    where
        B: Buf,
    {
        let value = match wire_type {
            WireType::Varint => {
                let value = encoding::decode_varint(buf)?;
                UnknownFieldValue::Varint(value)
            }
            WireType::SixtyFourBit => {
                let mut value = [0; 8];
                if buf.remaining() < value.len() {
                    return Err(DecodeError::new("buffer underflow"));
                }
                buf.copy_to_slice(&mut value);
                UnknownFieldValue::SixtyFourBit(value)
            }
            WireType::LengthDelimited => {
                let mut value = Bytes::default();
                encoding::bytes::merge(wire_type, &mut value, buf, ctx)?;
                UnknownFieldValue::LengthDelimited(value)
            }
            WireType::StartGroup => {
                let mut value = UnknownFieldSet::default();
                encoding::group::merge(number, wire_type, &mut value, buf, ctx)?;
                UnknownFieldValue::Group(value)
            }
            WireType::EndGroup => return Err(DecodeError::new("unexpected end group tag")),
            WireType::ThirtyTwoBit => {
                let mut value = [0; 4];
                if buf.remaining() < value.len() {
                    return Err(DecodeError::new("buffer underflow"));
                }
                buf.copy_to_slice(&mut value);
                UnknownFieldValue::ThirtyTwoBit(value)
            }
        };

        Ok(UnknownField { number, value })
    }

    /// Gets the length of this field when encoded to its byte representation.
    pub fn encoded_len(&self) -> usize {
        match &self.value {
            UnknownFieldValue::Varint(value) => {
                encoding::key_len(self.number) + encoding::encoded_len_varint(*value)
            }
            UnknownFieldValue::SixtyFourBit(value) => encoding::key_len(self.number) + value.len(),
            UnknownFieldValue::LengthDelimited(value) => {
                encoding::bytes::encoded_len(self.number, value)
            }
            UnknownFieldValue::Group(value) => encoding::group::encoded_len(self.number, value),
            UnknownFieldValue::ThirtyTwoBit(value) => encoding::key_len(self.number) + value.len(),
        }
    }
}

impl fmt::Display for UnknownField {
    /// Formats this unknown field using the protobuf text format.
    ///
    /// The protobuf format does not include type information, so the formatter will attempt to infer types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost_reflect::{DescriptorPool, UnknownField};
    /// # use prost::encoding::DecodeContext;
    /// # let pool = DescriptorPool::decode(include_bytes!("../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("google.protobuf.Empty").unwrap();
    /// let unknown_field = UnknownField::decode(&mut b"\x1a\x02\x10\x42".as_ref(), DecodeContext::default()).unwrap();
    /// assert_eq!(format!("{}", unknown_field), "3{2:66}");
    /// // The alternate format specifier may be used to indent the output
    /// assert_eq!(format!("{:#}", unknown_field), "3 {\n  2: 66\n}");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        text_format::Writer::new(text_format::FormatOptions::new().pretty(f.alternate()), f)
            .fmt_unknown_field(self)
    }
}

impl UnknownFieldSet {
    pub(crate) fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub(crate) fn iter(&self) -> slice::Iter<'_, UnknownField> {
        self.fields.iter()
    }

    pub(crate) fn insert(&mut self, unknown: UnknownField) {
        self.fields.push(unknown);
    }
}

impl Message for UnknownFieldSet {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        for field in &self.fields {
            field.encode(buf)
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
        let field = UnknownField::decode_value(number, wire_type, buf, ctx)?;
        self.fields.push(field);
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        for field in &self.fields {
            len += field.encoded_len();
        }
        len
    }

    fn clear(&mut self) {
        self.fields.clear();
    }
}

impl FromIterator<UnknownField> for UnknownFieldSet {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = UnknownField>,
    {
        UnknownFieldSet {
            fields: Vec::from_iter(iter),
        }
    }
}

#[cfg(test)]
mod tests {
    use prost::{
        bytes::Bytes,
        encoding::{DecodeContext, WireType},
    };

    use super::{UnknownField, UnknownFieldSet, UnknownFieldValue};

    fn assert_roundtrip(expected: &[u8], value: &UnknownField) {
        assert_eq!(expected.len(), value.encoded_len());

        let mut actual = Vec::with_capacity(expected.len());
        value.encode(&mut actual);
        assert_eq!(expected, actual.as_slice());
    }

    #[test]
    fn sixty_four_bit() {
        let bytes = b"\x09\x9a\x99\x99\x99\x99\x99\xf1\x3ftail";
        let mut buf = bytes.as_ref();

        let value = UnknownField::decode(&mut buf, DecodeContext::default()).unwrap();

        assert_eq!(value.number(), 1);
        assert_eq!(value.wire_type(), WireType::SixtyFourBit);
        assert_eq!(
            value.value(),
            &UnknownFieldValue::SixtyFourBit(*b"\x9a\x99\x99\x99\x99\x99\xf1\x3f")
        );
        assert_eq!(buf, b"tail");

        assert_roundtrip(bytes.strip_suffix(buf).unwrap(), &value);
    }

    #[test]
    fn thirty_two_bit() {
        let bytes = b"\x15\xcd\xcc\x0c\x40tail";
        let mut buf = bytes.as_ref();

        let value = UnknownField::decode(&mut buf, DecodeContext::default()).unwrap();

        assert_eq!(value.number(), 2);
        assert_eq!(value.wire_type(), WireType::ThirtyTwoBit);
        assert_eq!(
            value.value(),
            &UnknownFieldValue::ThirtyTwoBit(*b"\xcd\xcc\x0c\x40")
        );
        assert_eq!(buf, b"tail");

        assert_roundtrip(bytes.strip_suffix(buf).unwrap(), &value);
    }

    #[test]
    fn varint() {
        let bytes = b"\x18\x03tail";
        let mut buf = bytes.as_ref();

        let value = UnknownField::decode(&mut buf, DecodeContext::default()).unwrap();

        assert_eq!(value.number(), 3);
        assert_eq!(value.wire_type(), WireType::Varint);
        assert_eq!(value.value(), &UnknownFieldValue::Varint(3));
        assert_eq!(buf, b"tail");

        assert_roundtrip(bytes.strip_suffix(buf).unwrap(), &value);
    }

    #[test]
    fn length_delimited() {
        let bytes = b"\x7a\x07\x69\xa6\xbe\x6d\xb6\xff\x58tail";
        let mut buf = bytes.as_ref();

        let value = UnknownField::decode(&mut buf, DecodeContext::default()).unwrap();

        assert_eq!(value.number(), 15);
        assert_eq!(value.wire_type(), WireType::LengthDelimited);
        assert_eq!(
            value.value(),
            &UnknownFieldValue::LengthDelimited(Bytes::from_static(
                b"\x69\xa6\xbe\x6d\xb6\xff\x58"
            ))
        );
        assert_eq!(buf, b"tail");

        assert_roundtrip(bytes.strip_suffix(buf).unwrap(), &value);
    }

    #[test]
    fn group() {
        let bytes = b"\x1b\x0a\x05\x68\x65\x6c\x6c\x6f\x10\x0a\x10\x0b\x1ctail";
        let mut buf = bytes.as_ref();

        let value = UnknownField::decode(&mut buf, DecodeContext::default()).unwrap();

        assert_eq!(value.number(), 3);
        assert_eq!(value.wire_type(), WireType::StartGroup);
        assert_eq!(
            value.value(),
            &UnknownFieldValue::Group(UnknownFieldSet::from_iter([
                UnknownField {
                    number: 1,
                    value: UnknownFieldValue::LengthDelimited(Bytes::from_static(b"hello"))
                },
                UnknownField {
                    number: 2,
                    value: UnknownFieldValue::Varint(10)
                },
                UnknownField {
                    number: 2,
                    value: UnknownFieldValue::Varint(11)
                },
            ]))
        );
        assert_eq!(buf, b"tail");

        assert_roundtrip(bytes.strip_suffix(buf).unwrap(), &value);
    }
}
