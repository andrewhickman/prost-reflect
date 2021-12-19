//! This crate provides support for dynamic protobuf messages. These are useful when the
//! protobuf type definition is not known ahead of time.

#![deny(missing_debug_implementations, missing_docs)]
#![allow(dead_code)]

mod ty;

use std::{fmt, sync::Arc};

use prost_types::{
    FileDescriptorProto, FileDescriptorSet, MethodDescriptorProto, ServiceDescriptorProto,
};

/// A wrapper around a [`FileDescriptorSet`], which provides convenient APIs for the
/// protobuf message definitions.
#[derive(Debug, Clone)]
pub struct FileSet {
    inner: Arc<FileSetInner>,
}

#[derive(Debug)]
struct FileSetInner {
    raw: FileDescriptorSet,
    type_map: ty::TypeMap,
    services: Vec<ServiceInner>,
}

/// A protobuf service definition.
#[derive(Debug, Clone)]
pub struct Service {
    file_set: FileSet,
    index: usize,
}

#[derive(Debug)]
struct ServiceInner {
    name: String,
    methods: Vec<MethodInner>,
}

/// A method definition for a [`Service`].
#[derive(Debug, Clone)]
pub struct Method {
    service: Service,
    index: usize,
}

#[derive(Debug)]
struct MethodInner {
    name: String,
    request_ty: ty::TypeId,
    response_ty: ty::TypeId,
}

/// A protobuf message definition.
#[derive(Debug, Clone)]
pub struct Message {
    file_set: FileSet,
    ty: ty::TypeId,
}

#[derive(Debug)]
struct MessageInner {
    file_set: FileSet,
    index: usize,
}

/// An error that may occur while creating a [`FileSet`].
#[derive(Debug)]
pub struct FileSetError {
    kind: FileSetErrorKind,
}

#[derive(Debug)]
enum FileSetErrorKind {
    TypeNotFound { name: String },
    TypeAlreadyExists { name: String },
}

impl FileSet {
    /// Create a [`FileSet`] from a [`FileDescriptorSet`].
    ///
    /// This method may return an error if `file_descriptor_set` is invalid, for example
    /// it contains references to types not in the set. If `file_descriptor_set` was created by
    /// the protobuf compiler, these error cases should never occur.
    ///
    /// This type is immutable once constructed and uses reference couting internally, so it is
    /// cheap to clone.
    pub fn new(file_descriptor_set: FileDescriptorSet) -> Result<Self, FileSetError> {
        let inner = FileSet::from_raw(file_descriptor_set)?;
        Ok(FileSet {
            inner: Arc::new(inner),
        })
    }

    fn from_raw(raw: FileDescriptorSet) -> Result<FileSetInner, FileSetError> {
        let mut type_map = ty::TypeMap::new();
        type_map.add_files(&raw)?;
        type_map.shrink_to_fit();
        let type_map_ref = &type_map;

        let services =
            raw.file
                .iter()
                .flat_map(|raw_file| {
                    raw_file.service.iter().map(move |raw_service| {
                        Service::from_raw(raw_file, raw_service, type_map_ref)
                    })
                })
                .collect::<Result<_, _>>()?;

        Ok(FileSetInner {
            raw,
            type_map,
            services,
        })
    }

    /// Gets a reference the [`FileDescriptorSet`] wrapped by this [`FileSet`].
    pub fn file_descriptor_set(&self) -> &FileDescriptorSet {
        &self.inner.raw
    }

    /// Gets an iterator over the services defined in these protobuf files.
    pub fn services(&self) -> impl ExactSizeIterator<Item = Service> + '_ {
        (0..self.inner.services.len()).map(move |index| Service {
            file_set: self.clone(),
            index,
        })
    }

    /// Gets a protobuf message by its fully qualified name, for example `.PackageName.MessageName`.
    pub fn get_message_by_name(&self, name: &str) -> Option<Message> {
        let ty = self.inner.type_map.get_by_name(name).ok()?;
        Some(Message {
            file_set: self.clone(),
            ty,
        })
    }
}

impl Service {
    fn from_raw(
        raw_file: &FileDescriptorProto,
        raw_service: &ServiceDescriptorProto,
        type_map: &ty::TypeMap,
    ) -> Result<ServiceInner, FileSetError> {
        let methods = raw_service
            .method
            .iter()
            .map(|raw_method| Method::from_raw(raw_file, raw_service, raw_method, type_map))
            .collect::<Result<_, FileSetError>>()?;
        Ok(ServiceInner {
            name: raw_service.name().into(),
            methods,
        })
    }

    /// Gets a reference to the [`FileSet`] this service is part of.
    pub fn file_set(&self) -> &FileSet {
        &self.file_set
    }

    /// Gets the name of the service.
    pub fn name(&self) -> &str {
        self.inner().name.as_ref()
    }

    /// Gets an iterator over the methods defined in this service.
    pub fn methods(&self) -> impl ExactSizeIterator<Item = Method> + '_ {
        (0..self.inner().methods.len()).map(move |index| Method {
            service: self.clone(),
            index,
        })
    }

    fn inner(&self) -> &ServiceInner {
        &self.file_set().inner.services[self.index]
    }
}

impl Method {
    fn from_raw(
        _raw_file: &FileDescriptorProto,
        _raw_service: &ServiceDescriptorProto,
        raw_method: &MethodDescriptorProto,
        type_map: &ty::TypeMap,
    ) -> Result<MethodInner, FileSetError> {
        let request_ty = type_map.get_by_name(raw_method.input_type())?;
        let response_ty = type_map.get_by_name(raw_method.output_type())?;

        Ok(MethodInner {
            name: raw_method.name().to_owned(),
            request_ty,
            response_ty,
        })
    }

    /// Gets a reference to the [`FileSet`] this method is defined in.
    pub fn file_set(&self) -> &FileSet {
        self.service.file_set()
    }

    /// Gets the name of the method.
    pub fn name(&self) -> &str {
        self.inner().name.as_ref()
    }

    /// Gets the request message type of this method.
    pub fn request(&self) -> Message {
        Message {
            file_set: self.file_set().clone(),
            ty: self.inner().request_ty,
        }
    }

    /// Gets the response message type of this method.
    pub fn response(&self) -> Message {
        Message {
            file_set: self.file_set().clone(),
            ty: self.inner().response_ty,
        }
    }

    fn inner(&self) -> &MethodInner {
        &self.service.inner().methods[self.index]
    }
}

impl std::error::Error for FileSetError {}

impl fmt::Display for FileSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            FileSetErrorKind::TypeNotFound { name } => {
                write!(f, "the message or enum type '{}' was not found", name)
            }
            FileSetErrorKind::TypeAlreadyExists { name } => {
                write!(
                    f,
                    "the message or enum type '{}' is defined multiple times",
                    name
                )
            }
        }
    }
}

impl FileSetError {
    fn type_not_found(name: impl ToString) -> Self {
        FileSetError {
            kind: FileSetErrorKind::TypeNotFound {
                name: name.to_string(),
            },
        }
    }

    fn type_already_exists(name: impl ToString) -> Self {
        FileSetError {
            kind: FileSetErrorKind::TypeAlreadyExists {
                name: name.to_string(),
            },
        }
    }
}
