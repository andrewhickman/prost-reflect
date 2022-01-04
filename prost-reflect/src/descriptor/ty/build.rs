use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

use prost::bytes::Bytes;
use prost_types::{DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorSet};

use crate::{
    descriptor::{
        make_full_name, parse_namespace,
        ty::{
            Cardinality, EnumDescriptorInner, EnumValueDescriptorInner, ExtensionDescriptorInner,
            FieldDescriptorInner, MessageDescriptorInner, OneofDescriptorInner, Scalar, Type,
            TypeId, TypeMap,
        },
        MAP_ENTRY_KEY_NUMBER, MAP_ENTRY_VALUE_NUMBER,
    },
    DescriptorError,
};

impl TypeMap {
    pub fn add_files(&mut self, raw: &FileDescriptorSet) -> Result<(), DescriptorError> {
        let mut protos = HashMap::with_capacity(128);
        let mut extensions = Vec::new();
        iter_tys(raw, &mut protos, &mut extensions)?;

        for name in protos.keys() {
            self.build_named_type(&protos, name)?;
        }

        for ext in extensions {
            self.build_extension(&ext, &protos)?;
        }

        Ok(())
    }

    fn build_message(
        &mut self,
        name: &str,
        parent: Option<&str>,
        message_proto: &DescriptorProto,
        syntax: Syntax,
        protos: &HashMap<Rc<str>, TyProto>,
    ) -> Result<TypeId, DescriptorError> {
        if let Some(id) = self.try_get_by_name(name) {
            return Ok(id);
        }

        let is_map_entry = match &message_proto.options {
            Some(options) => options.map_entry(),
            None => false,
        };

        let id = self.add_message(
            // Add a dummy value while we handle any recursive references.
            MessageDescriptorInner::default(),
        );
        self.add_name(name, id);

        let mut oneof_decls: Box<[_]> = message_proto
            .oneof_decl
            .iter()
            .map(|oneof| OneofDescriptorInner {
                name: oneof.name().into(),
                full_name: make_full_name(name, oneof.name()),
                fields: Vec::new(),
            })
            .collect();

        let fields = message_proto
            .field
            .iter()
            .map(|field_proto| {
                self.build_message_field(name, field_proto, protos, syntax, &mut oneof_decls)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        oneof_decls
            .iter_mut()
            .for_each(|o| o.fields.shrink_to_fit());

        let field_names = fields
            .iter()
            .map(|(&number, field)| (field.name.clone(), number))
            .collect();
        let field_json_names = fields
            .iter()
            .map(|(&number, field)| (field.json_name.clone(), number))
            .collect();

        let reserved_ranges = message_proto
            .reserved_range
            .iter()
            .map(|n| (n.start() as u32)..(n.end() as u32))
            .collect();
        let reserved_names = message_proto
            .reserved_name
            .iter()
            .map(|n| n.as_str().into())
            .collect();
        let extension_ranges = message_proto
            .extension_range
            .iter()
            .map(|n| (n.start() as u32)..(n.end() as u32))
            .collect();

        if is_map_entry
            && (!fields.contains_key(&MAP_ENTRY_KEY_NUMBER)
                || !fields.contains_key(&MAP_ENTRY_VALUE_NUMBER))
        {
            return Err(DescriptorError::invalid_map_entry(name));
        }

        let parent = parent
            .map(|p| self.build_named_type(protos, p))
            .transpose()?;

        self.set_message(
            id,
            MessageDescriptorInner {
                fields,
                field_names,
                field_json_names,
                oneof_decls,
                full_name: name.into(),
                parent,
                is_map_entry,
                reserved_names,
                reserved_ranges,
                extension_ranges,
            },
        );

        Ok(id)
    }

    fn build_message_field(
        &mut self,
        message_name: &str,
        field_proto: &FieldDescriptorProto,
        protos: &HashMap<Rc<str>, TyProto>,
        syntax: Syntax,
        oneof_decls: &mut [OneofDescriptorInner],
    ) -> Result<(u32, FieldDescriptorInner), DescriptorError> {
        use prost_types::field_descriptor_proto::{Label, Type as ProtoType};

        let ty = self.add_message_field(message_name, field_proto, protos)?;
        let number = field_proto.number() as u32;
        let cardinality = match field_proto.label() {
            Label::Optional => Cardinality::Optional,
            Label::Required => Cardinality::Required,
            Label::Repeated => Cardinality::Repeated,
        };
        let is_packed = cardinality == Cardinality::Repeated
            && self.get(ty).is_packable()
            && (field_proto
                .options
                .as_ref()
                .map_or(syntax == Syntax::Proto3, |options| options.packed()));
        let supports_presence = field_proto.proto3_optional()
            || field_proto.oneof_index.is_some()
            || (cardinality != Cardinality::Repeated
                && (field_proto.r#type() == ProtoType::Message || syntax == Syntax::Proto2));
        let default_value = match &field_proto.default_value {
            Some(value) => match self.get(ty) {
                Type::Scalar(Scalar::Double) => {
                    value.parse().map(crate::Value::F64).map_err(|_| ())
                }
                Type::Scalar(Scalar::Float) => value.parse().map(crate::Value::F32).map_err(|_| ()),
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
                Type::Scalar(Scalar::Bool) => value.parse().map(crate::Value::Bool).map_err(|_| ()),
                Type::Scalar(Scalar::String) => Ok(crate::Value::String(value.to_owned())),
                Type::Scalar(Scalar::Bytes) => {
                    unescape_c_escape_string(value).map(crate::Value::Bytes)
                }
                Type::Enum(enum_ty) => enum_ty
                    .values
                    .iter()
                    .find(|(_, v)| *v.name == *value)
                    .map(|(&n, _)| crate::Value::EnumNumber(n))
                    .ok_or(()),
                Type::Message(_) => Err(()),
            }
            .map(Some)
            .map_err(|()| {
                DescriptorError::invalid_default_value(message_name, field_proto.name(), value)
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
                        message_name,
                        field_proto.name(),
                    ));
                }
                Some(index)
            }
            None => None,
        };
        let field = FieldDescriptorInner {
            name: field_proto.name().into(),
            full_name: make_full_name(message_name, field_proto.name()),
            json_name: field_proto.json_name().into(),
            is_group: field_proto.r#type() == ProtoType::Group,
            cardinality,
            is_packed,
            supports_presence,
            default_value,
            oneof_index,
            ty,
        };
        Ok((number, field))
    }

    fn add_message_field(
        &mut self,
        namespace: &str,
        field_proto: &FieldDescriptorProto,
        protos: &HashMap<Rc<str>, TyProto>,
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
                let type_name =
                    self.resolve_type_name(namespace, protos, field_proto.type_name())?;
                self.build_named_type(protos, &type_name)?
            }
        };

        Ok(ty)
    }

