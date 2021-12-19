mod map;

use std::collections::{BTreeMap, HashMap};

use prost_types::{
    field_descriptor_proto, DescriptorProto, EnumDescriptorProto, FieldDescriptorProto,
    FileDescriptorSet,
};

use crate::FileSetError;

pub(crate) use self::map::{TypeId, TypeMap};

#[derive(Debug)]
pub(crate) enum Ty {
    Message(Message),
    Enum(Enum),
    Scalar(Scalar),
    List(TypeId),
    Map(TypeId),
    Group(TypeId),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum Scalar {
    Double = 0,
    Float,
    Int32,
    Int64,
    Uint32,
    Uint64,
    Sint32,
    Sint64,
    Fixed32,
    Fixed64,
    Sfixed32,
    Sfixed64,
    Bool,
    String,
    Bytes,
}

#[derive(Debug)]
pub(crate) struct Message {
    name: String,
    fields: BTreeMap<i32, MessageField>,
}

#[derive(Debug)]
pub(crate) struct MessageField {
    name: String,
    json_name: String,
    is_group: bool,
    ty: TypeId,
}

#[derive(Debug)]
pub(crate) struct Enum {
    name: String,
    values: Vec<EnumValue>,
}

#[derive(Debug)]
pub(crate) struct EnumValue {
    name: String,
    number: i32,
}

impl TypeMap {
    pub fn add_files(&mut self, raw: &FileDescriptorSet) -> Result<(), FileSetError> {
        let protos = iter_tys(raw)?;

        for (name, proto) in &protos {
            match *proto {
                TyProto::Message { message_proto } => {
                    self.add_message(name, message_proto, &protos)?;
                }
                TyProto::Enum { enum_proto } => {
                    self.add_enum(name, enum_proto)?;
                }
            }
        }

        Ok(())
    }

    fn add_message(
        &mut self,
        name: &str,
        message_proto: &DescriptorProto,
        protos: &HashMap<String, TyProto>,
    ) -> Result<TypeId, FileSetError> {
        if let Some(id) = self.try_get_by_name(name) {
            return Ok(id);
        }

        let id = self.add_with_name(
            name.to_owned(),
            // Add a dummy value while we handle any recursive references.
            Ty::Message(Message {
                fields: Default::default(),
                name: Default::default(),
            }),
        );

        let fields = message_proto
            .field
            .iter()
            .map(|field_proto| {
                let ty = self.add_message_field(field_proto, protos)?;

                let tag = field_proto.number();
                let field = MessageField {
                    name: field_proto.name().to_owned(),
                    json_name: field_proto.json_name().to_owned(),
                    is_group: field_proto.r#type() == field_descriptor_proto::Type::Group,
                    ty,
                };

                Ok((tag, field))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        self[id] = Ty::Message(Message {
            fields,
            name: name.to_owned(),
        });

        Ok(id)
    }

    fn add_message_field(
        &mut self,
        field_proto: &FieldDescriptorProto,
        protos: &HashMap<String, TyProto>,
    ) -> Result<TypeId, FileSetError> {
        use prost_types::field_descriptor_proto::{Label, Type};

        let is_repeated = field_proto.label() == Label::Repeated;
        let mut is_map = false;

        let mut base_ty = match field_proto.r#type() {
            Type::Double => self.get_scalar(Scalar::Double),
            Type::Float => self.get_scalar(Scalar::Float),
            Type::Int64 => self.get_scalar(Scalar::Int64),
            Type::Uint64 => self.get_scalar(Scalar::Uint64),
            Type::Int32 => self.get_scalar(Scalar::Int32),
            Type::Fixed64 => self.get_scalar(Scalar::Fixed64),
            Type::Fixed32 => self.get_scalar(Scalar::Fixed32),
            Type::Bool => self.get_scalar(Scalar::Bool),
            Type::String => self.get_scalar(Scalar::String),
            Type::Bytes => self.get_scalar(Scalar::Bytes),
            Type::Uint32 => self.get_scalar(Scalar::Uint32),
            Type::Sfixed32 => self.get_scalar(Scalar::Sfixed32),
            Type::Sfixed64 => self.get_scalar(Scalar::Sfixed64),
            Type::Sint32 => self.get_scalar(Scalar::Sint32),
            Type::Sint64 => self.get_scalar(Scalar::Sint64),
            Type::Enum | Type::Message | Type::Group => match protos.get(field_proto.type_name()) {
                None => return Err(FileSetError::type_not_found(field_proto.type_name())),
                Some(TyProto::Message { message_proto }) => {
                    is_map = match &message_proto.options {
                        Some(options) => options.map_entry(),
                        None => false,
                    };
                    self.add_message(field_proto.type_name(), message_proto, protos)?
                }
                Some(TyProto::Enum { enum_proto }) => {
                    self.add_enum(field_proto.type_name(), enum_proto)?
                }
            },
        };

        if field_proto.r#type() == Type::Group {
            base_ty = self.add(Ty::Group(base_ty));
        }

        if is_map {
            Ok(self.add(Ty::Map(base_ty)))
        } else if is_repeated {
            Ok(self.add(Ty::List(base_ty)))
        } else {
            Ok(base_ty)
        }
    }

