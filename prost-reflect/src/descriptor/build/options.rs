use std::sync::Arc;

use prost::{bytes::Bytes, Message};

use crate::{
    descriptor::{
        build::{
            join_path,
            visit::{visit, Visitor},
            DescriptorPoolOffsets,
        },
        error::{DescriptorErrorKind, Label},
        tag,
        types::{
            uninterpreted_option, DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto,
            FieldDescriptorProto, FileDescriptorProto, MethodDescriptorProto, OneofDescriptorProto,
            Options, ServiceDescriptorProto, UninterpretedOption,
        },
        Definition, DefinitionKind, EnumIndex, EnumValueIndex, ExtensionIndex, FieldIndex,
        FileIndex, MessageIndex, MethodIndex, OneofIndex, ServiceIndex, MAP_ENTRY_KEY_NUMBER,
        MAP_ENTRY_VALUE_NUMBER,
    },
    dynamic::{fmt_string, FieldDescriptorLike},
    Cardinality, DescriptorError, DescriptorPool, DynamicMessage, EnumDescriptor,
    ExtensionDescriptor, MapKey, MessageDescriptor, ReflectMessage, Value,
};

use super::resolve_name;

impl DescriptorPool {
    pub(super) fn resolve_options<'a>(
        &mut self,
        offsets: DescriptorPoolOffsets,
        files: impl Iterator<Item = &'a FileDescriptorProto>,
    ) -> Result<(), DescriptorError> {
        debug_assert_eq!(Arc::strong_count(&self.inner), 1);
        let mut visitor = OptionsVisitor {
            pool: self,
            errors: Vec::new(),
            options: Vec::new(),
            locations: Vec::new(),
        };
        visit(offsets, files, &mut visitor);

        if !visitor.errors.is_empty() {
            return Err(DescriptorError::new(visitor.errors));
        }

        debug_assert_eq!(Arc::strong_count(&visitor.pool.inner), 1);
        let inner = Arc::get_mut(&mut visitor.pool.inner).unwrap();
        for (file, path, encoded) in visitor.options {
            let file = &mut inner.files[file as usize].raw;
            set_file_option(file, &path, &encoded);
        }

        for (file, from, to) in visitor.locations {
            let file = &mut inner.files[file as usize].raw;
            if let Some(source_code_info) = &mut file.source_code_info {
                for location in &mut source_code_info.location {
                    if location.path.starts_with(&from) {
                        location.path.splice(..from.len(), to.iter().copied());
                    }
                }
            }
        }

        Ok(())
    }
}

struct OptionsVisitor<'a> {
    pool: &'a mut DescriptorPool,
    errors: Vec<DescriptorErrorKind>,
    options: Vec<(FileIndex, Box<[i32]>, Vec<u8>)>,
    #[allow(clippy::type_complexity)]
    locations: Vec<(FileIndex, Box<[i32]>, Box<[i32]>)>,
}

impl<'a> Visitor for OptionsVisitor<'a> {
    fn visit_file(&mut self, path: &[i32], index: FileIndex, file: &FileDescriptorProto) {
        if let Some(options) = &file.options {
            let path = join_path(path, &[tag::file::OPTIONS]);
            let encoded = self.resolve_options(
                "google.protobuf.FileOptions",
                options,
                &options.value.uninterpreted_option,
                file.package(),
                index,
                &path,
            );
            self.options.push((index, path, encoded));
        }
    }

    fn visit_message(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        _: Option<MessageIndex>,
        _: MessageIndex,
        message: &DescriptorProto,
    ) {
        if let Some(options) = &message.options {
            let path = join_path(path, &[tag::message::OPTIONS]);
            let encoded = self.resolve_options(
                "google.protobuf.MessageOptions",
                options,
                &options.value.uninterpreted_option,
                full_name,
                file,
                &path,
            );
            self.options.push((file, path, encoded));
        }

        for (i, extension_range) in message.extension_range.iter().enumerate() {
            let path = join_path(
                path,
                &[
                    tag::message::EXTENSION_RANGE,
                    i as i32,
                    tag::message::extension_range::OPTIONS,
                ],
            );
            if let Some(options) = &extension_range.options {
                let encoded = self.resolve_options(
                    "google.protobuf.ExtensionRangeOptions",
                    options,
                    &options.value.uninterpreted_option,
                    full_name,
                    file,
                    &path,
                );
                self.options.push((file, path, encoded));
            }
        }
    }

