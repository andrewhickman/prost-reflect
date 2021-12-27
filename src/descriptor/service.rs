use prost_types::{FileDescriptorProto, MethodDescriptorProto, ServiceDescriptorProto};

use super::{make_full_name, parse_name, ty, DescriptorError, FileDescriptor, MessageDescriptor};

/// A protobuf service definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDescriptor {
    file_descriptor: FileDescriptor,
    index: usize,
}

#[derive(Debug)]
pub(super) struct ServiceDescriptorInner {
    full_name: String,
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
    full_name: String,
    request_ty: ty::TypeId,
    response_ty: ty::TypeId,
    server_streaming: bool,
    client_streaming: bool,
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

    /// Gets a reference to the [`FileDescriptor`] this service is defined in.
    pub fn parent_file(&self) -> &FileDescriptor {
        &self.file_descriptor
    }

    /// Gets the short name of the service, e.g. `MyService`.
    pub fn name(&self) -> &str {
        parse_name(self.full_name())
    }

    /// Gets the full name of the service, e.g. `my.package.Service`.
    pub fn full_name(&self) -> &str {
        &self.inner().full_name
    }

    /// Gets an iterator yielding a [`MethodDescriptor`] for each method defined in this service.
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
        let full_name = make_full_name(raw_file.package(), raw_service.name());
        let methods = raw_service
            .method
            .iter()
            .map(|raw_method| {
                MethodDescriptorInner::from_raw(
                    &full_name,
                    raw_file,
                    raw_service,
                    raw_method,
                    type_map,
                )
            })
            .collect::<Result<_, DescriptorError>>()?;
        Ok(ServiceDescriptorInner { full_name, methods })
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

    /// Gets the short name of the method, e.g. `method`.
    pub fn name(&self) -> &str {
        parse_name(self.full_name())
    }

    /// Gets the full name of the method, e.g. `my.package.MyService.my_method`.
    pub fn full_name(&self) -> &str {
        &self.inner().full_name
    }

    /// Gets the [`MessageDescriptor`] for the input type of this method.
    pub fn input(&self) -> MessageDescriptor {
        MessageDescriptor::new(self.parent_file().clone(), self.inner().request_ty)
    }

    /// Gets the [`MessageDescriptor`] for the output type of this method.
    pub fn output(&self) -> MessageDescriptor {
        MessageDescriptor::new(self.parent_file().clone(), self.inner().response_ty)
    }

    /// Returns `true` if the client streams multiple messages.
    pub fn is_client_streaming(&self) -> bool {
        self.inner().client_streaming
    }

    /// Returns `true` if the server streams multiple messages.
    pub fn is_server_streaming(&self) -> bool {
        self.inner().server_streaming
    }

    fn inner(&self) -> &MethodDescriptorInner {
        &self.service.inner().methods[self.index]
    }
}

impl MethodDescriptorInner {
    fn from_raw(
        namespace: &str,
        _raw_file: &FileDescriptorProto,
        _raw_service: &ServiceDescriptorProto,
        raw_method: &MethodDescriptorProto,
        type_map: &ty::TypeMap,
    ) -> Result<MethodDescriptorInner, DescriptorError> {
        let request_ty = type_map.get_by_name(raw_method.input_type())?;
        let response_ty = type_map.get_by_name(raw_method.output_type())?;

        Ok(MethodDescriptorInner {
            full_name: make_full_name(namespace, raw_method.name()),
            request_ty,
            response_ty,
            client_streaming: raw_method.client_streaming(),
            server_streaming: raw_method.server_streaming(),
        })
    }
}
