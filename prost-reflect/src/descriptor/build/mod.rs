mod names;
mod options;
mod resolve;
mod visit;

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    descriptor::{
        error::NameNotFoundHelp, to_index, types::FileDescriptorProto, Definition, DefinitionKind,
        DescriptorPoolInner, EnumIndex, ExtensionIndex, FileIndex, MessageIndex, ServiceIndex,
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
) -> Result<(Cow<'b, str>, &'a Definition), NameNotFoundHelp> {
    match name.strip_prefix('.') {
        Some(full_name) => match names.get(full_name) {
            Some(def) => {
                if dependencies.contains(&def.file) {
                    Ok((Cow::Borrowed(full_name), def))
                } else {
                    Err(vec![(full_name.to_owned(), def.file)])
                }
            }
            None => Err(vec![]),
        },
        None => {
            let (resolved_name, def) = resolve_relative_name(dependencies, names, scope, name)?;
            Ok((Cow::Owned(resolved_name), def))
        }
    }
}

fn resolve_relative_name<'a>(
    dependencies: &HashSet<FileIndex>,
    names: &'a HashMap<Box<str>, Definition>,
    scope: &str,
    relative_name: &str,
) -> Result<(String, &'a Definition), NameNotFoundHelp> {
    let mut buf = format!(".{}.{}", scope, relative_name);
    let mut help = Vec::new();

    if let Some(def) = names.get(&buf[1..]) {
        if dependencies.contains(&def.file) {
            return Ok((buf, def));
        } else {
            help.push((buf[1..].to_owned(), def.file));
        }
    }

    for (i, _) in scope.rmatch_indices('.') {
        buf.truncate(i + 2);
        buf.push_str(relative_name);

        if let Some(def) = names.get(&buf[1..]) {
            if dependencies.contains(&def.file) {
                return Ok((buf, def));
            } else {
                help.push((buf[1..].to_owned(), def.file));
            }
        }
    }

    buf.truncate(1);
    buf.push_str(relative_name);
    if let Some(def) = names.get(&buf[1..]) {
        if dependencies.contains(&def.file) {
            return Ok((buf, def));
        } else {
            help.push((buf[1..].to_owned(), def.file));
        }
    }

    Err(help)
}

fn join_path(path1: &[i32], path2: &[i32]) -> Box<[i32]> {
    let mut path = Vec::with_capacity(path1.len() + path2.len());
    path.extend_from_slice(path1);
    path.extend_from_slice(path2);
    path.into_boxed_slice()
}