    fn add_enum(
        &mut self,
        name: &str,
        enum_proto: &EnumDescriptorProto,
    ) -> Result<TypeId, FileSetError> {
        if let Some(id) = self.try_get_by_name(name) {
            return Ok(id);
        }

        let ty = Ty::Enum(Enum {
            name: name.to_owned(),
            values: enum_proto
                .value
                .iter()
                .map(|value_proto| EnumValue {
                    name: value_proto.name().to_owned(),
                    number: value_proto.number(),
                })
                .collect(),
        });
        Ok(self.add_with_name(name.to_owned(), ty))
    }
}

#[derive(Clone)]
enum TyProto<'a> {
    Message { message_proto: &'a DescriptorProto },
    Enum { enum_proto: &'a EnumDescriptorProto },
}

fn iter_tys(raw: &FileDescriptorSet) -> Result<HashMap<String, TyProto<'_>>, FileSetError> {
    let mut result = HashMap::with_capacity(128);

    for file in &raw.file {
        let namespace = match file.package() {
            "" => String::default(),
            package => format!(".{}", package),
        };

        for message_proto in &file.message_type {
            let full_name = format!("{}.{}", namespace, message_proto.name());
            iter_message(&full_name, &mut result, message_proto)?;
            if result
                .insert(full_name.clone(), TyProto::Message { message_proto })
                .is_some()
            {
                return Err(FileSetError::type_already_exists(full_name));
            }
        }
        for enum_proto in &file.enum_type {
            let full_name = format!("{}.{}", namespace, enum_proto.name());
            if result
                .insert(full_name.clone(), TyProto::Enum { enum_proto })
                .is_some()
            {
                return Err(FileSetError::type_already_exists(full_name));
            }
        }
    }

    Ok(result)
}

fn iter_message<'a>(
    namespace: &str,
    result: &mut HashMap<String, TyProto<'a>>,
    raw: &'a DescriptorProto,
) -> Result<(), FileSetError> {
    for message_proto in &raw.nested_type {
        let full_name = format!("{}.{}", namespace, message_proto.name());
        iter_message(&full_name, result, message_proto)?;
        if result
            .insert(full_name.clone(), TyProto::Message { message_proto })
            .is_some()
        {
            return Err(FileSetError::type_already_exists(full_name));
        }
    }

    for enum_proto in &raw.enum_type {
        let full_name = format!("{}.{}", namespace, enum_proto.name());
        if result
            .insert(full_name.clone(), TyProto::Enum { enum_proto })
            .is_some()
        {
            return Err(FileSetError::type_already_exists(full_name));
        }
    }

    Ok(())
}
