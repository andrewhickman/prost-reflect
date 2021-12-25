pub(crate) mod ty;

use std::{fmt, sync::Arc};

use prost_types::{
    FileDescriptorProto, FileDescriptorSet, MethodDescriptorProto, ServiceDescriptorProto,
};

/// A wrapper around a [`FileDescriptorSet`], which provides convenient APIs for the
/// protobuf message definitions.
///
/// This type is immutable once constructed and uses reference counting internally, so it is
/// cheap to clone.
#[derive(Debug, Clone)]
pub struct FileDescriptor {
    inner: Arc<FileDescriptorInner>,
}

#[derive(Debug)]
struct FileDescriptorInner {
    raw: FileDescriptorSet,
    type_map: ty::TypeMap,
    services: Vec<ServiceDescriptorInner>,
}

/// A protobuf service definition.
#[derive(Debug, Clone)]
pub struct ServiceDescriptor {
    file_set: FileDescriptor,
    index: usize,
}

#[derive(Debug)]
struct ServiceDescriptorInner {
    name: String,
    methods: Vec<MethodDescriptorInner>,
}

/// A method definition for a [`ServiceDescriptor`].
#[derive(Debug, Clone)]
pub struct MethodDescriptor {
    service: ServiceDescriptor,
    index: usize,
}

#[derive(Debug)]
struct MethodDescriptorInner {
    name: String,
    request_ty: ty::TypeId,
    response_ty: ty::TypeId,
}

/// A protobuf message definition.
#[derive(Debug, Clone)]
pub struct Descriptor {
    file_set: FileDescriptor,
    ty: ty::TypeId,
}

/// A protobuf message definition.
#[derive(Debug, Clone)]
pub struct FieldDescriptor {
    message: Descriptor,
    field: u32,
}

/// An error that may occur while creating a [`FileDescriptor`].
#[derive(Debug)]
pub struct DescriptorError {
    kind: DescriptorErrorKind,
}

#[derive(Debug)]
enum DescriptorErrorKind {
    TypeNotFound { name: String },
    TypeAlreadyExists { name: String },
    UnknownSyntax { syntax: String },
    InvalidMapEntry { name: String },
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
                    ServiceDescriptor::from_raw(raw_file, raw_service, type_map_ref)
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
        (0..self.inner.services.len()).map(move |index| ServiceDescriptor {
            file_set: self.clone(),
            index,
        })
    }

    /// Gets a protobuf message by its fully qualified name, for example `.PackageName.MessageName`.
    pub fn get_message_by_name(&self, name: &str) -> Option<Descriptor> {
        let ty = self.inner.type_map.get_by_name(name).ok()?;
        Some(Descriptor {
            file_set: self.clone(),
            ty,
        })
    }
}

impl ServiceDescriptor {
    fn from_raw(
        raw_file: &FileDescriptorProto,
        raw_service: &ServiceDescriptorProto,
        type_map: &ty::TypeMap,
    ) -> Result<ServiceDescriptorInner, DescriptorError> {
        let methods = raw_service
            .method
            .iter()
            .map(|raw_method| {
                MethodDescriptor::from_raw(raw_file, raw_service, raw_method, type_map)
            })
            .collect::<Result<_, DescriptorError>>()?;
        Ok(ServiceDescriptorInner {
            name: raw_service.name().into(),
            methods,
        })
    }

    /// Gets a reference to the [`FileDescriptor`] this service is part of.
    pub fn file_descriptor(&self) -> &FileDescriptor {
        &self.file_set
    }

    /// Gets the name of the service.
    pub fn name(&self) -> &str {
        self.inner().name.as_ref()
    }

    /// Gets an iterator over the methods defined in this service.
    pub fn methods(&self) -> impl ExactSizeIterator<Item = MethodDescriptor> + '_ {
        (0..self.inner().methods.len()).map(move |index| MethodDescriptor {
            service: self.clone(),
            index,
        })
    }

    fn inner(&self) -> &ServiceDescriptorInner {
        &self.file_descriptor().inner.services[self.index]
    }
}

impl MethodDescriptor {
    fn from_raw(
        _raw_file: &FileDescriptorProto,
        _raw_service: &ServiceDescriptorProto,
        raw_method: &MethodDescriptorProto,
        type_map: &ty::TypeMap,
    ) -> Result<MethodDescriptorInner, DescriptorError> {
        let request_ty = type_map.get_by_name(raw_method.input_type())?;
        let response_ty = type_map.get_by_name(raw_method.output_type())?;

        Ok(MethodDescriptorInner {
            name: raw_method.name().to_owned(),
            request_ty,
            response_ty,
        })
    }

