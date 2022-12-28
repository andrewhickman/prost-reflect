mod names;
mod options;
mod resolve;
mod visit;

use std::sync::Arc;

use crate::{
    descriptor::{
        to_index, types::FileDescriptorProto, DefinitionKind, DescriptorPoolInner, EnumIndex,
        ExtensionIndex, FileIndex, MessageIndex, ServiceIndex,
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

        let result = self.build_files_inner(offsets, files);
        if result.is_err() {
            debug_assert_eq!(Arc::strong_count(&self.inner), 1);
            offsets.rollback(Arc::get_mut(&mut self.inner).unwrap());
        }

        result
    }

    fn build_files_inner<I>(
        &mut self,
        offsets: DescriptorPoolOffsets,
        files: I,
    ) -> Result<(), DescriptorError>
    where
        I: IntoIterator<Item = FileDescriptorProto>,
    {
        let inner = Arc::make_mut(&mut self.inner);
        let deduped_files: Vec<_> = files
            .into_iter()
            .filter(|f| match inner.file_names.get(f.name()) {
                Some(&index) => &inner.files[index as usize].raw != f,
                None => true,
            })
            .collect();

        inner.collect_names(offsets, deduped_files.iter())?;

        inner.resolve_names(offsets, deduped_files.iter())?;

        self.resolve_options(offsets, deduped_files.iter())?;

        debug_assert_eq!(Arc::strong_count(&self.inner), 1);
        let inner = Arc::get_mut(&mut self.inner).unwrap();
        for file in &mut inner.files[offsets.file as usize..] {
            file.prost = file.raw.to_prost();
        }

        Ok(())
    }
}

fn join_path(path1: &[i32], path2: &[i32]) -> Box<[i32]> {
    let mut path = Vec::with_capacity(path1.len() + path2.len());
    path.extend_from_slice(path1);
    path.extend_from_slice(path2);
    path.into_boxed_slice()
}
