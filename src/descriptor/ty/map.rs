use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use crate::DescriptorError;

use super::{Scalar, Type};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct TypeId(usize);

#[derive(Debug)]
pub(crate) struct TypeMap {
    named_types: HashMap<String, TypeId>,
    storage: Vec<Type>,
}

impl TypeMap {
    pub fn new() -> Self {
        let mut result = TypeMap {
            named_types: HashMap::with_capacity(128),
            storage: Vec::with_capacity(128),
        };

        result.add_scalars();

        result
    }

    pub fn shrink_to_fit(&mut self) {
        self.named_types.shrink_to_fit();
        self.storage.shrink_to_fit();
    }

    pub fn add(&mut self, ty: Type) -> TypeId {
        let index = self.storage.len();
        self.storage.push(ty);
        TypeId(index)
    }

    pub fn add_with_name(&mut self, name: String, ty: Type) -> TypeId {
        let id = self.add(ty);
        self.named_types.insert(name, id);
        id
    }

    pub fn try_get_by_name(&self, name: &str) -> Option<TypeId> {
        self.named_types.get(name).copied()
    }

    pub fn get_by_name(&self, name: &str) -> Result<TypeId, DescriptorError> {
        match self.try_get_by_name(name) {
            Some(id) => Ok(id),
            None => Err(DescriptorError::type_not_found(name)),
        }
    }

    pub fn get_scalar(&self, scalar: Scalar) -> TypeId {
        TypeId(scalar as usize)
    }

    fn add_scalars(&mut self) {
        let scalars = [
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

        for scalar in scalars {
            let id = self.add(Type::Scalar(scalar));
            debug_assert_eq!(self.get_scalar(scalar), id);
        }
    }
}

impl Index<TypeId> for TypeMap {
    type Output = Type;

    fn index(&self, index: TypeId) -> &Self::Output {
        &self.storage[index.0]
    }
}

impl IndexMut<TypeId> for TypeMap {
    fn index_mut(&mut self, index: TypeId) -> &mut Self::Output {
        &mut self.storage[index.0]
    }
}

impl TypeId {
    pub const DOUBLE: Self = TypeId(Scalar::Double as usize);
    pub const FLOAT: Self = TypeId(Scalar::Float as usize);
    pub const INT32: Self = TypeId(Scalar::Int32 as usize);
    pub const INT64: Self = TypeId(Scalar::Int64 as usize);
    pub const UINT32: Self = TypeId(Scalar::Uint32 as usize);
    pub const UINT64: Self = TypeId(Scalar::Uint64 as usize);
    pub const SINT32: Self = TypeId(Scalar::Sint32 as usize);
    pub const SINT64: Self = TypeId(Scalar::Sint64 as usize);
    pub const FIXED32: Self = TypeId(Scalar::Fixed32 as usize);
    pub const FIXED64: Self = TypeId(Scalar::Fixed64 as usize);
    pub const SFIXED32: Self = TypeId(Scalar::Sfixed32 as usize);
    pub const SFIXED64: Self = TypeId(Scalar::Sfixed64 as usize);
    pub const BOOL: Self = TypeId(Scalar::Bool as usize);
    pub const STRING: Self = TypeId(Scalar::String as usize);
    pub const BYTES: Self = TypeId(Scalar::Bytes as usize);
}