    fn visit_field(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        _: MessageIndex,
        _: FieldIndex,
        field: &FieldDescriptorProto,
    ) {
        if let Some(options) = &field.options {
            let path = join_path(path, &[tag::field::OPTIONS]);
            let encoded = self.resolve_options(
                "google.protobuf.FieldOptions",
                options,
                &options.value.uninterpreted_option,
                full_name,
                file,
                &path,
            );
            self.options.push((file, path, encoded));
        }
    }

    fn visit_oneof(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        _: MessageIndex,
        _: OneofIndex,
        oneof: &OneofDescriptorProto,
    ) {
        if let Some(options) = &oneof.options {
            let path = join_path(path, &[tag::oneof::OPTIONS]);
            let encoded = self.resolve_options(
                "google.protobuf.OneofOptions",
                options,
                &options.value.uninterpreted_option,
                full_name,
                file,
                &path,
            );
            self.options.push((file, path, encoded));
        }
    }

    fn visit_service(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        _: ServiceIndex,
        service: &ServiceDescriptorProto,
    ) {
        if let Some(options) = &service.options {
            let path = join_path(path, &[tag::service::OPTIONS]);
            let encoded = self.resolve_options(
                "google.protobuf.ServiceOptions",
                options,
                &options.value.uninterpreted_option,
                full_name,
                file,
                &path,
            );
            self.options.push((file, path, encoded));
        }
    }

    fn visit_method(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        _: ServiceIndex,
        _: MethodIndex,
        method: &MethodDescriptorProto,
    ) {
        if let Some(options) = &method.options {
            let path = join_path(path, &[tag::method::OPTIONS]);
            let encoded = self.resolve_options(
                "google.protobuf.MethodOptions",
                options,
                &options.value.uninterpreted_option,
                full_name,
                file,
                &path,
            );
            self.options.push((file, path, encoded));
        }
    }

    fn visit_enum(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        _: Option<MessageIndex>,
        _: EnumIndex,
        enum_: &EnumDescriptorProto,
    ) {
        if let Some(options) = &enum_.options {
            let path = join_path(path, &[tag::enum_::OPTIONS]);
            let encoded = self.resolve_options(
                "google.protobuf.EnumOptions",
                options,
                &options.value.uninterpreted_option,
                full_name,
                file,
                &path,
            );
            self.options.push((file, path, encoded));
        }
    }

    fn visit_enum_value(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        _: EnumIndex,
        _: EnumValueIndex,
        value: &EnumValueDescriptorProto,
    ) {
        if let Some(options) = &value.options {
            let path = join_path(path, &[tag::enum_value::OPTIONS]);
            let encoded = self.resolve_options(
                "google.protobuf.EnumValueOptions",
                options,
                &options.value.uninterpreted_option,
                full_name,
                file,
                &path,
            );
            self.options.push((file, path, encoded));
        }
    }

    fn visit_extension(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        _: Option<MessageIndex>,
        _: ExtensionIndex,
        extension: &FieldDescriptorProto,
    ) {
        if let Some(options) = &extension.options {
            let path = join_path(path, &[tag::field::OPTIONS]);
            let encoded = self.resolve_options(
                "google.protobuf.FieldOptions",
                options,
                &options.value.uninterpreted_option,
                full_name,
                file,
                &path,
            );
            self.options.push((file, path, encoded));
        }
    }
}

