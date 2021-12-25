use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use crate::DescriptorError;

use super::{Scalar, Type};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(in crate::descriptor) struct TypeId(usize);

#[derive(Debug)]
pub(in crate::descriptor) struct TypeMap {
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
