use std::{collections::BTreeMap, rc::Rc};

use prost::bytes::Bytes;
use prost_types::{
    field_descriptor_proto::{self, Label},
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorSet,
};

use crate::{
    descriptor::{
        make_full_name, parse_namespace,
        ty::{
            Cardinality, EnumDescriptorInner, EnumValueDescriptorInner, ExtensionDescriptorInner,
            FieldDescriptorInner, MessageDescriptorInner, OneofDescriptorInner, TypeId, TypeMap,
        },
        MAP_ENTRY_KEY_NUMBER, MAP_ENTRY_VALUE_NUMBER,
    },
    DescriptorError,
};

impl TypeMap {
    pub fn add_files(&mut self, raw: &FileDescriptorSet) -> Result<(), DescriptorError> {
        let mut messages = Vec::new();
        let mut enums = Vec::new();
        let mut extensions = Vec::new();

        self.iter_files(raw, &mut messages, &mut enums, &mut extensions)?;

        for enum_ in enums {
            self.build_enum(enum_)?;
        }

        for message in messages {
            self.build_message(message)?;
        }

        for extension in extensions {
            self.build_extension(extension)?;
        }

        Ok(())
    }

    fn build_message(
        &mut self,
        MessageProto {
            full_name,
            message_proto,
            parent,
            syntax,
        }: MessageProto,
    ) -> Result<(), DescriptorError> {
        let is_map_entry = match &message_proto.options {
            Some(options) => options.map_entry(),
            None => false,
        };

        let mut oneof_decls: Box<[_]> = message_proto
            .oneof_decl
            .iter()
            .map(|oneof| OneofDescriptorInner {
                name: oneof.name().into(),
                full_name: make_full_name(&full_name, oneof.name()),
                fields: Vec::new(),
            })
            .collect();

        let fields = message_proto
            .field
            .iter()
            .map(|field_proto| {
                self.build_message_field(&full_name, field_proto, syntax, &mut oneof_decls)
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
            return Err(DescriptorError::invalid_map_entry(full_name));
        }

        let parent = parent.map(|p| self.try_get_by_name(&p)).transpose()?;

        debug_assert_eq!(
            self.get_by_name(&full_name),
            Some(TypeId::new_message(self.messages.len()))
        );
        self.messages.push(MessageDescriptorInner {
            fields,
            field_names,
            field_json_names,
            oneof_decls,
            full_name,
            parent,
            is_map_entry,
            reserved_names,
            reserved_ranges,
            extension_ranges,
        });

        Ok(())
    }

    fn build_message_field(
        &mut self,
        message_name: &str,
        field_proto: &FieldDescriptorProto,
        syntax: Syntax,
        oneof_decls: &mut [OneofDescriptorInner],
    ) -> Result<(u32, FieldDescriptorInner), DescriptorError> {
        let ty = self.resolve_message_field_type(message_name, field_proto)?;
        let number = field_proto.number() as u32;
        let cardinality = match field_proto.label() {
            Label::Optional => Cardinality::Optional,
            Label::Required => Cardinality::Required,
            Label::Repeated => Cardinality::Repeated,
        };
        let is_packed = cardinality == Cardinality::Repeated
            && ty.is_packable()
            && (field_proto
                .options
                .as_ref()
                .map_or(syntax == Syntax::Proto3, |options| options.packed()));
        let supports_presence = field_proto.proto3_optional()
            || field_proto.oneof_index.is_some()
            || (cardinality != Cardinality::Repeated
                && (ty.is_message() || syntax == Syntax::Proto2));
        let default_value = match &field_proto.default_value {
            Some(value) => match ty.0 {
                field_descriptor_proto::Type::Double => {
                    value.parse().map(crate::Value::F64).map_err(|_| ())
                }
                field_descriptor_proto::Type::Float => {
                    value.parse().map(crate::Value::F32).map_err(|_| ())
                }
                field_descriptor_proto::Type::Int32
                | field_descriptor_proto::Type::Sint32
                | field_descriptor_proto::Type::Sfixed32 => {
                    value.parse().map(crate::Value::I32).map_err(|_| ())
                }
                field_descriptor_proto::Type::Int64
                | field_descriptor_proto::Type::Sint64
                | field_descriptor_proto::Type::Sfixed64 => {
                    value.parse().map(crate::Value::I64).map_err(|_| ())
                }
                field_descriptor_proto::Type::Uint32 | field_descriptor_proto::Type::Fixed32 => {
                    value.parse().map(crate::Value::U32).map_err(|_| ())
                }
                field_descriptor_proto::Type::Uint64 | field_descriptor_proto::Type::Fixed64 => {
                    value.parse().map(crate::Value::U64).map_err(|_| ())
                }
                field_descriptor_proto::Type::Bool => {
                    value.parse().map(crate::Value::Bool).map_err(|_| ())
                }
                field_descriptor_proto::Type::String => Ok(crate::Value::String(value.to_owned())),
                field_descriptor_proto::Type::Bytes => {
                    unescape_c_escape_string(value).map(crate::Value::Bytes)
                }
                field_descriptor_proto::Type::Enum => {
                    let enum_ty = self.get_enum(ty);
                    enum_ty
                        .values
                        .iter()
                        .find(|(_, v)| *v.name == *value)
                        .map(|(&n, _)| crate::Value::EnumNumber(n))
                        .ok_or(())
                }
                field_descriptor_proto::Type::Message | field_descriptor_proto::Type::Group => {
                    Err(())
                }
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
            is_group: field_proto.r#type == Some(field_descriptor_proto::Type::Group as i32),
            cardinality,
            is_packed,
            supports_presence,
            default_value,
            oneof_index,
            ty,
        };
        Ok((number, field))
    }

    fn resolve_message_field_type(
        &mut self,
        namespace: &str,
        field_proto: &FieldDescriptorProto,
    ) -> Result<TypeId, DescriptorError> {
        match field_proto
            .r#type
            .and_then(field_descriptor_proto::Type::from_i32)
        {
            None
            | Some(
                field_descriptor_proto::Type::Message
                | field_descriptor_proto::Type::Group
                | field_descriptor_proto::Type::Enum,
            ) => self.resolve_type_name(namespace, field_proto.type_name()),
            Some(scalar) => Ok(TypeId::new_scalar(scalar)),
        }
    }

    fn build_enum(
        &mut self,
        EnumProto {
            full_name,
            enum_proto,
            parent,
            syntax,
        }: EnumProto,
    ) -> Result<(), DescriptorError> {
        let value_names = enum_proto
            .value
            .iter()
            .map(|value| (value.name().to_owned(), value.number()))
            .collect();

        let package_name = parse_namespace(&full_name);
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

        let parent = parent.map(|p| self.try_get_by_name(&p)).transpose()?;

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

        debug_assert_eq!(
            self.get_by_name(&full_name),
            Some(TypeId::new_enum(self.enums.len()))
        );
        self.enums.push(EnumDescriptorInner {
            full_name,
            parent,
            value_names,
            values,
            default_value,
            reserved_ranges,
            reserved_names,
        });
        Ok(())
    }

    fn build_extension(
        &mut self,
        ExtensionProto {
            namespace,
            field_proto,
            parent,
            syntax,
        }: ExtensionProto,
    ) -> Result<(), DescriptorError> {
        let (number, field) = self.build_message_field(&namespace, field_proto, syntax, &mut [])?;

        let extendee = self.resolve_type_name(&namespace, field_proto.extendee())?;

        let mut json_name = String::with_capacity(2 + field.full_name.len());
        json_name.push('[');
        json_name.push_str(&field.full_name);
        json_name.push(']');
        let json_name = json_name.into_boxed_str();

        let parent = parent.map(|p| self.try_get_by_name(&p)).transpose()?;

        self.extension_names
            .insert(json_name.clone(), self.extensions.len());
        self.extensions.push(ExtensionDescriptorInner {
            field,
            number,
            parent,
            extendee,
            json_name,
        });

        Ok(())
    }

    fn iter_files<'a>(
        &mut self,
        raw: &'a FileDescriptorSet,
        messages: &mut Vec<MessageProto<'a>>,
        enums: &mut Vec<EnumProto<'a>>,
        extensions: &mut Vec<ExtensionProto<'a>>,
    ) -> Result<(), DescriptorError> {
        for file in &raw.file {
            let syntax = match file.syntax.as_deref() {
                None | Some("proto2") => Syntax::Proto2,
                Some("proto3") => Syntax::Proto3,
                Some(s) => return Err(DescriptorError::unknown_syntax(s)),
            };

            let namespace = file.package();

            for message_proto in &file.message_type {
                let full_name = make_full_name(namespace, message_proto.name());
                self.iter_message(
                    &full_name,
                    messages,
                    enums,
                    extensions,
                    message_proto,
                    syntax,
                )?;

                self.add_named_type(
                    full_name.clone(),
                    TypeId::new_message(self.messages.len() + messages.len()),
                )?;
                messages.push(MessageProto {
                    full_name,
                    message_proto,
                    parent: None,
                    syntax,
                });
            }

            for enum_proto in &file.enum_type {
                let full_name = make_full_name(namespace, enum_proto.name());

                self.add_named_type(
                    full_name.clone(),
                    TypeId::new_enum(self.enums.len() + enums.len()),
                )?;
                enums.push(EnumProto {
                    full_name,
                    enum_proto,
                    parent: None,
                    syntax,
                });
            }

            for field_proto in &file.extension {
                extensions.push(ExtensionProto {
                    namespace: namespace.into(),
                    field_proto,
                    parent: None,
                    syntax,
                });
            }
        }

        Ok(())
    }

    fn iter_message<'a>(
        &mut self,
        namespace: &str,
        messages: &mut Vec<MessageProto<'a>>,
        enums: &mut Vec<EnumProto<'a>>,
        extensions: &mut Vec<ExtensionProto<'a>>,
        raw: &'a DescriptorProto,
        syntax: Syntax,
    ) -> Result<(), DescriptorError> {
        for message_proto in &raw.nested_type {
            let full_name = make_full_name(namespace, message_proto.name());
            self.iter_message(
                &full_name,
                messages,
                enums,
                extensions,
                message_proto,
                syntax,
            )?;

            self.add_named_type(
                full_name.clone(),
                TypeId::new_message(self.messages.len() + messages.len()),
            )?;
            messages.push(MessageProto {
                full_name,
                message_proto,
                parent: Some(namespace.into()),
                syntax,
            });
        }

        for enum_proto in &raw.enum_type {
            let full_name = make_full_name(namespace, enum_proto.name());

            self.add_named_type(
                full_name.clone(),
                TypeId::new_enum(self.enums.len() + enums.len()),
            )?;
            enums.push(EnumProto {
                full_name,
                enum_proto,
                parent: Some(namespace.into()),
                syntax,
            });
        }

        for field_proto in &raw.extension {
            extensions.push(ExtensionProto {
                namespace: namespace.into(),
                field_proto,
                parent: Some(namespace.into()),
                syntax,
            });
        }

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Syntax {
    Proto2,
    Proto3,
}

#[derive(Clone)]
struct MessageProto<'a> {
    full_name: Box<str>,
    message_proto: &'a DescriptorProto,
    parent: Option<Box<str>>,
    syntax: Syntax,
}

#[derive(Clone)]
struct EnumProto<'a> {
    full_name: Box<str>,
    enum_proto: &'a EnumDescriptorProto,
    parent: Option<Box<str>>,
    syntax: Syntax,
}

#[derive(Clone)]
struct ExtensionProto<'a> {
    namespace: Box<str>,
    field_proto: &'a FieldDescriptorProto,
    parent: Option<Rc<str>>,
    syntax: Syntax,
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
