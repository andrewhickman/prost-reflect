mod map;

use std::collections::{BTreeMap, HashMap};

use prost_types::{DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorSet};

use crate::{
    descriptor::{Cardinality, MAP_ENTRY_KEY_TAG, MAP_ENTRY_VALUE_TAG},
    DescriptorError,
};

pub(in crate::descriptor) use self::map::{TypeId, TypeMap};

#[derive(Debug)]
pub(in crate::descriptor) enum Type {
    Message(Message),
    Enum(Enum),
    Scalar(Scalar),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(in crate::descriptor) enum Scalar {
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
pub(in crate::descriptor) struct Message {
    #[allow(unused)]
    pub name: String,
    pub is_map_entry: bool,
    pub fields: BTreeMap<u32, MessageField>,
    pub field_names: HashMap<String, u32>,
}

#[derive(Debug)]
pub(in crate::descriptor) struct MessageField {
    pub name: String,
    pub json_name: String,
    pub is_group: bool,
    pub cardinality: Cardinality,
    pub is_packed: bool,
    pub has_presence: bool,
    pub ty: TypeId,
}

#[derive(Debug)]
pub(in crate::descriptor) struct Enum {
    #[allow(unused)]
    name: String,
    #[allow(unused)]
    values: Vec<EnumValue>,
}

#[derive(Debug)]
pub(in crate::descriptor) struct EnumValue {
    #[allow(unused)]
    name: String,
    #[allow(unused)]
    number: i32,
}

impl TypeMap {
    pub fn add_files(&mut self, raw: &FileDescriptorSet) -> Result<(), DescriptorError> {
        let protos = iter_tys(raw)?;

        for (name, proto) in &protos {
            match *proto {
                TyProto::Message {
                    message_proto,
                    syntax,
                } => {
                    self.add_message(name, message_proto, syntax, &protos)?;
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
        syntax: Syntax,
        protos: &HashMap<String, TyProto>,
    ) -> Result<TypeId, DescriptorError> {
        use prost_types::field_descriptor_proto::{Label, Type as ProtoType};

        if let Some(id) = self.try_get_by_name(name) {
            return Ok(id);
        }

        let is_map_entry = match &message_proto.options {
            Some(options) => options.map_entry(),
            None => false,
        };

        let id = self.add_with_name(
            name.to_owned(),
            // Add a dummy value while we handle any recursive references.
            Type::Message(Message {
                name: Default::default(),
                fields: Default::default(),
                field_names: Default::default(),
                is_map_entry,
            }),
        );

        let fields = message_proto
            .field
            .iter()
            .map(|field_proto| {
                let ty = self.add_message_field(field_proto, protos)?;

                let tag = field_proto.number() as u32;

                let cardinality = match field_proto.label() {
                    Label::Optional => Cardinality::Optional,
                    Label::Required => Cardinality::Required,
                    Label::Repeated => Cardinality::Repeated,
                };

                let is_packed = self[ty].is_packable()
                    && (field_proto
                        .options
                        .as_ref()
                        .map_or(syntax == Syntax::Proto3, |options| options.packed()));

                let has_presence = field_proto.proto3_optional()
                    || (cardinality == Cardinality::Optional
                        && (field_proto.r#type() == ProtoType::Message
                            || syntax == Syntax::Proto2));

                let field = MessageField {
                    name: field_proto.name().to_owned(),
                    json_name: field_proto.json_name().to_owned(),
                    is_group: field_proto.r#type() == ProtoType::Group,
                    cardinality,
                    is_packed,
                    has_presence,
                    ty,
                };

                Ok((tag, field))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        let field_names = fields
            .iter()
            .map(|(&tag, field)| (field.name.clone(), tag))
            .collect();

        if is_map_entry
            && (!fields.contains_key(&MAP_ENTRY_KEY_TAG)
                || !fields.contains_key(&MAP_ENTRY_VALUE_TAG))
        {
            return Err(DescriptorError::invalid_map_entry(name));
        }

        self[id] = Type::Message(Message {
            fields,
            field_names,
            name: name.to_owned(),
            is_map_entry,
        });

        Ok(id)
    }

    fn add_message_field(
        &mut self,
        field_proto: &FieldDescriptorProto,
        protos: &HashMap<String, TyProto>,
    ) -> Result<TypeId, DescriptorError> {
        use prost_types::field_descriptor_proto::Type as ProtoType;

        let ty = match field_proto.r#type() {
            ProtoType::Double => self.get_scalar(Scalar::Double),
            ProtoType::Float => self.get_scalar(Scalar::Float),
            ProtoType::Int64 => self.get_scalar(Scalar::Int64),
            ProtoType::Uint64 => self.get_scalar(Scalar::Uint64),
            ProtoType::Int32 => self.get_scalar(Scalar::Int32),
            ProtoType::Fixed64 => self.get_scalar(Scalar::Fixed64),
            ProtoType::Fixed32 => self.get_scalar(Scalar::Fixed32),
            ProtoType::Bool => self.get_scalar(Scalar::Bool),
            ProtoType::String => self.get_scalar(Scalar::String),
            ProtoType::Bytes => self.get_scalar(Scalar::Bytes),
            ProtoType::Uint32 => self.get_scalar(Scalar::Uint32),
            ProtoType::Sfixed32 => self.get_scalar(Scalar::Sfixed32),
            ProtoType::Sfixed64 => self.get_scalar(Scalar::Sfixed64),
            ProtoType::Sint32 => self.get_scalar(Scalar::Sint32),
            ProtoType::Sint64 => self.get_scalar(Scalar::Sint64),
            ProtoType::Enum | ProtoType::Message | ProtoType::Group => {
                match protos.get(field_proto.type_name()) {
                    None => return Err(DescriptorError::type_not_found(field_proto.type_name())),
                    Some(&TyProto::Message {
                        message_proto,
                        syntax,
                    }) => {
                        self.add_message(field_proto.type_name(), message_proto, syntax, protos)?
                    }
                    Some(TyProto::Enum { enum_proto }) => {
                        self.add_enum(field_proto.type_name(), enum_proto)?
                    }
                }
            }
        };

        Ok(ty)
    }

    fn add_enum(
        &mut self,
        name: &str,
        enum_proto: &EnumDescriptorProto,
    ) -> Result<TypeId, DescriptorError> {
        if let Some(id) = self.try_get_by_name(name) {
            return Ok(id);
        }

        let ty = Type::Enum(Enum {
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

impl Type {
    pub(in crate::descriptor) fn as_message(&self) -> Option<&Message> {
        match self {
            Type::Message(message) => Some(message),
            _ => None,
        }
    }

    fn is_packable(&self) -> bool {
        match self {
            Type::Scalar(scalar) => scalar.is_packable(),
            Type::Enum(_) => true,
            _ => false,
        }
    }
}

impl Scalar {
    fn is_packable(&self) -> bool {
        match self {
            Scalar::Double
            | Scalar::Float
            | Scalar::Int32
            | Scalar::Int64
            | Scalar::Uint32
            | Scalar::Uint64
            | Scalar::Sint32
            | Scalar::Sint64
            | Scalar::Fixed32
            | Scalar::Fixed64
            | Scalar::Sfixed32
            | Scalar::Sfixed64
            | Scalar::Bool => true,
            Scalar::String | Scalar::Bytes => false,
        }
    }
}

#[derive(Clone)]
enum TyProto<'a> {
    Message {
        message_proto: &'a DescriptorProto,
        syntax: Syntax,
    },
    Enum {
        enum_proto: &'a EnumDescriptorProto,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Syntax {
    Proto2,
    Proto3,
}

fn iter_tys(raw: &FileDescriptorSet) -> Result<HashMap<String, TyProto<'_>>, DescriptorError> {
    let mut result = HashMap::with_capacity(128);

    for file in &raw.file {
        let syntax = match file.syntax.as_deref() {
            None | Some("proto2") => Syntax::Proto2,
            Some("proto3") => Syntax::Proto3,
            Some(s) => return Err(DescriptorError::unknown_syntax(s)),
        };

        let namespace = match file.package() {
            "" => String::default(),
            package => format!(".{}", package),
        };

        for message_proto in &file.message_type {
            let full_name = format!("{}.{}", namespace, message_proto.name());
            iter_message(&full_name, &mut result, message_proto, syntax)?;
            if result
                .insert(
                    full_name.clone(),
                    TyProto::Message {
                        message_proto,
                        syntax,
                    },
                )
                .is_some()
            {
                return Err(DescriptorError::type_already_exists(full_name));
            }
        }
        for enum_proto in &file.enum_type {
            let full_name = format!("{}.{}", namespace, enum_proto.name());
            if result
                .insert(full_name.clone(), TyProto::Enum { enum_proto })
                .is_some()
            {
                return Err(DescriptorError::type_already_exists(full_name));
            }
        }
    }

    Ok(result)
}

fn iter_message<'a>(
    namespace: &str,
    result: &mut HashMap<String, TyProto<'a>>,
    raw: &'a DescriptorProto,
    syntax: Syntax,
) -> Result<(), DescriptorError> {
    for message_proto in &raw.nested_type {
        let full_name = format!("{}.{}", namespace, message_proto.name());
        iter_message(&full_name, result, message_proto, syntax)?;
        if result
            .insert(
                full_name.clone(),
                TyProto::Message {
                    message_proto,
                    syntax,
                },
            )
            .is_some()
        {
            return Err(DescriptorError::type_already_exists(full_name));
        }
    }

    for enum_proto in &raw.enum_type {
        let full_name = format!("{}.{}", namespace, enum_proto.name());
        if result
            .insert(full_name.clone(), TyProto::Enum { enum_proto })
            .is_some()
        {
            return Err(DescriptorError::type_already_exists(full_name));
        }
    }

    Ok(())
}
