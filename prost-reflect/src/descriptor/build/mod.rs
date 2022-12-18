mod names;
mod options;
mod resolve;
mod visit;

use prost_types::FileDescriptorProto;

use crate::{
    descriptor::{
        to_index, DefinitionKind, DescriptorPoolInner, EnumIndex, ExtensionIndex, FileIndex,
        MessageIndex, ServiceIndex,
    },
    DescriptorError,
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
                f.raw.package().starts_with(name.as_ref())
                    && matches!(
                        f.raw.package().as_bytes().get(name.len()),
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

impl DescriptorPoolInner {
    pub(super) fn build_files<I>(&mut self, files: I) -> Result<(), DescriptorError>
    where
        I: IntoIterator<Item = FileDescriptorProto>, // todo: use custom FileDescriptorProto type to preserve extension options
    {
        let offsets = DescriptorPoolOffsets::new(self);

        let result = self.build_files_inner(offsets, files);
        if result.is_err() {
            offsets.rollback(self);
        }

        result
    }

    fn build_files_inner<I>(
        &mut self,
        offsets: DescriptorPoolOffsets,
        files: I,
    ) -> Result<(), DescriptorError>
    where
        I: IntoIterator<Item = FileDescriptorProto>, // todo: use custom FileDescriptorProto type to preserve extension options
    {
        let deduped_files: Vec<_> = files
            .into_iter()
            .filter(|f| match self.file_names.get(f.name()) {
                Some(&index) => self.files[index as usize].raw != *f,
                None => true,
            })
            .collect();

        self.collect_names(offsets, deduped_files.iter())?;

        self.resolve_names(offsets, deduped_files.iter())?;

        self.resolve_options(offsets, deduped_files.iter());

        Ok(())
    }
}

fn join_path(path1: &[i32], path2: &[i32]) -> Box<[i32]> {
    let mut path = Vec::with_capacity(path1.len() + path2.len());
    path.extend_from_slice(path1);
    path.extend_from_slice(path2);
    path.into_boxed_slice()
}
