use std::collections::{hash_map, BTreeMap, HashMap, HashSet};

use crate::{
    descriptor::{
        build::{
            join_path,
            options::option_to_bool,
            visit::{visit, Visitor},
            DescriptorPoolOffsets,
        },
        error::{DescriptorError, DescriptorErrorKind, Label},
        tag, to_index,
        types::{
            DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto,
            FileDescriptorProto, MethodDescriptorProto, OneofDescriptorProto,
            ServiceDescriptorProto,
        },
        Definition, DefinitionKind, DescriptorPoolInner, EnumDescriptorInner, EnumIndex,
        EnumValueDescriptorInner, EnumValueIndex, ExtensionIndex, FieldIndex, FileDescriptorInner,
        FileIndex, Identity, MessageDescriptorInner, MessageIndex, MethodIndex,
        OneofDescriptorInner, OneofIndex, ServiceIndex,
    },
    Syntax,
};

impl DescriptorPoolInner {
    pub(super) fn collect_names(
        &mut self,
        offsets: DescriptorPoolOffsets,
        files: &[FileDescriptorProto],
    ) -> Result<(), DescriptorError> {
        let mut visitor = NameVisitor {
            pool: self,
            errors: vec![],
        };
        visit(offsets, files, &mut visitor);
        if visitor.errors.is_empty() {
            Ok(())
        } else {
            Err(DescriptorError::new(visitor.errors))
        }
    }
}

struct NameVisitor<'a> {
    pool: &'a mut DescriptorPoolInner,
    errors: Vec<DescriptorErrorKind>,
}

impl<'a> Visitor for NameVisitor<'a> {
    fn visit_file(&mut self, path: &[i32], index: FileIndex, file: &FileDescriptorProto) {
        debug_assert_eq!(to_index(self.pool.files.len()), index);

        let syntax = match file.syntax.as_deref() {
            None | Some("proto2") => Syntax::Proto2,
            Some("proto3") => Syntax::Proto3,
            Some(syntax) => {
                self.errors.push(DescriptorErrorKind::UnknownSyntax {
                    syntax: syntax.to_owned(),
                    found: Label::new(
                        &self.pool.files,
                        "found here",
                        index,
                        join_path(path, &[tag::file::SYNTAX]),
                    ),
                });
                return;
            }
        };

        if self
            .pool
            .file_names
            .insert(file.name().into(), index)
            .is_some()
        {
            self.errors.push(DescriptorErrorKind::DuplicateFileName {
                name: file.name().to_owned(),
            });
        }
        self.pool.files.push(FileDescriptorInner {
            syntax,
            raw: file.clone(),
            prost: Default::default(), // the prost descriptor is initialized from the internal descriptor once resolution is complete, to avoid needing to duplicate all modifications
            dependencies: Vec::with_capacity(file.dependency.len()),
            transitive_dependencies: HashSet::default(),
        });

        if !file.package().is_empty() {
            for (i, _) in file.package().match_indices('.') {
                self.add_name(
                    index,
                    &file.package()[..i],
                    path,
                    &[tag::file::PACKAGE],
                    DefinitionKind::Package,
                );
            }
            self.add_name(
                index,
                file.package(),
                path,
                &[tag::file::PACKAGE],
                DefinitionKind::Package,
            );
        }
    }

    fn visit_message(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        parent: Option<MessageIndex>,
        index: MessageIndex,
        message: &DescriptorProto,
    ) {
        self.add_name(
            file,
            full_name,
            path,
            &[tag::message::NAME],
            DefinitionKind::Message(index),
        );

        debug_assert_eq!(to_index(self.pool.messages.len()), index);
        self.pool.messages.push(MessageDescriptorInner {
            id: Identity::new(file, path, full_name, message.name()),
            fields: Vec::with_capacity(message.field.len()),
            field_numbers: BTreeMap::new(),
            field_names: HashMap::with_capacity(message.field.len()),
            field_json_names: HashMap::with_capacity(message.field.len()),
            oneofs: Vec::with_capacity(message.oneof_decl.len()),
            extensions: Vec::new(),
            parent,
        });

        if self.pool.files[file as usize].syntax != Syntax::Proto2 {
            self.check_message_field_camel_case_names(file, path, message);
        }
    }

    fn visit_field(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        message: MessageIndex,
        index: FieldIndex,
        _: &FieldDescriptorProto,
    ) {
        self.add_name(
            file,
            full_name,
            path,
            &[tag::field::NAME],
            DefinitionKind::Field(message, index),
        );
    }

    fn visit_oneof(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        message: MessageIndex,
        index: OneofIndex,
        oneof: &OneofDescriptorProto,
    ) {
        self.add_name(
            file,
            full_name,
            path,
            &[tag::oneof::NAME],
            DefinitionKind::Oneof(message, index),
        );

        debug_assert_eq!(
            to_index(self.pool.messages[message as usize].oneofs.len()),
            index
        );
        self.pool.messages[message as usize]
            .oneofs
            .push(OneofDescriptorInner {
                id: Identity::new(file, path, full_name, oneof.name()),
                fields: Vec::new(),
            });
    }

    fn visit_service(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        index: ServiceIndex,
        _: &ServiceDescriptorProto,
    ) {
        self.add_name(
            file,
            full_name,
            path,
            &[tag::service::NAME],
            DefinitionKind::Service(index),
        );
    }

