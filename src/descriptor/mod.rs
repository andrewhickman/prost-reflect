mod error;
mod service;
mod ty;

pub use self::{
    error::DescriptorError,
    service::{MethodDescriptor, ServiceDescriptor},
    ty::{
        EnumDescriptor, EnumValueDescriptor, FieldDescriptor, Kind, MessageDescriptor,
        OneofDescriptor,
    },
};

use std::{fmt, sync::Arc};

use prost_types::FileDescriptorSet;

use self::service::ServiceDescriptorInner;

pub(crate) const MAP_ENTRY_KEY_NUMBER: u32 = 1;
pub(crate) const MAP_ENTRY_VALUE_NUMBER: u32 = 2;

/// A wrapper around a [`FileDescriptorSet`], which provides convenient APIs for the
/// protobuf message definitions.
///
/// This type is immutable once constructed, and uses reference counting internally so it is
/// cheap to clone.
#[derive(Clone)]
pub struct FileDescriptor {
    inner: Arc<FileDescriptorInner>,
}

struct FileDescriptorInner {
    raw: FileDescriptorSet,
    type_map: ty::TypeMap,
    services: Vec<ServiceDescriptorInner>,
}

impl FileDescriptor {
    /// Create a [`FileDescriptor`] from a [`FileDescriptorSet`].
    ///
    /// This method may return an error if `file_descriptor_set` is invalid, for example
    /// it contains references to types not in the set. If `file_descriptor_set` was created by
    /// the protobuf compiler, these error cases should never occur.
    pub fn new(file_descriptor_set: FileDescriptorSet) -> Result<Self, DescriptorError> {
        let inner = FileDescriptor::from_raw(file_descriptor_set)?;
        Ok(FileDescriptor {
            inner: Arc::new(inner),
        })
    }

    fn from_raw(raw: FileDescriptorSet) -> Result<FileDescriptorInner, DescriptorError> {
        let mut type_map = ty::TypeMap::new();
        type_map.add_files(&raw)?;
        type_map.shrink_to_fit();
        let type_map_ref = &type_map;

        let services = raw
            .file
            .iter()
            .flat_map(|raw_file| {
                raw_file.service.iter().map(move |raw_service| {
                    ServiceDescriptorInner::from_raw(raw_file, raw_service, type_map_ref)
                })
            })
            .collect::<Result<_, _>>()?;

        Ok(FileDescriptorInner {
            raw,
            type_map,
            services,
        })
    }

    /// Gets a reference the [`FileDescriptorSet`] wrapped by this [`FileDescriptor`].
    pub fn file_descriptor_set(&self) -> &FileDescriptorSet {
        &self.inner.raw
    }

    /// Gets an iterator over the services defined in these protobuf files.
    pub fn services(&self) -> impl ExactSizeIterator<Item = ServiceDescriptor> + '_ {
        (0..self.inner.services.len()).map(move |index| ServiceDescriptor::new(self.clone(), index))
    }

    fn messages(&self) -> impl Iterator<Item = MessageDescriptor> + '_ {
        MessageDescriptor::iter(self)
    }

    fn enums(&self) -> impl Iterator<Item = EnumDescriptor> + '_ {
        EnumDescriptor::iter(self)
    }

    /// Gets a [`MessageDescriptor`] by its fully qualified name, for example `PackageName.MessageName`.
    pub fn get_message_by_name(&self, name: &str) -> Option<MessageDescriptor> {
        MessageDescriptor::try_get_by_name(self, name)
    }

    /// Gets an [`EnumDescriptor`] by its fully qualified name, for example `PackageName.EnumName`.
    pub fn get_enum_by_name(&self, name: &str) -> Option<EnumDescriptor> {
        EnumDescriptor::try_get_by_name(self, name)
    }
}

impl fmt::Debug for FileDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileDescriptor")
            .field("services", &debug_fmt_iter(self.services()))
            .field("messages", &debug_fmt_iter(self.messages()))
            .field("enums", &debug_fmt_iter(self.enums()))
            .finish()
    }
}

impl PartialEq for FileDescriptor {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for FileDescriptor {}

fn make_full_name(namespace: &str, name: &str) -> String {
    let namespace = namespace.trim_start_matches('.');
    if namespace.is_empty() {
        name.to_owned()
    } else {
        format!("{}.{}", namespace, name)
    }
}

fn parse_namespace(full_name: &str) -> &str {
    match full_name.rsplit_once('.') {
        Some((namespace, _)) => namespace,
        None => "",
    }
}

fn parse_name(full_name: &str) -> &str {
    match full_name.rsplit_once('.') {
        Some((_, name)) => name,
        None => full_name,
    }
}

fn debug_fmt_iter<I>(i: I) -> impl fmt::Debug
where
    I: Iterator,
    I::Item: fmt::Debug,
{
    struct Wrapper<T>(Vec<T>);

    impl<T> fmt::Debug for Wrapper<T>
    where
        T: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(&self.0).finish()
        }
    }

    Wrapper(i.collect())
}