impl<'a> OptionsVisitor<'a> {
    fn resolve_options<T>(
        &mut self,
        desc_name: &str,
        options: &Options<T>,
        uninterpreted: &[UninterpretedOption],
        scope: &str,
        file: FileIndex,
        path: &[i32],
    ) -> Vec<u8> {
        let desc = self.pool.get_message_by_name(desc_name).unwrap_or_else(|| {
            DescriptorPool::global()
                .get_message_by_name(desc_name)
                .unwrap()
        });

        let mut message = match DynamicMessage::decode(desc, options.encoded.as_slice()) {
            Ok(message) => message,
            Err(err) => {
                self.errors
                    .push(DescriptorErrorKind::DecodeFileDescriptorSet { err });
                return Vec::new();
            }
        };

        for (i, option) in uninterpreted.iter().enumerate() {
            if let Err(err) = self.set_option(
                &mut message,
                option,
                scope,
                file,
                join_path(path, &[tag::UNINTERPRETED_OPTION, i as i32]),
            ) {
                self.errors.push(err);
            }
        }

        message.clear_field_by_number(tag::UNINTERPRETED_OPTION as u32);

        message.encode_to_vec()
    }

    #[allow(clippy::result_large_err)]
    fn set_option(
        &mut self,
        mut message: &mut DynamicMessage,
        option: &UninterpretedOption,
        scope: &str,
        file: FileIndex,
        path: Box<[i32]>,
    ) -> Result<(), DescriptorErrorKind> {
        let mut resolved_path = Vec::with_capacity(path.len() - 2 + option.name.len());
        resolved_path.extend_from_slice(&path[..path.len() - 2]);

        for (i, part) in option.name.iter().enumerate() {
            let is_last = i == option.name.len() - 1;

            let desc = message.descriptor();
            if part.is_extension {
                match self.find_extension(scope, &part.name_part) {
                    Some(extension_desc) => {
                        resolved_path.push(extension_desc.number() as i32);

                        if is_last {
                            if extension_desc.cardinality() != Cardinality::Repeated
                                && message.has_extension(&extension_desc)
                            {
                                return Err(DescriptorErrorKind::DuplicateOption {
                                    name: fmt_option_name(&option.name),
                                    found: Label::new(
                                        &self.pool.inner.files,
                                        "found here",
                                        file,
                                        path,
                                    ),
                                });
                            } else {
                                self.set_field_value(
                                    message.get_extension_mut(&extension_desc),
                                    &mut resolved_path,
                                    &extension_desc,
                                    option,
                                    file,
                                    &path,
                                )?;
                            }
                        } else if let Value::Message(submessage) =
                            message.get_extension_mut(&extension_desc)
                        {
                            message = submessage;
                        } else {
                            return Err(DescriptorErrorKind::InvalidOptionType {
                                name: fmt_option_name(&option.name[..i + 1]),
                                ty: fmt_field_ty(&extension_desc),
                                value: fmt_value(option),
                                is_last,
                                found: Label::new(&self.pool.inner.files, "found here", file, path),
                            });
                        }
                    }
                    None => {
                        return Err(DescriptorErrorKind::OptionNotFound {
                            name: fmt_option_name(&option.name[..i + 1]),
                            found: Label::new(&self.pool.inner.files, "found here", file, path),
                        })
                    }
                }
            } else {
                match desc.get_field_by_name(&part.name_part) {
                    Some(field_desc) => {
                        resolved_path.push(field_desc.number() as i32);

                        if is_last {
                            if field_desc.cardinality() != Cardinality::Repeated
                                && message.has_field(&field_desc)
                            {
                                return Err(DescriptorErrorKind::DuplicateOption {
                                    name: fmt_option_name(&option.name),
                                    found: Label::new(
                                        &self.pool.inner.files,
                                        "found here",
                                        file,
                                        path,
                                    ),
                                });
                            } else {
                                self.set_field_value(
                                    message.get_field_mut(&field_desc),
                                    &mut resolved_path,
                                    &field_desc,
                                    option,
                                    file,
                                    &path,
                                )?;
                            }
                        } else if let Value::Message(submessage) =
                            message.get_field_mut(&field_desc)
                        {
                            message = submessage;
                        } else {
                            return Err(DescriptorErrorKind::InvalidOptionType {
                                name: fmt_option_name(&option.name[..i + 1]),
                                ty: fmt_field_ty(&field_desc),
                                value: fmt_value(option),
                                is_last,
                                found: Label::new(&self.pool.inner.files, "found here", file, path),
                            });
                        }
                    }
                    None => {
                        return Err(DescriptorErrorKind::OptionNotFound {
                            name: fmt_option_name(&option.name[..i + 1]),
                            found: Label::new(&self.pool.inner.files, "found here", file, path),
                        })
                    }
                }
            }
        }

        self.locations.push((file, path, resolved_path.into()));

        Ok(())
    }

