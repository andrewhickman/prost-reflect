use std::{
    collections::{BTreeMap, HashMap},
    convert::{TryFrom, TryInto},
    rc::Rc,
};

use prost::bytes::Bytes;
use prost_types::{
    field_descriptor_proto::{self, Label},
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto,
};

use crate::{
    descriptor::{
        make_full_name, parse_namespace,
        ty::{
            Cardinality, EnumDescriptorInner, EnumValueDescriptorInner, ExtensionDescriptorInner,
            FieldDescriptorInner, MessageDescriptorInner, OneofDescriptorInner, ParentKind, TypeId,
            TypeMap,
        },
        EnumValueIndex, FileDescriptorInner, FileIndex, Syntax, MAP_ENTRY_KEY_NUMBER,
        MAP_ENTRY_VALUE_NUMBER,
    },
    DescriptorError,
};

impl TypeMap {
    pub fn add_files<'a>(
        &mut self,
        raw: impl Iterator<Item = (FileIndex, &'a FileDescriptorInner)>,
    ) -> Result<(), DescriptorError> {
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
            file,
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

        if is_map_entry
            && (!fields.contains_key(&MAP_ENTRY_KEY_NUMBER)
                || !fields.contains_key(&MAP_ENTRY_VALUE_NUMBER))
        {
            return Err(DescriptorError::invalid_map_entry(full_name));
        }

        let parent = match parent {
            None => ParentKind::File,
            Some(parent_name) => ParentKind::Message {
                index: self.try_get_by_name(&parent_name)?.1,
            },
        };

        debug_assert_eq!(
            self.get_by_name(&full_name),
            Some(TypeId::new_message(self.messages.len()))
        );
        self.messages.push(MessageDescriptorInner {
            file,
            fields,
            field_names,
            field_json_names,
            oneof_decls,
            full_name,
            parent,
            is_map_entry,
            extensions: vec![],
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
                    let enum_ty = self.get_enum(ty.1);
                    enum_ty
                        .value_names
                        .get(value.as_str())
                        .map(|&index| {
                            crate::Value::EnumNumber(enum_ty.values[index as usize].number)
                        })
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
            file,
            full_name,
            enum_proto,
            parent,
            syntax,
        }: EnumProto,
    ) -> Result<(), DescriptorError> {
        let package_name = parse_namespace(&full_name);
        let mut values: Vec<_> = enum_proto
            .value
            .iter()
            .map(|value_proto| EnumValueDescriptorInner {
                name: value_proto.name().into(),
                number: value_proto.number(),
                full_name: make_full_name(package_name, value_proto.name()),
            })
            .collect();
        values.sort_by_key(|v| v.number);

        let value_names: HashMap<Box<str>, EnumValueIndex> = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                (
                    value.name.clone(),
                    EnumValueIndex::try_from(index).expect("index too large"),
                )
            })
            .collect();

        let parent = match parent {
            None => ParentKind::File,
            Some(parent_name) => ParentKind::Message {
                index: self.try_get_by_name(&parent_name)?.1,
            },
        };

        let default_value = if syntax == Syntax::Proto2 {
            let name = enum_proto
                .value
                .get(0)
                .ok_or_else(DescriptorError::empty_enum)?
                .name();
            value_names[name]
        } else {
            values
                .iter()
                .position(|v| v.number == 0)
                .ok_or_else(DescriptorError::empty_enum)?
                .try_into()
                .expect("index too large")
        };

