use std::collections::{BTreeMap, HashMap};

use prost::bytes::Bytes;
use prost_types::{DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorSet};

use crate::{
    descriptor::{
        make_full_name, parse_namespace,
        ty::{
            Cardinality, EnumDescriptorInner, EnumValueDescriptorInner, FieldDescriptorInner,
            MessageDescriptorInner, OneofDescriptorInner, Scalar, Type, TypeId, TypeMap,
        },
        MAP_ENTRY_KEY_NUMBER, MAP_ENTRY_VALUE_NUMBER,
    },
    DescriptorError,
};

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
            Type::Message(MessageDescriptorInner {
                full_name: Default::default(),
                fields: Default::default(),
                field_names: Default::default(),
                field_json_names: Default::default(),
                oneof_decls: Default::default(),
                is_map_entry,
            }),
        );

        let mut oneof_decls: Vec<_> = message_proto
            .oneof_decl
            .iter()
            .map(|oneof| OneofDescriptorInner {
                name: oneof.name().to_owned(),
                full_name: make_full_name(name, oneof.name()),
                fields: Vec::new(),
            })
            .collect();

        let fields = message_proto
            .field
            .iter()
            .map(|field_proto| {
                let ty = self.add_message_field(field_proto, protos)?;

                let number = field_proto.number() as u32;

                let cardinality = match field_proto.label() {
                    Label::Optional => Cardinality::Optional,
                    Label::Required => Cardinality::Required,
                    Label::Repeated => Cardinality::Repeated,
                };

                let is_packed = self.get(ty).is_packable()
                    && (field_proto
                        .options
                        .as_ref()
                        .map_or(syntax == Syntax::Proto3, |options| options.packed()));

                let supports_presence = field_proto.proto3_optional()
                    || field_proto.oneof_index.is_some()
                    || (cardinality != Cardinality::Repeated
                        && (field_proto.r#type() == ProtoType::Message
                            || syntax == Syntax::Proto2));

                let default_value = match &field_proto.default_value {
                    Some(value) => match self.get(ty) {
                        Type::Scalar(Scalar::Double) => {
                            value.parse().map(crate::Value::F64).map_err(|_| ())
                        }
                        Type::Scalar(Scalar::Float) => {
                            value.parse().map(crate::Value::F32).map_err(|_| ())
                        }
                        Type::Scalar(Scalar::Int32)
                        | Type::Scalar(Scalar::Sint32)
                        | Type::Scalar(Scalar::Sfixed32) => {
                            value.parse().map(crate::Value::I32).map_err(|_| ())
                        }
                        Type::Scalar(Scalar::Int64)
                        | Type::Scalar(Scalar::Sint64)
                        | Type::Scalar(Scalar::Sfixed64) => {
                            value.parse().map(crate::Value::I64).map_err(|_| ())
                        }
                        Type::Scalar(Scalar::Uint32) | Type::Scalar(Scalar::Fixed32) => {
                            value.parse().map(crate::Value::U32).map_err(|_| ())
                        }
                        Type::Scalar(Scalar::Uint64) | Type::Scalar(Scalar::Fixed64) => {
                            value.parse().map(crate::Value::U64).map_err(|_| ())
                        }
                        Type::Scalar(Scalar::Bool) => {
                            value.parse().map(crate::Value::Bool).map_err(|_| ())
                        }
                        Type::Scalar(Scalar::String) => Ok(crate::Value::String(value.to_owned())),
                        Type::Scalar(Scalar::Bytes) => {
                            unescape_c_escape_string(value).map(crate::Value::Bytes)
                        }
                        Type::Enum(enum_ty) => enum_ty
                            .values
                            .iter()
                            .find(|(_, v)| &v.name == value)
                            .map(|(&n, _)| crate::Value::EnumNumber(n))
                            .ok_or(()),
                        Type::Message(_) => Err(()),
                    }
                    .map(Some)
                    .map_err(|()| {
                        DescriptorError::invalid_default_value(name, field_proto.name(), value)
                    })?,
                    None => None,
                };

                let oneof_index = match field_proto.oneof_index {
                    Some(index) => {
                        let index = index as usize;
                        if let Some(oneof) = oneof_decls.get_mut(index) {
                            oneof.fields.push(number);
                        } else {
                            return Err(DescriptorError::invalid_oneof_index(
                                name,
                                field_proto.name(),
                            ));
                        }
                        Some(index)
                    }
                    None => None,
                };

                let field = FieldDescriptorInner {
                    name: field_proto.name().to_owned(),
                    full_name: make_full_name(name, field_proto.name()),
                    json_name: field_proto.json_name().to_owned(),
                    is_group: field_proto.r#type() == ProtoType::Group,
                    cardinality,
                    is_packed,
                    supports_presence,
                    default_value,
                    oneof_index,
                    ty,
                };

                Ok((number, field))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        let field_names = fields
            .iter()
            .map(|(&number, field)| (field.name.clone(), number))
            .collect();
        let field_json_names = fields
            .iter()
            .map(|(&number, field)| (field.json_name.clone(), number))
            .collect();

        if is_map_entry
            && (!fields.contains_key(&MAP_ENTRY_KEY_NUMBER)
                || !fields.contains_key(&MAP_ENTRY_VALUE_NUMBER))
        {
            return Err(DescriptorError::invalid_map_entry(name));
        }

        self.set(
            id,
            Type::Message(MessageDescriptorInner {
                fields,
                field_names,
                field_json_names,
                oneof_decls,
                full_name: name.to_owned(),
                is_map_entry,
            }),
        );

        Ok(id)
    }

    fn add_message_field(
        &mut self,
        field_proto: &FieldDescriptorProto,
        protos: &HashMap<String, TyProto>,
    ) -> Result<TypeId, DescriptorError> {
        use prost_types::field_descriptor_proto::Type as ProtoType;

        let type_name = field_proto.type_name().trim_start_matches('.');

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
                match protos.get(type_name) {
                    None => return Err(DescriptorError::type_not_found(type_name)),
                    Some(&TyProto::Message {
                        message_proto,
                        syntax,
                    }) => self.add_message(type_name, message_proto, syntax, protos)?,
                    Some(TyProto::Enum { enum_proto }) => self.add_enum(type_name, enum_proto)?,
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

        let package_name = parse_namespace(name);

        let value_names = enum_proto
            .value
            .iter()
            .map(|value| (value.name().to_owned(), value.number()))
            .collect();

        let ty = EnumDescriptorInner {
            full_name: name.to_owned(),
            value_names,
            values: enum_proto
                .value
                .iter()
                .map(|value_proto| {
                    (
                        value_proto.number(),
                        EnumValueDescriptorInner {
                            name: value_proto.name().to_owned(),
                            full_name: make_full_name(package_name, value_proto.name()),
                        },
                    )
                })
                .collect(),
        };

        if ty.values.is_empty() {
            return Err(DescriptorError::empty_enum());
        }

        Ok(self.add_with_name(name.to_owned(), Type::Enum(ty)))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Syntax {
    Proto2,
    Proto3,
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

fn iter_tys(raw: &FileDescriptorSet) -> Result<HashMap<String, TyProto<'_>>, DescriptorError> {
    let mut result = HashMap::with_capacity(128);

    for file in &raw.file {
        let syntax = match file.syntax.as_deref() {
            None | Some("proto2") => Syntax::Proto2,
            Some("proto3") => Syntax::Proto3,
            Some(s) => return Err(DescriptorError::unknown_syntax(s)),
        };

        let namespace = file.package();

        for message_proto in &file.message_type {
            let full_name = make_full_name(namespace, message_proto.name());
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
            let full_name = make_full_name(namespace, enum_proto.name());
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
        let full_name = make_full_name(namespace, message_proto.name());
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
        let full_name = make_full_name(namespace, enum_proto.name());
        if result
            .insert(full_name.clone(), TyProto::Enum { enum_proto })
            .is_some()
        {
            return Err(DescriptorError::type_already_exists(full_name));
        }
    }

    Ok(())
}

/// From https://github.com/tokio-rs/prost/blob/c3b7037a7f2c56cef327b41ca32a8c4e9ce5a41c/prost-build/src/code_generator.rs#L887
/// Based on [`google::protobuf::UnescapeCEscapeString`][1]
/// [1]: https://github.com/google/protobuf/blob/3.3.x/src/google/protobuf/stubs/strutil.cc#L312-L322
fn unescape_c_escape_string(s: &str) -> Result<Bytes, ()> {
    let src = s.as_bytes();
    let len = src.len();
    let mut dst = Vec::new();

    let mut p = 0;

    while p < len {
        if src[p] != b'\\' {
            dst.push(src[p]);
            p += 1;
        } else {
            p += 1;
            if p == len {
                return Err(());
            }
            match src[p] {
                b'a' => {
                    dst.push(0x07);
                    p += 1;
                }
                b'b' => {
                    dst.push(0x08);
                    p += 1;
                }
                b'f' => {
                    dst.push(0x0C);
                    p += 1;
                }
                b'n' => {
                    dst.push(0x0A);
                    p += 1;
                }
                b'r' => {
                    dst.push(0x0D);
                    p += 1;
                }
                b't' => {
                    dst.push(0x09);
                    p += 1;
                }
                b'v' => {
                    dst.push(0x0B);
                    p += 1;
                }
                b'\\' => {
                    dst.push(0x5C);
                    p += 1;
                }
                b'?' => {
                    dst.push(0x3F);
                    p += 1;
                }
                b'\'' => {
                    dst.push(0x27);
                    p += 1;
                }
                b'"' => {
                    dst.push(0x22);
                    p += 1;
                }
                b'0'..=b'7' => {
                    let mut octal = 0;
                    for _ in 0..3 {
                        if p < len && src[p] >= b'0' && src[p] <= b'7' {
                            octal = octal * 8 + (src[p] - b'0');
                            p += 1;
                        } else {
                            break;
                        }
                    }
                    dst.push(octal);
                }
                b'x' | b'X' => {
                    if p + 2 > len {
                        return Err(());
                    }
                    match u8::from_str_radix(&s[p + 1..p + 3], 16) {
                        Ok(b) => dst.push(b),
                        _ => return Err(()),
                    }
                    p += 3;
                }
                _ => return Err(()),
            }
        }
    }
    Ok(dst.into())
}
