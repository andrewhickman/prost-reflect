mod names;
mod options;
mod resolve;
mod visit;

use core::fmt;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    iter,
    sync::Arc,
};

use crate::{
    descriptor::{
        error::{DescriptorErrorKind, Label},
        to_index,
        types::FileDescriptorProto,
        Definition, DefinitionKind, DescriptorPoolInner, EnumIndex, ExtensionIndex,
        FileDescriptorInner, FileIndex, MessageIndex, ServiceIndex,
    },
    DescriptorError, DescriptorPool,
};

#[derive(Clone, Copy)]
struct DescriptorPoolOffsets {
    file: FileIndex,
    message: MessageIndex,
    enum_: EnumIndex,
    service: ServiceIndex,
    extension: ExtensionIndex,
}

#[derive(Copy, Clone, Debug)]
enum ResolveNameFilter {
    Message,
    Extension,
    FieldType,
}

enum ResolveNameResult<'a, 'b> {
    Found {
        name: Cow<'b, str>,
        def: &'a Definition,
    },
    InvalidType {
        name: Cow<'b, str>,
        def: &'a Definition,
        filter: ResolveNameFilter,
    },
    NotImported {
        name: Cow<'b, str>,
        file: FileIndex,
    },
    NotFound,
}

impl DescriptorPoolOffsets {
    fn new(pool: &DescriptorPoolInner) -> Self {
        DescriptorPoolOffsets {
            file: to_index(pool.files.len()),
            message: to_index(pool.messages.len()),
            enum_: to_index(pool.enums.len()),
            service: to_index(pool.services.len()),
            extension: to_index(pool.extensions.len()),
        }
    }

    fn rollback(&self, pool: &mut DescriptorPoolInner) {
        pool.files.truncate(self.file as usize);
        pool.messages.truncate(self.message as usize);
        pool.enums.truncate(self.enum_ as usize);
        pool.extensions.truncate(self.extension as usize);
        pool.services.truncate(self.service as usize);
        pool.names.retain(|name, definition| match definition.kind {
            DefinitionKind::Package => pool.files.iter().any(|f| {
                f.prost.package().starts_with(name.as_ref())
                    && matches!(
                        f.prost.package().as_bytes().get(name.len()),
                        None | Some(&b'.')
                    )
            }),
            DefinitionKind::Message(message)
            | DefinitionKind::Field(message, _)
            | DefinitionKind::Oneof(message, _) => message < self.message,
            DefinitionKind::Service(service) | DefinitionKind::Method(service, _) => {
                service < self.service
            }
            DefinitionKind::Enum(enum_) | DefinitionKind::EnumValue(enum_, _) => enum_ < self.enum_,
            DefinitionKind::Extension(extension) => extension < self.extension,
        });
        pool.file_names.retain(|_, &mut file| file < self.file);
        for message in &mut pool.messages {
            message.extensions.retain(|&message| message < self.message);
        }
    }
}

impl DescriptorPool {
    pub(super) fn build_files<I>(&mut self, files: I) -> Result<(), DescriptorError>
    where
        I: IntoIterator<Item = FileDescriptorProto>,
    {
        let offsets = DescriptorPoolOffsets::new(&self.inner);

        let deduped_files: Vec<_> = files
            .into_iter()
            .filter(|f| !self.inner.file_names.contains_key(f.name()))
            .collect();

        let result = self.build_files_deduped(offsets, &deduped_files);
        if result.is_err() {
            debug_assert_eq!(Arc::strong_count(&self.inner), 1);
            offsets.rollback(Arc::get_mut(&mut self.inner).unwrap());
        }

        result
    }

    fn build_files_deduped(
        &mut self,
        offsets: DescriptorPoolOffsets,
        deduped_files: &[FileDescriptorProto],
    ) -> Result<(), DescriptorError> {
        if deduped_files.is_empty() {
            return Ok(());
        }

        let inner = Arc::make_mut(&mut self.inner);

        inner.collect_names(offsets, deduped_files)?;

        inner.resolve_names(offsets, deduped_files)?;

        self.resolve_options(offsets, deduped_files)?;

        debug_assert_eq!(Arc::strong_count(&self.inner), 1);
        let inner = Arc::get_mut(&mut self.inner).unwrap();
        for file in &mut inner.files[offsets.file as usize..] {
            file.prost = file.raw.to_prost();
        }

        Ok(())
    }
}

impl ResolveNameFilter {
    fn is_match(&self, def: &DefinitionKind) -> bool {
        matches!(
            (self, def),
            (ResolveNameFilter::Message, DefinitionKind::Message(_))
                | (ResolveNameFilter::Extension, DefinitionKind::Extension(_))
                | (
                    ResolveNameFilter::FieldType,
                    DefinitionKind::Message(_) | DefinitionKind::Enum(_),
                )
        )
    }
}

impl fmt::Display for ResolveNameFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveNameFilter::Message => f.write_str("a message type"),
            ResolveNameFilter::Extension => f.write_str("an extension"),
            ResolveNameFilter::FieldType => f.write_str("a message or enum type"),
        }
    }
}