    #[allow(clippy::result_large_err)]
    fn set_field_value(
        &self,
        value: &mut Value,
        resolved_path: &mut Vec<i32>,
        desc: &impl FieldDescriptorLike,
        option: &UninterpretedOption,
        file: FileIndex,
        path: &[i32],
    ) -> Result<(), DescriptorErrorKind> {
        let err = |()| DescriptorErrorKind::InvalidOptionType {
            name: fmt_option_name(&option.name),
            ty: fmt_field_ty(desc),
            value: fmt_value(option),
            is_last: true,
            found: Label::new(&self.pool.inner.files, "found here", file, path.into()),
        };

        match value {
            Value::Bool(value) => *value = option_to_bool(option).map_err(err)?,
            Value::I32(value) => *value = option_to_int(option).map_err(err)?,
            Value::I64(value) => *value = option_to_int(option).map_err(err)?,
            Value::U32(value) => *value = option_to_int(option).map_err(err)?,
            Value::U64(value) => *value = option_to_int(option).map_err(err)?,
            Value::F32(value) => *value = option_to_float(option).map_err(err)? as f32,
            Value::F64(value) => *value = option_to_float(option).map_err(err)?,
            Value::String(value) => *value = option_to_string(option).map_err(err)?,
            Value::Bytes(value) => *value = option_to_bytes(option).map_err(err)?,
            Value::EnumNumber(value) => {
                *value = option_to_enum(option, desc.kind().as_enum().unwrap()).map_err(err)?
            }
            Value::Message(value) => {
                *value =
                    option_to_message(option, desc.kind().as_message().unwrap()).map_err(err)?
            }
            Value::List(value) => {
                resolved_path.push(value.len() as i32);

                let mut entry = Value::default_value(&desc.kind());
                self.set_field_value(&mut entry, resolved_path, desc, option, file, path)?;
                value.push(entry);
            }
            Value::Map(value) => {
                let (entry_key, entry_value) =
                    option_to_map_entry(option, desc.kind().as_message().unwrap()).map_err(err)?;
                value.insert(entry_key, entry_value);
            }
        }

        Ok(())
    }

    fn find_extension(&self, scope: &str, name: &str) -> Option<ExtensionDescriptor> {
        match resolve_name(&self.pool.inner.names, scope, name) {
            Some((
                _,
                &Definition {
                    kind: DefinitionKind::Extension(index),
                    ..
                },
            )) => Some(ExtensionDescriptor {
                pool: self.pool.clone(),
                index,
            }),
            _ => None,
        }
    }
}

fn fmt_option_name(parts: &[uninterpreted_option::NamePart]) -> String {
    let mut result = String::new();
    for part in parts {
        if !result.is_empty() {
            result.push('.');
        }
        if part.is_extension {
            result.push('(');
            result.push_str(&part.name_part);
            result.push(')');
        } else {
            result.push_str(&part.name_part);
        }
    }
    result
}

pub(super) fn option_to_bool(option: &UninterpretedOption) -> Result<bool, ()> {
    match option.identifier_value.as_deref() {
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        _ => Err(()),
    }
}

