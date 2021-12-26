use prost_types::{FileDescriptorProto, MethodDescriptorProto, ServiceDescriptorProto};

use super::{ty, DescriptorError, FileDescriptor, MessageDescriptor};

/// A protobuf service definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDescriptor {
    file_descriptor: FileDescriptor,
    index: usize,
}

#[derive(Debug)]
pub(super) struct ServiceDescriptorInner {
    name: String,
    methods: Vec<MethodDescriptorInner>,
}

/// A method definition for a [`ServiceDescriptor`].
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl ServiceDescriptor {
    /// Create a new [`ServiceDescriptor`] referencing the service at `index` within the given [`FileDescriptor`].
    ///
    /// # Panics
    ///
    /// Panics if `index` is out-of-bounds.
    pub fn new(file_descriptor: FileDescriptor, index: usize) -> Self {
        debug_assert!(index < file_descriptor.services().len());
        ServiceDescriptor {
            file_descriptor,
            index,
        }
    }

    /// Returns the index of this [`ServiceDescriptor`] within the parent [`FileDescriptor`].
    pub fn index(&self) -> usize {
        self.index
    }

    /// Gets a reference to the [`FileDescriptor`] this service is part of.
    pub fn parent_file(&self) -> &FileDescriptor {
        &self.file_descriptor
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
        &self.parent_file().inner.services[self.index]
    }
}

impl ServiceDescriptorInner {
    pub(super) fn from_raw(
        raw_file: &FileDescriptorProto,
        raw_service: &ServiceDescriptorProto,
        type_map: &ty::TypeMap,
    ) -> Result<ServiceDescriptorInner, DescriptorError> {
        let methods = raw_service
            .method
            .iter()
            .map(|raw_method| {
                MethodDescriptorInner::from_raw(raw_file, raw_service, raw_method, type_map)
            })
            .collect::<Result<_, DescriptorError>>()?;
        Ok(ServiceDescriptorInner {
            name: raw_service.name().into(),
            methods,
        })
    }
}

impl MethodDescriptor {
    /// Create a new [`MethodDescriptor`] referencing the method at `index` within the [`ServiceDescriptor`].
    ///
    /// # Panics
    ///
    /// Panics if `index` is out-of-bounds.
    pub fn new(service: ServiceDescriptor, index: usize) -> Self {
        debug_assert!(index < service.methods().len());
        MethodDescriptor { service, index }
    }

    /// Gets the index of the method within the parent [`ServiceDescriptor`].
    pub fn index(&self) -> usize {
        self.index
    }

    /// Gets a reference to the [`ServiceDescriptor`] this method is defined in.
    pub fn parent_service(&self) -> &ServiceDescriptor {
        &self.service
    }

    /// Gets a reference to the [`FileDescriptor`] this method is defined in.
    pub fn parent_file(&self) -> &FileDescriptor {
        self.service.parent_file()
    }

    /// Gets the name of the method.
    pub fn name(&self) -> &str {
        self.inner().name.as_ref()
    }

    /// Gets the request message type of this method.
    pub fn request(&self) -> MessageDescriptor {
        MessageDescriptor {
            file_set: self.parent_file().clone(),
            ty: self.inner().request_ty,
        }
    }

    /// Gets the response message type of this method.
    pub fn response(&self) -> MessageDescriptor {
        MessageDescriptor {
            file_set: self.parent_file().clone(),
            ty: self.inner().response_ty,
        }
    }

    fn inner(&self) -> &MethodDescriptorInner {
        &self.service.inner().methods[self.index]
    }
}

impl MethodDescriptorInner {
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
}
