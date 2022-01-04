use std::{collections::BTreeMap, borrow::Cow};

use prost::{
    bytes::{Buf, BufMut},
    encoding::{DecodeContext, WireType},
    DecodeError, Message,
};

use crate::{ExtensionDescriptor, Value};

use super::field::DynamicMessageField;

/// A set of extension fields in a protobuf message.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct ExtensionFieldSet {
    fields: BTreeMap<u32, DynamicMessageField<ExtensionDescriptor>>,
}

impl ExtensionFieldSet {
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn has(&self, extension_desc: &ExtensionDescriptor) -> bool {
        self.fields.get(&extension_desc.number()).map(|field| field.has()).unwrap_or(false)
    }

    pub fn get(&self, extension_desc: &ExtensionDescriptor) -> Cow<'_, Value> {
        match self.fields.get(&extension_desc.number()) {
            Some(field) => field.get(),
            None => Cow::Owned(Value::default_value_for_extension(extension_desc)),
        }
    }

    pub fn set(&mut self, extension_desc: &ExtensionDescriptor, value: crate::Value) {
        self.fields.entry(extension_desc.number())
            .or_insert_with(|| DynamicMessageField::new(extension_desc.clone()))
            .set(value);
    }

    pub fn clear(&mut self, extension_desc: &ExtensionDescriptor) {
        self.fields.remove(&extension_desc.number());
    }
}

impl Message for ExtensionFieldSet {
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
        _: u32,
        _: WireType,
        _: &mut B,
        _: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        unimplemented!("extensions are not decoded by default")
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
        self.fields.clear();
    }
}