pub(super) fn option_to_int<T>(option: &UninterpretedOption) -> Result<T, ()>
where
    T: TryFrom<u64> + TryFrom<i64>,
{
    if let Some(int) = option.positive_int_value {
        int.try_into().map_err(drop)
    } else if let Some(int) = option.negative_int_value {
        int.try_into().map_err(drop)
    } else {
        Err(())
    }
}

pub(super) fn option_to_float(option: &UninterpretedOption) -> Result<f64, ()> {
    if let Some(float) = option.double_value {
        Ok(float)
    } else if let Some(int) = option.positive_int_value {
        Ok(int as f64)
    } else if let Some(int) = option.negative_int_value {
        Ok(int as f64)
    } else {
        Err(())
    }
}

pub(super) fn option_to_string(option: &UninterpretedOption) -> Result<String, ()> {
    if let Some(bytes) = &option.string_value {
        String::from_utf8(bytes.clone()).map_err(drop)
    } else {
        Err(())
    }
}

pub(super) fn option_to_bytes(option: &UninterpretedOption) -> Result<Bytes, ()> {
    if let Some(bytes) = &option.string_value {
        Ok(Bytes::copy_from_slice(bytes))
    } else {
        Err(())
    }
}

pub(super) fn option_to_enum(
    option: &UninterpretedOption,
    desc: &EnumDescriptor,
) -> Result<i32, ()> {
    if let Some(ident) = &option.identifier_value {
        if let Some(value) = desc.get_value_by_name(ident) {
            Ok(value.number())
        } else {
            Err(())
        }
    } else {
        Err(())
    }
}

#[cfg(feature = "text-format")]
pub(super) fn option_to_message(
    option: &UninterpretedOption,
    desc: &MessageDescriptor,
) -> Result<DynamicMessage, ()> {
    if let Some(text_format) = &option.aggregate_value {
        DynamicMessage::parse_text_format(desc.clone(), text_format).map_err(drop)
    } else {
        Err(())
    }
}

#[cfg(not(feature = "text-format"))]
pub(super) fn option_to_message(
    option: &UninterpretedOption,
    desc: &MessageDescriptor,
) -> Result<DynamicMessage, ()> {
    if option.aggregate_value.is_some() {
        Ok(DynamicMessage::new(desc.clone()))
    } else {
        Err(())
    }
}

pub(super) fn option_to_map_entry(
    option: &UninterpretedOption,
    desc: &MessageDescriptor,
) -> Result<(MapKey, Value), ()> {
    debug_assert!(desc.is_map_entry());
    let entry = option_to_message(option, desc)?;
    let key = entry
        .get_field_by_number(MAP_ENTRY_KEY_NUMBER)
        .ok_or(())?
        .into_owned()
        .into_map_key()
        .ok_or(())?;
    let value = entry
        .get_field_by_number(MAP_ENTRY_VALUE_NUMBER)
        .ok_or(())?
        .into_owned();
    Ok((key, value))
}

fn fmt_field_ty(field: &impl FieldDescriptorLike) -> String {
    if field.is_map() {
        let entry = field.kind();
        let entry = entry.as_message().unwrap();
        format!(
            "map<{:?}, {:?}>",
            entry.map_entry_key_field().kind(),
            entry.map_entry_value_field().kind()
        )
    } else if field.is_list() {
        format!("repeated {:?}", field.kind())
    } else {
        format!("{:?}", field.kind())
    }
}

fn fmt_value(option: &UninterpretedOption) -> String {
    if let Some(value) = &option.identifier_value {
        value.clone()
    } else if let Some(value) = &option.positive_int_value {
        value.to_string()
    } else if let Some(value) = &option.negative_int_value {
        value.to_string()
    } else if let Some(value) = &option.double_value {
        value.to_string()
    } else if let Some(value) = &option.string_value {
        let mut string = String::new();
        fmt_string(&mut string, value).unwrap();
        string
    } else if let Some(value) = &option.aggregate_value {
        value.clone()
    } else {
        String::new()
    }
}