    fn resolve_type_name<'a>(
        &self,
        mut namespace: &str,
        protos: &HashMap<Rc<str>, TyProto>,
        type_name: &'a str,
    ) -> Result<Cow<'a, str>, DescriptorError> {
        match type_name.strip_prefix('.') {
            Some(full_name) => Ok(Cow::Borrowed(full_name)),
            None => loop {
                let full_name = make_full_name(namespace, type_name);
                if protos.contains_key(full_name.as_ref()) {
                    break Ok(Cow::Owned(full_name.into()));
                } else if protos.contains_key(namespace) {
                    namespace = parse_namespace(namespace);
                } else {
                    break Err(DescriptorError::type_not_found(type_name));
                }
            },
        }
    }

    fn build_named_type(
        &mut self,
        protos: &HashMap<Rc<str>, TyProto>,
        type_name: &str,
    ) -> Result<TypeId, DescriptorError> {
        Ok(match protos.get(type_name) {
            None => return Err(DescriptorError::type_not_found(type_name)),
            Some(&TyProto::Message {
                message_proto,
                ref parent,
                syntax,
            }) => {
                self.build_message(type_name, parent.as_deref(), message_proto, syntax, protos)?
            }
            Some(&TyProto::Enum {
                enum_proto,
                ref parent,
                syntax,
            }) => self.build_enum(type_name, parent.as_deref(), enum_proto, syntax, protos)?,
        })
    }

    fn build_enum(
        &mut self,
        name: &str,
        parent: Option<&str>,
        enum_proto: &EnumDescriptorProto,
        syntax: Syntax,
        protos: &HashMap<Rc<str>, TyProto>,
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

        let values: BTreeMap<_, _> = enum_proto
            .value
            .iter()
            .map(|value_proto| {
                (
                    value_proto.number(),
                    EnumValueDescriptorInner {
                        name: value_proto.name().into(),
                        full_name: make_full_name(package_name, value_proto.name()),
                    },
                )
            })
            .collect();

        let parent = parent
            .map(|p| self.build_named_type(protos, p))
            .transpose()?;

        let default_value = if syntax == Syntax::Proto2 {
            enum_proto
                .value
                .get(0)
                .map(|v| v.number())
                .ok_or_else(DescriptorError::empty_enum)?
        } else {
            if !values.contains_key(&0) {
                return Err(DescriptorError::empty_enum());
            }

            0
        };

        let reserved_ranges = enum_proto
            .reserved_range
            .iter()
            .map(|n| n.start()..=n.end())
            .collect();
        let reserved_names = enum_proto
            .reserved_name
            .iter()
            .map(|n| n.as_str().into())
            .collect();

        let id = self.add_enum(EnumDescriptorInner {
            full_name: name.into(),
            parent,
            value_names,
            values,
            default_value,
            reserved_ranges,
            reserved_names,
        });
        self.add_name(name, id);
        Ok(id)
    }

    fn build_extension(
        &mut self,
        ext: &ExtProto,
        protos: &HashMap<Rc<str>, TyProto>,
    ) -> Result<(), DescriptorError> {
        let (number, field) =
            self.build_message_field(&ext.namespace, ext.field_proto, protos, ext.syntax, &mut [])?;

        let parent = match &ext.parent {
            Some(parent_name) => Some(self.build_named_type(protos, parent_name)?),
            None => None,
        };

        let extendee_name =
            self.resolve_type_name(&ext.namespace, protos, ext.field_proto.extendee())?;
        let extendee = self.build_named_type(protos, &extendee_name)?;

        self.add_extension(ExtensionDescriptorInner {
            field,
            number,
            parent,
            extendee,
        });

        Ok(())
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
        parent: Option<Rc<str>>,
        syntax: Syntax,
    },
    Enum {
        enum_proto: &'a EnumDescriptorProto,
        parent: Option<Rc<str>>,
        syntax: Syntax,
    },
}

