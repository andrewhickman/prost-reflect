use std::{collections::HashMap, convert::TryInto};

use crate::{
    descriptor::ty::{EnumDescriptorInner, MessageDescriptorInner},
    DescriptorError,
};

use super::{Scalar, Type};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(in crate::descriptor) struct TypeId(TypeKind, u32);

#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum TypeKind {
    Scalar,
    Message,
    Enum,
}

pub(in crate::descriptor) struct TypeMap {
    named_types: HashMap<String, TypeId>,
    messages: Vec<MessageDescriptorInner>,
    enums: Vec<EnumDescriptorInner>,
}

impl TypeMap {
    const SCALARS: &'static [Scalar] = &[
        Scalar::Double,
        Scalar::Float,
        Scalar::Int32,
        Scalar::Int64,
        Scalar::Uint32,
        Scalar::Uint64,
        Scalar::Sint32,
        Scalar::Sint64,
        Scalar::Fixed32,
        Scalar::Fixed64,
        Scalar::Sfixed32,
        Scalar::Sfixed64,
        Scalar::Bool,
        Scalar::String,
        Scalar::Bytes,
    ];

    pub fn new() -> Self {
        TypeMap {
            named_types: HashMap::new(),
            messages: Vec::new(),
            enums: Vec::new(),
        }
    }

    pub fn messages(&self) -> impl ExactSizeIterator<Item = TypeId> + '_ {
        (0..self.messages.len()).map(|id| TypeId(TypeKind::Message, id as u32))
    }

    pub fn enums(&self) -> impl ExactSizeIterator<Item = TypeId> + '_ {
        (0..self.enums.len()).map(|id| TypeId(TypeKind::Message, id as u32))
    }

    pub fn shrink_to_fit(&mut self) {
        self.named_types.shrink_to_fit();
        self.messages.shrink_to_fit();
        self.enums.shrink_to_fit();
    }

    pub(super) fn add_message(&mut self, message: MessageDescriptorInner) -> TypeId {
        let id = self.messages.len().try_into().expect("too many messages");
        self.messages.push(message);
        TypeId(TypeKind::Message, id)
    }

    pub(super) fn add_enum(&mut self, enum_desc: EnumDescriptorInner) -> TypeId {
        let id = self.enums.len().try_into().expect("too many messages");
        self.enums.push(enum_desc);
        TypeId(TypeKind::Enum, id)
    }

    pub(super) fn add_name(&mut self, mut name: &str, ty: TypeId) {
        if name.starts_with('.') {
            name = &name[1..];
        }

        self.named_types.insert(name.to_owned(), ty);
    }

    pub(super) fn get(&self, id: TypeId) -> Type {
        match id.0 {
            TypeKind::Scalar => Type::Scalar(Self::SCALARS[id.1 as usize]),
            TypeKind::Message => Type::Message(&self.messages[id.1 as usize]),
            TypeKind::Enum => Type::Enum(&self.enums[id.1 as usize]),
        }
    }

    pub(super) fn set_message(&mut self, id: TypeId, message: MessageDescriptorInner) {
        assert_eq!(id.0, TypeKind::Message);
        self.messages[id.1 as usize] = message;
    }

    pub fn try_get_by_name(&self, name: &str) -> Option<TypeId> {
        self.named_types.get(name.trim_start_matches('.')).copied()
    }

    pub fn get_by_name(&self, name: &str) -> Result<TypeId, DescriptorError> {
        match self.try_get_by_name(name) {
            Some(id) => Ok(id),
            None => Err(DescriptorError::type_not_found(name)),
        }
    }

    pub(super) fn get_scalar(&self, scalar: Scalar) -> TypeId {
        TypeId(TypeKind::Scalar, scalar as u32)
    }
}