fn set_file_option(file: &mut FileDescriptorProto, path: &[i32], encoded: &[u8]) {
    match path[0] {
        tag::file::OPTIONS => {
            debug_assert_eq!(path.len(), 1);
            file.options = Some(Options::decode(encoded).unwrap());
        }
        tag::file::MESSAGE_TYPE => {
            let message = &mut file.message_type[path[1] as usize];
            set_message_option(message, &path[2..], encoded);
        }
        tag::file::ENUM_TYPE => {
            let enum_ = &mut file.enum_type[path[1] as usize];
            set_enum_option(enum_, &path[2..], encoded);
        }
        tag::file::SERVICE => {
            let service = &mut file.service[path[1] as usize];
            match path[2] {
                tag::service::OPTIONS => service.options = Some(Options::decode(encoded).unwrap()),
                tag::service::METHOD => {
                    debug_assert_eq!(path.len(), 5);
                    debug_assert_eq!(path[4], tag::method::OPTIONS);
                    let value = &mut service.method[path[3] as usize];
                    value.options = Some(Options::decode(encoded).unwrap());
                }
                p => panic!("unknown path element {}", p),
            }
        }
        tag::file::EXTENSION => {
            debug_assert_eq!(path.len(), 3);
            debug_assert_eq!(path[2], tag::field::OPTIONS);
            let field = &mut file.extension[path[1] as usize];
            field.options = Some(Options::decode(encoded).unwrap());
        }
        p => panic!("unknown path element {}", p),
    }
}

fn set_message_option(message: &mut DescriptorProto, path: &[i32], encoded: &[u8]) {
    match path[0] {
        tag::message::OPTIONS => {
            debug_assert_eq!(path.len(), 1);
            message.options = Some(Options::decode(encoded).unwrap());
        }
        tag::message::EXTENSION_RANGE => {
            debug_assert_eq!(path.len(), 3);
            debug_assert_eq!(path[2], tag::message::extension_range::OPTIONS);
            let extension_range = &mut message.extension_range[path[1] as usize];
            extension_range.options = Some(Options::decode(encoded).unwrap());
        }
        tag::message::FIELD => {
            debug_assert_eq!(path.len(), 3);
            debug_assert_eq!(path[2], tag::field::OPTIONS);
            let field = &mut message.field[path[1] as usize];
            field.options = Some(Options::decode(encoded).unwrap());
        }
        tag::message::ONEOF_DECL => {
            debug_assert_eq!(path.len(), 3);
            debug_assert_eq!(path[2], tag::oneof::OPTIONS);
            let field = &mut message.oneof_decl[path[1] as usize];
            field.options = Some(Options::decode(encoded).unwrap());
        }
        tag::message::NESTED_TYPE => {
            let nested_message = &mut message.nested_type[path[1] as usize];
            set_message_option(nested_message, &path[2..], encoded);
        }
        tag::message::ENUM_TYPE => {
            let enum_ = &mut message.enum_type[path[1] as usize];
            set_enum_option(enum_, &path[2..], encoded);
        }
        tag::message::EXTENSION => {
            debug_assert_eq!(path.len(), 3);
            debug_assert_eq!(path[2], tag::field::OPTIONS);
            let field = &mut message.extension[path[1] as usize];
            field.options = Some(Options::decode(encoded).unwrap());
        }
        p => panic!("unknown path element {}", p),
    }
}

fn set_enum_option(enum_: &mut EnumDescriptorProto, path: &[i32], encoded: &[u8]) {
    match path[0] {
        tag::enum_::OPTIONS => enum_.options = Some(Options::decode(encoded).unwrap()),
        tag::enum_::VALUE => {
            debug_assert_eq!(path.len(), 3);
            debug_assert_eq!(path[2], tag::enum_value::OPTIONS);
            let value = &mut enum_.value[path[1] as usize];
            value.options = Some(Options::decode(encoded).unwrap());
        }
        p => panic!("unknown path element {}", p),
    }
}