impl<'a, 'b> ResolveNameResult<'a, 'b> {
    fn new(
        dependencies: &HashSet<FileIndex>,
        names: &'a HashMap<Box<str>, Definition>,
        name: impl Into<Cow<'b, str>>,
        filter: ResolveNameFilter,
    ) -> Self {
        let name = name.into();
        if let Some(def) = names.get(name.as_ref()) {
            if !dependencies.contains(&def.file) {
                ResolveNameResult::NotImported {
                    name,
                    file: def.file,
                }
            } else if !filter.is_match(&def.kind) {
                ResolveNameResult::InvalidType { name, def, filter }
            } else {
                ResolveNameResult::Found { name, def }
            }
        } else {
            ResolveNameResult::NotFound
        }
    }

    fn into_owned(self) -> ResolveNameResult<'a, 'static> {
        match self {
            ResolveNameResult::Found { name, def } => ResolveNameResult::Found {
                name: Cow::Owned(name.into_owned()),
                def,
            },
            ResolveNameResult::InvalidType { name, def, filter } => {
                ResolveNameResult::InvalidType {
                    name: Cow::Owned(name.into_owned()),
                    def,
                    filter,
                }
            }
            ResolveNameResult::NotImported { name, file } => ResolveNameResult::NotImported {
                name: Cow::Owned(name.into_owned()),
                file,
            },
            ResolveNameResult::NotFound => ResolveNameResult::NotFound,
        }
    }

    fn is_found(&self) -> bool {
        matches!(self, ResolveNameResult::Found { .. })
    }

    #[allow(clippy::result_large_err)]
    fn into_result(
        self,
        orig_name: impl Into<String>,
        files: &[FileDescriptorInner],
        found_file: FileIndex,
        found_path1: &[i32],
        found_path2: &[i32],
    ) -> Result<(Cow<'b, str>, &'a Definition), DescriptorErrorKind> {
        match self {
            ResolveNameResult::Found { name, def } => Ok((name, def)),
            ResolveNameResult::InvalidType { name, def, filter } => {
                Err(DescriptorErrorKind::InvalidType {
                    name: name.into_owned(),
                    expected: filter.to_string(),
                    found: Label::new(
                        files,
                        "found here",
                        found_file,
                        join_path(found_path1, found_path2),
                    ),
                    defined: Label::new(files, "defined here", def.file, def.path.clone()),
                })
            }
            ResolveNameResult::NotImported { name, file } => {
                let root_name = files[found_file as usize].raw.name();
                let dep_name = files[file as usize].raw.name();
                Err(DescriptorErrorKind::NameNotFound {
                    found: Label::new(
                        files,
                        "found here",
                        found_file,
                        join_path(found_path1, found_path2),
                    ),
                    help: Some(format!(
                        "'{}' is defined in '{}', which is not imported by '{}'",
                        name, dep_name, root_name
                    )),
                    name: name.into_owned(),
                })
            }
            ResolveNameResult::NotFound => Err(DescriptorErrorKind::NameNotFound {
                name: orig_name.into(),
                found: Label::new(
                    files,
                    "found here",
                    found_file,
                    join_path(found_path1, found_path2),
                ),
                help: None,
            }),
        }
    }
}

fn to_json_name(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut uppercase_next = false;

    for ch in name.chars() {
        if ch == '_' {
            uppercase_next = true
        } else if uppercase_next {
            result.push(ch.to_ascii_uppercase());
            uppercase_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

fn resolve_name<'a, 'b>(
    dependencies: &HashSet<FileIndex>,
    names: &'a HashMap<Box<str>, Definition>,
    scope: &str,
    name: &'b str,
    filter: ResolveNameFilter,
) -> ResolveNameResult<'a, 'b> {
    match name.strip_prefix('.') {
        Some(full_name) => ResolveNameResult::new(dependencies, names, full_name, filter),
        None if scope.is_empty() => ResolveNameResult::new(dependencies, names, name, filter),
        None => resolve_relative_name(dependencies, names, scope, name, filter),
    }
}

fn resolve_relative_name<'a>(
    dependencies: &HashSet<FileIndex>,
    names: &'a HashMap<Box<str>, Definition>,
    scope: &str,
    relative_name: &str,
    filter: ResolveNameFilter,
) -> ResolveNameResult<'a, 'static> {
    let mut err = ResolveNameResult::NotFound;

    for candidate in resolve_relative_name_candidates(scope, relative_name) {
        let res = ResolveNameResult::new(dependencies, names, candidate, filter);
        if res.is_found() {
            return res.into_owned();
        } else if matches!(err, ResolveNameResult::NotFound) {
            err = res;
        }
    }

    err.into_owned()
}

fn resolve_relative_name_candidates<'b: 'c, 'c>(
    scope: &'c str,
    relative_name: &'b str,
) -> impl Iterator<Item = Cow<'b, str>> + 'c {
    iter::once(Cow::Owned(format!("{scope}.{relative_name}")))
        .chain(
            scope
                .rmatch_indices('.')
                .map(move |(i, _)| Cow::Owned(format!("{}.{relative_name}", &scope[..i]))),
        )
        .chain(iter::once(Cow::Borrowed(relative_name)))
}

fn join_path(path1: &[i32], path2: &[i32]) -> Box<[i32]> {
    let mut path = Vec::with_capacity(path1.len() + path2.len());
    path.extend_from_slice(path1);
    path.extend_from_slice(path2);
    path.into_boxed_slice()
}