        debug_assert_eq!(
            self.get_by_name(&full_name),
            Some(TypeId::new_enum(self.enums.len()))
        );
        self.enums.push(EnumDescriptorInner {
            file,
            full_name,
            parent,
            value_names,
            values,
            default_value,
        });
        Ok(())
    }

    fn build_extension(
        &mut self,
        ExtensionProto {
            file,
            namespace,
            field_proto,
            parent,
            syntax,
        }: ExtensionProto,
    ) -> Result<(), DescriptorError> {
        let (number, field) = self.build_message_field(&namespace, field_proto, syntax, &mut [])?;

        let extendee = self.resolve_type_name(&namespace, field_proto.extendee())?;
        if !extendee.is_message() {
            return Err(DescriptorError::invalid_extendee_type(
                field.full_name,
                field_proto.extendee(),
            ));
        }

        let mut json_name = String::with_capacity(2 + field.full_name.len());
        json_name.push('[');
        json_name.push_str(&field.full_name);
        json_name.push(']');
        let json_name = json_name.into_boxed_str();

        let parent = match parent {
            None => ParentKind::File,
            Some(parent_name) => ParentKind::Message {
                index: self.try_get_by_name(&parent_name)?.1,
            },
        };

        let index = self.extensions.len().try_into().expect("index too large");
        self.get_message_mut(extendee).extensions.push(index);
        self.extensions.push(ExtensionDescriptorInner {
            file,
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
        raw: impl Iterator<Item = (FileIndex, &'a FileDescriptorInner)>,
        messages: &mut Vec<MessageProto<'a>>,
        enums: &mut Vec<EnumProto<'a>>,
        extensions: &mut Vec<ExtensionProto<'a>>,
    ) -> Result<(), DescriptorError> {
        for (file_index, file) in raw {
            let namespace = file.raw.package();

            for message_proto in &file.raw.message_type {
                let full_name = make_full_name(namespace, message_proto.name());
                self.iter_message(
                    file_index,
                    &full_name,
                    messages,
                    enums,
                    extensions,
                    message_proto,
                    file.syntax,
                )?;

                self.add_named_type(
                    full_name.clone(),
                    TypeId::new_message(self.messages.len() + messages.len()),
                )?;
                messages.push(MessageProto {
                    file: file_index,
                    full_name,
                    message_proto,
                    parent: None,
                    syntax: file.syntax,
                });
            }

            for enum_proto in &file.raw.enum_type {
                let full_name = make_full_name(namespace, enum_proto.name());

                self.add_named_type(
                    full_name.clone(),
                    TypeId::new_enum(self.enums.len() + enums.len()),
                )?;
                enums.push(EnumProto {
                    file: file_index,
                    full_name,
                    enum_proto,
                    parent: None,
                    syntax: file.syntax,
                });
            }

            for field_proto in &file.raw.extension {
                extensions.push(ExtensionProto {
                    file: file_index,
                    namespace: namespace.into(),
                    field_proto,
                    parent: None,
                    syntax: file.syntax,
                });
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn iter_message<'a>(
        &mut self,
        file_index: FileIndex,
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
                file_index,
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
                file: file_index,
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
                file: file_index,
                full_name,
                enum_proto,
                parent: Some(namespace.into()),
                syntax,
            });
        }

        for field_proto in &raw.extension {
            extensions.push(ExtensionProto {
                file: file_index,
                namespace: namespace.into(),
                field_proto,
                parent: Some(namespace.into()),
                syntax,
            });
        }

        Ok(())
    }
}

#[derive(Clone)]
struct MessageProto<'a> {
    file: FileIndex,
    full_name: Box<str>,
    message_proto: &'a DescriptorProto,
    parent: Option<Box<str>>,
    syntax: Syntax,
}

#[derive(Clone)]
struct EnumProto<'a> {
    file: FileIndex,
    full_name: Box<str>,
    enum_proto: &'a EnumDescriptorProto,
    parent: Option<Box<str>>,
    syntax: Syntax,
}

#[derive(Clone)]
struct ExtensionProto<'a> {
    file: FileIndex,
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
                    if p + 3 > len {
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

#[test]
fn test_unescape_c_escape_string() {
    assert_eq!(Ok(Bytes::default()), unescape_c_escape_string(""));
    assert_eq!(
        Ok(Bytes::from_static(b"hello world")),
        unescape_c_escape_string("hello world"),
    );
    assert_eq!(
        Ok(Bytes::from_static(b"\0")),
        unescape_c_escape_string(r#"\0"#),
    );
    assert_eq!(
        Ok(Bytes::from_static(&[0o012, 0o156])),
        unescape_c_escape_string(r#"\012\156"#),
    );
    assert_eq!(
        Ok(Bytes::from_static(&[0x01, 0x02])),
        unescape_c_escape_string(r#"\x01\x02"#)
    );
    assert_eq!(
        Ok(Bytes::from_static(
            b"\0\x01\x07\x08\x0C\n\r\t\x0B\\\'\"\xFE?"
        )),
        unescape_c_escape_string(r#"\0\001\a\b\f\n\r\t\v\\\'\"\xfe\?"#),
    );
    assert_eq!(Err(()), unescape_c_escape_string(r#"\x"#));
    assert_eq!(Err(()), unescape_c_escape_string(r#"\x1"#));
    assert_eq!(
        Ok(Bytes::from_static(b"\x11")),
        unescape_c_escape_string(r#"\x11"#),
    );
    assert_eq!(
        Ok(Bytes::from_static(b"\x111")),
        unescape_c_escape_string(r#"\x111"#),
    );
    assert_eq!(Err(()), unescape_c_escape_string(r#"\w"#));
    assert_eq!(Err(()), unescape_c_escape_string(r#"\x__"#));
}