    /// Gets a reference to the [`FileDescriptor`] this method is defined in.
    pub fn file_descriptor(&self) -> &FileDescriptor {
        self.service.file_descriptor()
    }

    /// Gets the name of the method.
    pub fn name(&self) -> &str {
        self.inner().name.as_ref()
    }

    /// Gets the request message type of this method.
    pub fn request(&self) -> Descriptor {
        Descriptor {
            file_set: self.file_descriptor().clone(),
            ty: self.inner().request_ty,
        }
    }

    /// Gets the response message type of this method.
    pub fn response(&self) -> Descriptor {
        Descriptor {
            file_set: self.file_descriptor().clone(),
            ty: self.inner().response_ty,
        }
    }

    fn inner(&self) -> &MethodDescriptorInner {
        &self.service.inner().methods[self.index]
    }
}

impl Descriptor {
    pub fn fields(&self) -> impl ExactSizeIterator<Item = FieldDescriptor> + '_ {
        self.message_ty()
            .fields
            .keys()
            .map(move |&field| FieldDescriptor {
                message: self.clone(),
                field,
            })
    }

    pub fn get_field(&self, number: u32) -> Option<FieldDescriptor> {
        if self.message_ty().fields.contains_key(&number) {
            Some(FieldDescriptor {
                message: self.clone(),
                field: number,
            })
        } else {
            None
        }
    }

    fn message_ty(&self) -> &ty::Message {
        self.file_set.inner.type_map[self.ty]
            .as_message()
            .expect("descriptor is not a message type")
    }

    pub(crate) fn type_map(&self) -> &ty::TypeMap {
        &self.file_set.inner.type_map
    }
}

impl FieldDescriptor {
    pub fn tag(&self) -> u32 {
        self.field
    }

    pub fn name(&self) -> &str {
        &self.message_field_ty().name
    }

    pub fn json_name(&self) -> &str {
        &self.message_field_ty().json_name
    }

    pub fn is_group(&self) -> bool {
        self.message_field_ty().is_group
    }

    /// If this field is a message, returns a [`Descriptor`] representing the message type.
    pub fn message_descriptor(&self) -> Option<Descriptor> {
        match self.ty() {
            ty::Type::Message(_) => Some(Descriptor {
                file_set: self.message.file_set.clone(),
                ty: self.message_field_ty().ty,
            }),
            _ => None,
        }
    }

    pub(crate) fn map_entry_descriptor(&self) -> Option<Descriptor> {
        match self.ty() {
            ty::Type::Map(map) => Some(Descriptor {
                file_set: self.message.file_set.clone(),
                ty: map.entry_ty,
            }),
            _ => None,
        }
    }

    pub(crate) fn ty(&self) -> &ty::Type {
        &self.message.type_map()[self.message_field_ty().ty]
    }

    pub(crate) fn ty_id(&self) -> ty::TypeId {
        self.message_field_ty().ty
    }

    fn message_field_ty(&self) -> &ty::MessageField {
        &self.message.message_ty().fields[&self.field]
    }
}

impl std::error::Error for DescriptorError {}

impl fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            DescriptorErrorKind::TypeNotFound { name } => {
                write!(f, "the message or enum type '{}' was not found", name)
            }
            DescriptorErrorKind::TypeAlreadyExists { name } => {
                write!(
                    f,
                    "the message or enum type '{}' is defined multiple times",
                    name
                )
            }
            DescriptorErrorKind::UnknownSyntax { syntax } => {
                write!(f, "the syntax '{}' is not recognized", syntax)
            }
            DescriptorErrorKind::InvalidMapEntry { name } => {
                write!(f, "the map entry message '{}' is invalid", name)
            }
        }
    }
}

impl DescriptorError {
    pub(crate) fn type_not_found(name: impl ToString) -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::TypeNotFound {
                name: name.to_string(),
            },
        }
    }

    pub(crate) fn type_already_exists(name: impl ToString) -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::TypeAlreadyExists {
                name: name.to_string(),
            },
        }
    }

    pub(crate) fn unknown_syntax(syntax: impl ToString) -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::UnknownSyntax {
                syntax: syntax.to_string(),
            },
        }
    }

    pub(crate) fn invalid_map_entry(name: impl ToString) -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::InvalidMapEntry {
                name: name.to_string(),
            },
        }
    }
}