    fn visit_method(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        service: ServiceIndex,
        index: MethodIndex,
        _: &MethodDescriptorProto,
    ) {
        self.add_name(
            file,
            full_name,
            path,
            &[tag::service::NAME],
            DefinitionKind::Method(service, index),
        );
    }

    fn visit_enum(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        parent: Option<MessageIndex>,
        index: EnumIndex,
        enum_: &EnumDescriptorProto,
    ) {
        self.add_name(
            file,
            full_name,
            path,
            &[tag::enum_::NAME],
            DefinitionKind::Enum(index),
        );

        if enum_.value.is_empty() {
            self.errors.push(DescriptorErrorKind::EmptyEnum {
                found: Label::new(&self.pool.files, "enum defined here", file, path.into()),
            });
        } else if self.pool.files[file as usize].syntax != Syntax::Proto2
            && enum_.value[0].number() != 0
        {
            self.errors
                .push(DescriptorErrorKind::InvalidProto3EnumDefault {
                    found: Label::new(
                        &self.pool.files,
                        "defined here",
                        file,
                        join_path(path, &[tag::enum_::VALUE, 0, tag::enum_value::NUMBER]),
                    ),
                });
        }

        let allow_alias = enum_.options.as_ref().map_or(false, |o| {
            o.value.allow_alias()
                || o.value.uninterpreted_option.iter().any(|u| {
                    u.name.len() == 1
                        && u.name[0].name_part == "allow_alias"
                        && !u.name[0].is_extension
                        && option_to_bool(u).unwrap_or(false)
                })
        });

        debug_assert_eq!(to_index(self.pool.enums.len()), index);
        self.pool.enums.push(EnumDescriptorInner {
            id: Identity::new(file, path, full_name, enum_.name()),
            parent,
            values: Vec::with_capacity(enum_.value.len()),
            value_numbers: Vec::with_capacity(enum_.value.len()),
            value_names: HashMap::with_capacity(enum_.value.len()),
            allow_alias,
        });
    }

    fn visit_enum_value(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        enum_: EnumIndex,
        index: EnumValueIndex,
        value: &EnumValueDescriptorProto,
    ) {
        self.add_name(
            file,
            full_name,
            path,
            &[tag::enum_value::NAME],
            DefinitionKind::EnumValue(enum_, index),
        );

        debug_assert_eq!(
            to_index(self.pool.enums[enum_ as usize].values.len()),
            index
        );
        self.pool.enums[enum_ as usize]
            .values
            .push(EnumValueDescriptorInner {
                id: Identity::new(file, path, full_name, value.name()),
                number: value.number(),
            });
    }

    fn visit_extension(
        &mut self,
        path: &[i32],
        full_name: &str,
        file: FileIndex,
        _: Option<MessageIndex>,
        index: ExtensionIndex,
        _: &FieldDescriptorProto,
    ) {
        self.add_name(
            file,
            full_name,
            path,
            &[tag::field::NAME],
            DefinitionKind::Extension(index),
        );
    }
}

impl<'a> NameVisitor<'a> {
    fn add_name(
        &mut self,
        file: FileIndex,
        name: &str,
        path1: &[i32],
        path2: &[i32],
        kind: DefinitionKind,
    ) {
        let path = join_path(path1, path2);

        match self.pool.names.entry(name.into()) {
            hash_map::Entry::Vacant(entry) => {
                entry.insert(Definition { file, kind, path });
            }
            hash_map::Entry::Occupied(_) => {
                let entry = &self.pool.names[name];

                if matches!(kind, DefinitionKind::Package)
                    && matches!(entry.kind, DefinitionKind::Package)
                {
                    return;
                }

                self.errors.push(DescriptorErrorKind::DuplicateName {
                    name: name.to_owned(),
                    first: Label::new(
                        &self.pool.files,
                        "first defined here",
                        entry.file,
                        entry.path.clone(),
                    ),
                    second: Label::new(&self.pool.files, "defined again here", file, path),
                })
            }
        }
    }

    fn check_message_field_camel_case_names(
        &mut self,
        file: FileIndex,
        path: &[i32],
        message: &DescriptorProto,
    ) {
        let mut names: HashMap<String, (&str, i32)> = HashMap::new();
        for (index, field) in message.field.iter().enumerate() {
            let name = field.name();
            let index = index as i32;

            match names.entry(to_lower_without_underscores(name)) {
                hash_map::Entry::Occupied(entry) => {
                    self.errors
                        .push(DescriptorErrorKind::DuplicateFieldCamelCaseName {
                            first_name: entry.get().0.to_owned(),
                            first: Label::new(
                                &self.pool.files,
                                "first defined here",
                                file,
                                join_path(
                                    path,
                                    &[tag::message::FIELD, entry.get().1, tag::field::NAME],
                                ),
                            ),
                            second_name: name.to_owned(),
                            second: Label::new(
                                &self.pool.files,
                                "defined again here",
                                file,
                                join_path(path, &[tag::message::FIELD, index, tag::field::NAME]),
                            ),
                        })
                }
                hash_map::Entry::Vacant(entry) => {
                    entry.insert((name, index));
                }
            }
        }
    }
}

fn to_lower_without_underscores(name: &str) -> String {
    name.chars()
        .filter_map(|ch| match ch {
            '_' => None,
            _ => Some(ch.to_ascii_lowercase()),
        })
        .collect()
}