#[derive(Clone)]
struct ExtProto<'a> {
    namespace: Rc<str>,
    field_proto: &'a FieldDescriptorProto,
    parent: Option<Rc<str>>,
    syntax: Syntax,
}

fn iter_tys<'a>(
    raw: &'a FileDescriptorSet,
    result: &mut HashMap<Rc<str>, TyProto<'a>>,
    extensions: &mut Vec<ExtProto<'a>>,
) -> Result<(), DescriptorError> {
    for file in &raw.file {
        let syntax = match file.syntax.as_deref() {
            None | Some("proto2") => Syntax::Proto2,
            Some("proto3") => Syntax::Proto3,
            Some(s) => return Err(DescriptorError::unknown_syntax(s)),
        };

        let namespace = Rc::from(file.package());

        for message_proto in &file.message_type {
            let full_name: Rc<str> = make_full_name(&namespace, message_proto.name()).into();
            iter_message(&full_name, result, extensions, message_proto, syntax)?;
            if result
                .insert(
                    full_name.clone(),
                    TyProto::Message {
                        message_proto,
                        parent: None,
                        syntax,
                    },
                )
                .is_some()
            {
                return Err(DescriptorError::type_already_exists(full_name));
            }
        }

        for enum_proto in &file.enum_type {
            let full_name: Rc<str> = make_full_name(&namespace, enum_proto.name()).into();
            if result
                .insert(
                    full_name.clone(),
                    TyProto::Enum {
                        enum_proto,
                        parent: None,
                        syntax,
                    },
                )
                .is_some()
            {
                return Err(DescriptorError::type_already_exists(full_name));
            }
        }

        for field_proto in &file.extension {
            extensions.push(ExtProto {
                namespace: namespace.clone(),
                field_proto,
                parent: None,
                syntax,
            });
        }
    }

    Ok(())
}

fn iter_message<'a>(
    namespace: &Rc<str>,
    result: &mut HashMap<Rc<str>, TyProto<'a>>,
    extensions: &mut Vec<ExtProto<'a>>,
    raw: &'a DescriptorProto,
    syntax: Syntax,
) -> Result<(), DescriptorError> {
    for message_proto in &raw.nested_type {
        let full_name: Rc<str> = make_full_name(namespace, message_proto.name()).into();
        iter_message(&full_name, result, extensions, message_proto, syntax)?;
        if result
            .insert(
                full_name.clone(),
                TyProto::Message {
                    message_proto,
                    parent: Some(namespace.clone()),
                    syntax,
                },
            )
            .is_some()
        {
            return Err(DescriptorError::type_already_exists(full_name));
        }
    }

    for enum_proto in &raw.enum_type {
        let full_name: Rc<str> = make_full_name(namespace, enum_proto.name()).into();
        if result
            .insert(
                full_name.clone(),
                TyProto::Enum {
                    enum_proto,
                    parent: Some(namespace.clone()),
                    syntax,
                },
            )
            .is_some()
        {
            return Err(DescriptorError::type_already_exists(full_name));
        }
    }

    for field_proto in &raw.extension {
        extensions.push(ExtProto {
            namespace: namespace.clone(),
            field_proto,
            parent: Some(namespace.clone()),
            syntax,
        });
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
