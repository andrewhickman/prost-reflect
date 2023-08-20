use std::{
    borrow::Cow,
    fmt, iter,
    ops::{Range, RangeInclusive},
    sync::Arc,
};

use prost::{
    bytes::{Buf, BufMut},
    encoding::{self, WireType},
    DecodeError, EncodeError, Message,
};
use prost_types::{
    DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto,
    FileDescriptorProto, FileDescriptorSet, MethodDescriptorProto, OneofDescriptorProto,
    ServiceDescriptorProto,
};

use crate::{
    descriptor::{
        error::DescriptorErrorKind,
        find_enum_proto, find_message_proto, tag, to_index,
        types::{self, Options},
        Definition, DefinitionKind, DescriptorIndex, EnumDescriptorInner, EnumValueDescriptorInner,
        ExtensionDescriptorInner, FieldDescriptorInner, FileDescriptorInner, KindIndex,
        MessageDescriptorInner, MethodDescriptorInner, OneofDescriptorInner,
        ServiceDescriptorInner, MAP_ENTRY_KEY_NUMBER, MAP_ENTRY_VALUE_NUMBER,
    },
    Cardinality, DescriptorError, DescriptorPool, DynamicMessage, EnumDescriptor,
    EnumValueDescriptor, ExtensionDescriptor, FieldDescriptor, FileDescriptor, Kind,
    MessageDescriptor, MethodDescriptor, OneofDescriptor, ServiceDescriptor, Syntax, Value,
};

impl fmt::Debug for Syntax {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Syntax::Proto2 => write!(f, "proto2"),
            Syntax::Proto3 => write!(f, "proto3"),
        }
    }
}

impl Kind {
    fn new(pool: &DescriptorPool, kind: KindIndex) -> Self {
        match kind {
            KindIndex::Double => Kind::Double,
            KindIndex::Float => Kind::Float,
            KindIndex::Int64 => Kind::Int64,
            KindIndex::Uint64 => Kind::Uint64,
            KindIndex::Int32 => Kind::Int32,
            KindIndex::Fixed64 => Kind::Fixed64,
            KindIndex::Fixed32 => Kind::Fixed32,
            KindIndex::Bool => Kind::Bool,
            KindIndex::String => Kind::String,
            KindIndex::Bytes => Kind::Bytes,
            KindIndex::Uint32 => Kind::Uint32,
            KindIndex::Sfixed32 => Kind::Sfixed32,
            KindIndex::Sfixed64 => Kind::Sfixed64,
            KindIndex::Sint32 => Kind::Sint32,
            KindIndex::Sint64 => Kind::Sint64,
            KindIndex::Message(index) | KindIndex::Group(index) => {
                Kind::Message(MessageDescriptor {
                    pool: pool.clone(),
                    index,
                })
            }
            KindIndex::Enum(index) => Kind::Enum(EnumDescriptor {
                pool: pool.clone(),
                index,
            }),
        }
    }

    /// Gets a reference to the [`MessageDescriptor`] if this is a message type,
    /// or `None` otherwise.
    pub fn as_message(&self) -> Option<&MessageDescriptor> {
        match self {
            Kind::Message(desc) => Some(desc),
            _ => None,
        }
    }

    /// Gets a reference to the [`EnumDescriptor`] if this is an enum type,
    /// or `None` otherwise.
    pub fn as_enum(&self) -> Option<&EnumDescriptor> {
        match self {
            Kind::Enum(desc) => Some(desc),
            _ => None,
        }
    }

    /// Returns the [`WireType`] used to encode this type.
    ///
    /// Note: The [`Kind::Message`] returns [` WireType::LengthDelimited`],
    /// as [groups are deprecated](https://protobuf.dev/programming-guides/encoding/#groups).
    pub fn wire_type(&self) -> WireType {
        match self {
            Kind::Double | Kind::Fixed64 | Kind::Sfixed64 => WireType::SixtyFourBit,
            Kind::Float | Kind::Fixed32 | Kind::Sfixed32 => WireType::ThirtyTwoBit,
            Kind::Enum(_)
            | Kind::Int32
            | Kind::Int64
            | Kind::Uint32
            | Kind::Uint64
            | Kind::Sint32
            | Kind::Sint64
            | Kind::Bool => WireType::Varint,
            Kind::String | Kind::Bytes | Kind::Message(_) => WireType::LengthDelimited,
        }
    }
}

impl fmt::Debug for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Double => write!(f, "double"),
            Self::Float => write!(f, "float"),
            Self::Int32 => write!(f, "int32"),
            Self::Int64 => write!(f, "int64"),
            Self::Uint32 => write!(f, "uint32"),
            Self::Uint64 => write!(f, "uint64"),
            Self::Sint32 => write!(f, "sint32"),
            Self::Sint64 => write!(f, "sint64"),
            Self::Fixed32 => write!(f, "fixed32"),
            Self::Fixed64 => write!(f, "fixed64"),
            Self::Sfixed32 => write!(f, "sfixed32"),
            Self::Sfixed64 => write!(f, "sfixed64"),
            Self::Bool => write!(f, "bool"),
            Self::String => write!(f, "string"),
            Self::Bytes => write!(f, "bytes"),
            Self::Message(m) => write!(f, "{}", m.full_name()),
            Self::Enum(e) => write!(f, "{}", e.full_name()),
        }
    }
}

impl DescriptorPool {
    /// Creates a new, empty [`DescriptorPool`].
    ///
    /// For the common case of creating a `DescriptorPool` from a single [`FileDescriptorSet`], see
    /// [`DescriptorPool::from_file_descriptor_set`] or [`DescriptorPool::decode`].
    pub fn new() -> Self {
        DescriptorPool::default()
    }

    /// Creates a [`DescriptorPool`] from a [`FileDescriptorSet`].
    ///
    /// A file descriptor set may be generated by running the protobuf compiler with the
    /// `--descriptor_set_out` flag. If you are using [`prost-build`](https://crates.io/crates/prost-build),
    /// then [`Config::file_descriptor_set_path`](https://docs.rs/prost-build/latest/prost_build/struct.Config.html#method..file_descriptor_set_path)
    /// is a convenient way to generate it as part of your build.
    pub fn from_file_descriptor_set(
        file_descriptor_set: FileDescriptorSet,
    ) -> Result<Self, DescriptorError> {
        let mut pool = DescriptorPool::new();
        pool.add_file_descriptor_set(file_descriptor_set)?;
        Ok(pool)
    }

    /// Decodes and adds a set of file descriptors to the pool.
    ///
    /// A file descriptor set may be generated by running the protobuf compiler with the
    /// `--descriptor_set_out` flag. If you are using [`prost-build`](https://crates.io/crates/prost-build),
    /// then [`Config::file_descriptor_set_path`](https://docs.rs/prost-build/latest/prost_build/struct.Config.html#method..file_descriptor_set_path)
    /// is a convenient way to generate it as part of your build.
    ///
    /// Unlike when using [`DescriptorPool::from_file_descriptor_set`], any extension options
    /// defined in the file descriptors are preserved.
    ///
    /// # Errors
    ///
    /// Returns an error if the given bytes are not a valid protobuf-encoded file descriptor set, or if the descriptor set itself
    /// is invalid. When using a file descriptor set generated by the protobuf compiler, this method will always succeed.
    pub fn decode<B>(bytes: B) -> Result<Self, DescriptorError>
    where
        B: Buf,
    {
        let file_descriptor_set = types::FileDescriptorSet::decode(bytes).map_err(|err| {
            DescriptorError::new(vec![DescriptorErrorKind::DecodeFileDescriptorSet { err }])
        })?;

        let mut pool = DescriptorPool::new();
        pool.build_files(file_descriptor_set.file.into_iter())?;
        Ok(pool)
    }

    /// Adds a new [`FileDescriptorSet`] to this [`DescriptorPool`].
    ///
    /// A file descriptor set may be generated by running the protobuf compiler with the
    /// `--descriptor_set_out` flag. If you are using [`prost-build`](https://crates.io/crates/prost-build),
    /// then [`Config::file_descriptor_set_path`](https://docs.rs/prost-build/latest/prost_build/struct.Config.html#method..file_descriptor_set_path)
    /// is a convenient way to generate it as part of your build.
    ///
    /// Any duplicates of files already in the pool will be skipped. Note this may cause issues when trying to add two different versions of a file with the same name.
    ///
    /// # Errors
    ///
    /// Returns an error if the descriptor set is invalid, for example if it references types not yet added
    /// to the pool. When using a file descriptor set generated by the protobuf compiler, this method will
    /// always succeed.
    pub fn add_file_descriptor_set(
        &mut self,
        file_descriptor_set: FileDescriptorSet,
    ) -> Result<(), DescriptorError> {
        self.add_file_descriptor_protos(file_descriptor_set.file)
    }

    /// Adds a collection of file descriptors to this pool.
    ///
    /// The file descriptors may be provided in any order, however all types referenced must be defined
    /// either in one of the files provided, or in a file previously added to the pool.
    ///
    /// Any duplicates of files already in the pool will be skipped. Note this may cause issues when trying to add two different versions of a file with the same name.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the given file descriptor is invalid, for example if they reference
    /// types not yet added to the pool.
    pub fn add_file_descriptor_protos<I>(&mut self, files: I) -> Result<(), DescriptorError>
    where
        I: IntoIterator<Item = FileDescriptorProto>,
    {
        self.build_files(
            files
                .into_iter()
                .map(types::FileDescriptorProto::from_prost),
        )
    }

    /// Add a single file descriptor to the pool.
    ///
    /// All types referenced by the file must be defined either in the file itself, or in a file
    /// previously added to the pool.
    ///
    /// If the file is a duplicate of a files already in the pool, it will be skipped. Note this may cause issues when trying to add two different versions of a file with the same name.
    ///
    /// # Errors
    ///
    /// Returns an error if the given file descriptor is invalid, for example if it references types not yet added
    /// to the pool.
    pub fn add_file_descriptor_proto(
        &mut self,
        file: FileDescriptorProto,
    ) -> Result<(), DescriptorError> {
        self.add_file_descriptor_protos(iter::once(file))
    }

    /// Decode and add a single file descriptor to the pool.
    ///
    /// All types referenced by the file must be defined either in the file itself, or in a file
    /// previously added to the pool.
    ///
    /// Unlike when using [`add_file_descriptor_proto()`][DescriptorPool::add_file_descriptor_proto], any extension options
    /// defined in the file descriptor are preserved.
    ///
    /// If the file is a duplicate of a files already in the pool, it will be skipped. Note this may cause issues when trying to add two different versions of a file with the same name.
    ///
    /// # Errors
    ///
    /// Returns an error if the given bytes are not a valid protobuf-encoded file descriptor, or if the file descriptor itself
    /// is invalid, for example if it references types not yet added to the pool.
    pub fn decode_file_descriptor_proto<B>(&mut self, bytes: B) -> Result<(), DescriptorError>
    where
        B: Buf,
    {
        let file = types::FileDescriptorProto::decode(bytes).map_err(|err| {
            DescriptorError::new(vec![DescriptorErrorKind::DecodeFileDescriptorSet { err }])
        })?;

        self.build_files(iter::once(file))
    }

    /// Decode and add a set of file descriptors to the pool.
    ///
    /// A file descriptor set may be generated by running the protobuf compiler with the
    /// `--descriptor_set_out` flag. If you are using [`prost-build`](https://crates.io/crates/prost-build),
    /// then [`Config::file_descriptor_set_path`](https://docs.rs/prost-build/latest/prost_build/struct.Config.html#method..file_descriptor_set_path)
    /// is a convenient way to generate it as part of your build.
    ///
    /// Unlike when using [`add_file_descriptor_set()`][DescriptorPool::add_file_descriptor_set], any extension options
    /// defined in the file descriptors are preserved.
    ///
    /// Any duplicates of files already in the pool will be skipped. Note this may cause issues when trying to add two different versions of a file with the same name.
    ///
    /// # Errors
    ///
    /// Returns an error if the given bytes are not a valid protobuf-encoded file descriptor set, or if the descriptor set itself
    /// is invalid. When using a file descriptor set generated by the protobuf compiler, this method will always succeed.
    pub fn decode_file_descriptor_set<B>(&mut self, bytes: B) -> Result<(), DescriptorError>
    where
        B: Buf,
    {
        let file = types::FileDescriptorSet::decode(bytes).map_err(|err| {
            DescriptorError::new(vec![DescriptorErrorKind::DecodeFileDescriptorSet { err }])
        })?;

        self.build_files(file.file)
    }

    /// Gets an iterator over the file descriptors added to this pool.
    pub fn files(&self) -> impl ExactSizeIterator<Item = FileDescriptor> + '_ {
        indices(&self.inner.files).map(|index| FileDescriptor {
            pool: self.clone(),
            index,
        })
    }

    /// Gets a file descriptor by its name, or `None` if no such file has been added.
    pub fn get_file_by_name(&self, name: &str) -> Option<FileDescriptor> {
        if let Some(&index) = self.inner.file_names.get(name) {
            Some(FileDescriptor {
                pool: self.clone(),
                index,
            })
        } else {
            None
        }
    }

    /// Gets a iterator over the raw [`FileDescriptorProto`] instances wrapped by this [`DescriptorPool`].
    pub fn file_descriptor_protos(
        &self,
    ) -> impl ExactSizeIterator<Item = &FileDescriptorProto> + '_ {
        indices(&self.inner.files).map(|index| &self.inner.files[index as usize].prost)
    }

    /// Encodes the files contained within this [`DescriptorPool`] to their byte representation.
    ///
    /// The encoded message is equivalent to a [`FileDescriptorSet`], however also includes
    /// any extension options that were defined.
    pub fn encode<B>(&self, buf: B) -> Result<(), EncodeError>
    where
        B: BufMut,
    {
        use prost::encoding::{encoded_len_varint, DecodeContext};

        struct FileDescriptorSet<'a> {
            files: &'a [FileDescriptorInner],
        }

        impl<'a> fmt::Debug for FileDescriptorSet<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("FileDescriptorSet").finish_non_exhaustive()
            }
        }

        impl<'a> Message for FileDescriptorSet<'a> {
            fn encode_raw<B>(&self, buf: &mut B)
            where
                B: BufMut,
                Self: Sized,
            {
                for file in self.files {
                    encoding::message::encode(
                        tag::file_descriptor_set::FILE as u32,
                        &file.raw,
                        buf,
                    );
                }
            }

            fn encoded_len(&self) -> usize {
                encoding::key_len(tag::file_descriptor_set::FILE as u32) * self.files.len()
                    + self
                        .files
                        .iter()
                        .map(|f| &f.raw)
                        .map(Message::encoded_len)
                        .map(|len| len + encoded_len_varint(len as u64))
                        .sum::<usize>()
            }

            fn merge_field<B>(
                &mut self,
                _: u32,
                _: WireType,
                _: &mut B,
                _: DecodeContext,
            ) -> Result<(), DecodeError>
            where
                B: Buf,
                Self: Sized,
            {
                unimplemented!()
            }

            fn clear(&mut self) {
                unimplemented!()
            }
        }

        let mut buf = buf;
        FileDescriptorSet {
            files: &self.inner.files,
        }
        .encode(&mut buf)
    }

    /// Encodes the files contained within this [`DescriptorPool`] to a newly allocated buffer.
    ///
    /// The encoded message is equivalent to a [`FileDescriptorSet`], however also includes
    /// any extension options that were defined.
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode(&mut buf).expect("vec should have capacity");
        buf
    }

    /// Gets an iterator over the services defined in these protobuf files.
    pub fn services(&self) -> impl ExactSizeIterator<Item = ServiceDescriptor> + '_ {
        indices(&self.inner.services).map(|index| ServiceDescriptor {
            pool: self.clone(),
            index,
        })
    }

    /// Gets an iterator over all message types defined in these protobuf files.
    ///
    /// The iterator includes nested messages defined in another message.
    pub fn all_messages(&self) -> impl ExactSizeIterator<Item = MessageDescriptor> + '_ {
        indices(&self.inner.messages).map(|index| MessageDescriptor {
            pool: self.clone(),
            index,
        })
    }

    /// Gets an iterator over all enum types defined in these protobuf files.
    ///
    /// The iterator includes nested enums defined in another message.
    pub fn all_enums(&self) -> impl ExactSizeIterator<Item = EnumDescriptor> + '_ {
        indices(&self.inner.enums).map(|index| EnumDescriptor {
            pool: self.clone(),
            index,
        })
    }

    /// Gets an iterator over all extension fields defined in these protobuf files.
    ///
    /// The iterator includes nested extension fields defined in another message.
    pub fn all_extensions(&self) -> impl ExactSizeIterator<Item = ExtensionDescriptor> + '_ {
        indices(&self.inner.extensions).map(|index| ExtensionDescriptor {
            pool: self.clone(),
            index,
        })
    }

    /// Gets a [`MessageDescriptor`] by its fully qualified name, for example `my.package.MessageName`.
    pub fn get_message_by_name(&self, name: &str) -> Option<MessageDescriptor> {
        match self.inner.get_by_name(name) {
            Some(&Definition {
                kind: DefinitionKind::Message(index),
                ..
            }) => Some(MessageDescriptor {
                pool: self.clone(),
                index,
            }),
            _ => None,
        }
    }

    /// Gets an [`EnumDescriptor`] by its fully qualified name, for example `my.package.EnumName`.
    pub fn get_enum_by_name(&self, name: &str) -> Option<EnumDescriptor> {
        match self.inner.get_by_name(name) {
            Some(&Definition {
                kind: DefinitionKind::Enum(index),
                ..
            }) => Some(EnumDescriptor {
                pool: self.clone(),
                index,
            }),
            _ => None,
        }
    }

    /// Gets an [`ExtensionDescriptor`] by its fully qualified name, for example `my.package.my_extension`.
    pub fn get_extension_by_name(&self, name: &str) -> Option<ExtensionDescriptor> {
        match self.inner.get_by_name(name) {
            Some(&Definition {
                kind: DefinitionKind::Extension(index),
                ..
            }) => Some(ExtensionDescriptor {
                pool: self.clone(),
                index,
            }),
            _ => None,
        }
    }

    /// Gets an [`ServiceDescriptor`] by its fully qualified name, for example `my.package.MyService`.
    pub fn get_service_by_name(&self, name: &str) -> Option<ServiceDescriptor> {
        match self.inner.get_by_name(name) {
            Some(&Definition {
                kind: DefinitionKind::Service(index),
                ..
            }) => Some(ServiceDescriptor {
                pool: self.clone(),
                index,
            }),
            _ => None,
        }
    }
}

impl fmt::Debug for DescriptorPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorPool")
            .field("files", &debug_fmt_iter(self.files()))
            .field("services", &debug_fmt_iter(self.services()))
            .field("all_messages", &debug_fmt_iter(self.all_messages()))
            .field("all_enums", &debug_fmt_iter(self.all_enums()))
            .field("all_extensions", &debug_fmt_iter(self.all_extensions()))
            .finish()
    }
}

impl PartialEq for DescriptorPool {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for DescriptorPool {}

impl FileDescriptor {
    /// Create a new [`FileDescriptor`] referencing the file at `index` within the given [`DescriptorPool`].
    ///
    /// # Panics
    ///
    /// Panics if `index` is out-of-bounds.
    pub fn new(descriptor_pool: DescriptorPool, index: usize) -> Self {
        debug_assert!(index < descriptor_pool.files().len());
        FileDescriptor {
            pool: descriptor_pool,
            index: to_index(index),
        }
    }

    /// Gets a reference to the [`DescriptorPool`] this file is included in.
    pub fn parent_pool(&self) -> &DescriptorPool {
        &self.pool
    }

    /// Gets the unique name of this file relative to the root of the source tree,
    /// e.g. `path/to/my_package.proto`.
    pub fn name(&self) -> &str {
        self.inner().prost.name()
    }

    /// Gets the name of the package specifier for a file, e.g. `my.package`.
    ///
    /// If no package name is set, an empty string is returned.
    pub fn package_name(&self) -> &str {
        self.inner().prost.package()
    }

    /// Gets the index of this file within the parent [`DescriptorPool`].
    pub fn index(&self) -> usize {
        self.index as usize
    }

    /// Gets the syntax of this protobuf file.
    pub fn syntax(&self) -> Syntax {
        self.inner().syntax
    }

    /// Gets the dependencies of this file.
    ///
    /// This corresponds to the [`FileDescriptorProto::dependency`] field.
    pub fn dependencies(&self) -> impl ExactSizeIterator<Item = FileDescriptor> + '_ {
        let pool = self.parent_pool();
        self.file_descriptor_proto()
            .dependency
            .iter()
            .map(|name| pool.get_file_by_name(name).expect("file not found"))
    }

    /// Gets the public dependencies of this file.
    ///
    /// This corresponds to the [`FileDescriptorProto::public_dependency`] field.
    pub fn public_dependencies(&self) -> impl ExactSizeIterator<Item = FileDescriptor> + '_ {
        let pool = self.parent_pool();
        let raw = self.file_descriptor_proto();
        raw.public_dependency.iter().map(|&index| {
            pool.get_file_by_name(&raw.dependency[index as usize])
                .expect("file not found")
        })
    }

    /// Gets the top-level message types defined within this file.
    ///
    /// This does not include nested messages defined within another message.
    pub fn messages(&self) -> impl ExactSizeIterator<Item = MessageDescriptor> + '_ {
        let pool = self.parent_pool();
        let raw_file = self.file_descriptor_proto();
        raw_file.message_type.iter().map(move |raw_message| {
            pool.get_message_by_name(join_name(raw_file.package(), raw_message.name()).as_ref())
                .expect("message not found")
        })
    }

    /// Gets the top-level enum types defined within this file.
    ///
    /// This does not include nested enums defined within another message.
    pub fn enums(&self) -> impl ExactSizeIterator<Item = EnumDescriptor> + '_ {
        let pool = self.parent_pool();
        let raw_file = self.file_descriptor_proto();
        raw_file.enum_type.iter().map(move |raw_enum| {
            pool.get_enum_by_name(join_name(raw_file.package(), raw_enum.name()).as_ref())
                .expect("enum not found")
        })
    }

    /// Gets the top-level extension fields defined within this file.
    ///
    /// This does not include nested extensions defined within another message.
    pub fn extensions(&self) -> impl ExactSizeIterator<Item = ExtensionDescriptor> + '_ {
        let pool = self.parent_pool();
        let raw_file = self.file_descriptor_proto();
        raw_file.extension.iter().map(move |raw_extension| {
            pool.get_extension_by_name(join_name(raw_file.package(), raw_extension.name()).as_ref())
                .expect("extension not found")
        })
    }

    /// Gets the services defined within this file.
    pub fn services(&self) -> impl ExactSizeIterator<Item = ServiceDescriptor> + '_ {
        let pool = self.parent_pool();
        let raw_file = self.file_descriptor_proto();
        raw_file.service.iter().map(move |raw_service| {
            pool.get_service_by_name(join_name(raw_file.package(), raw_service.name()).as_ref())
                .expect("service not found")
        })
    }

    /// Gets a reference to the raw [`FileDescriptorProto`] wrapped by this [`FileDescriptor`].
    pub fn file_descriptor_proto(&self) -> &FileDescriptorProto {
        &self.inner().prost
    }

    /// Encodes this file descriptor to its byte representation.
    ///
    /// The encoded message is equivalent to a [`FileDescriptorProto`], however also includes
    /// any extension options that were defined.
    pub fn encode<B>(&self, buf: B) -> Result<(), EncodeError>
    where
        B: BufMut,
    {
        let mut buf = buf;
        self.inner().raw.encode(&mut buf)
    }

    /// Encodes this file descriptor to a newly allocated buffer.
    ///
    /// The encoded message is equivalent to a [`FileDescriptorProto`], however also includes
    /// any extension options that were defined.
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode(&mut buf).expect("vec should have capacity");
        buf
    }

    /// Decodes the options defined for this [`FileDescriptor`], including any extension options.
    pub fn options(&self) -> DynamicMessage {
        decode_options(
            self.parent_pool(),
            "google.protobuf.FileOptions",
            &self.inner().raw.options,
        )
    }

    fn inner(&self) -> &FileDescriptorInner {
        &self.pool.inner.files[self.index as usize]
    }
}

impl fmt::Debug for FileDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileDescriptor")
            .field("name", &self.name())
            .field("package_name", &self.package_name())
            .finish()
    }
}

impl MessageDescriptor {
    /// Gets a reference to the [`DescriptorPool`] this message is defined in.
    pub fn parent_pool(&self) -> &DescriptorPool {
        &self.pool
    }

    /// Gets the [`FileDescriptor`] this message is defined in.
    pub fn parent_file(&self) -> FileDescriptor {
        FileDescriptor {
            pool: self.pool.clone(),
            index: self.inner().id.file,
        }
    }

    /// Gets the parent message type if this message type is nested inside a another message, or `None` otherwise
    pub fn parent_message(&self) -> Option<MessageDescriptor> {
        self.inner().parent.map(|index| MessageDescriptor {
            pool: self.pool.clone(),
            index,
        })
    }

    /// Gets the short name of the message type, e.g. `MyMessage`.
    pub fn name(&self) -> &str {
        self.inner().id.name()
    }

    /// Gets the full name of the message type, e.g. `my.package.MyMessage`.
    pub fn full_name(&self) -> &str {
        self.inner().id.full_name()
    }

    /// Gets the name of the package this message type is defined in, e.g. `my.package`.
    ///
    /// If no package name is set, an empty string is returned.
    pub fn package_name(&self) -> &str {
        self.raw_file().package()
    }

    /// Gets the path where this message is defined within the [`FileDescriptorProto`][FileDescriptorProto], e.g. `[4, 0]`.
    ///
    /// See [`path`][prost_types::source_code_info::Location::path] for more details on the structure of the path.
    pub fn path(&self) -> &[i32] {
        &self.inner().id.path
    }

    /// Gets a reference to the [`FileDescriptorProto`] in which this message is defined.
    pub fn parent_file_descriptor_proto(&self) -> &FileDescriptorProto {
        &self.pool.inner.files[self.inner().id.file as usize].prost
    }

    /// Gets a reference to the raw [`DescriptorProto`] wrapped by this [`MessageDescriptor`].
    pub fn descriptor_proto(&self) -> &DescriptorProto {
        find_message_proto_prost(self.parent_file_descriptor_proto(), self.path())
    }

    /// Decodes the options defined for this [`MessageDescriptor`], including any extension options.
    pub fn options(&self) -> DynamicMessage {
        decode_options(
            self.parent_pool(),
            "google.protobuf.MessageOptions",
            &self.raw().options,
        )
    }

    /// Gets an iterator yielding a [`FieldDescriptor`] for each field defined in this message.
    pub fn fields(&self) -> impl ExactSizeIterator<Item = FieldDescriptor> + '_ {
        self.inner()
            .field_numbers
            .values()
            .map(|&index| FieldDescriptor {
                message: self.clone(),
                index,
            })
    }

    /// Gets an iterator yielding a [`OneofDescriptor`] for each oneof field defined in this message.
    pub fn oneofs(&self) -> impl ExactSizeIterator<Item = OneofDescriptor> + '_ {
        indices(&self.inner().oneofs).map(|index| OneofDescriptor {
            message: self.clone(),
            index,
        })
    }

    /// Gets the nested message types defined within this message.
    pub fn child_messages(&self) -> impl ExactSizeIterator<Item = MessageDescriptor> + '_ {
        let pool = self.parent_pool();
        let namespace = self.full_name();
        let raw_message = self.descriptor_proto();
        raw_message.nested_type.iter().map(move |raw_message| {
            pool.get_message_by_name(join_name(namespace, raw_message.name()).as_ref())
                .expect("message not found")
        })
    }

    /// Gets the nested enum types defined within this message.
    pub fn child_enums(&self) -> impl ExactSizeIterator<Item = EnumDescriptor> + '_ {
        let pool = self.parent_pool();
        let namespace = self.full_name();
        let raw_message = self.descriptor_proto();
        raw_message.enum_type.iter().map(move |raw_enum| {
            pool.get_enum_by_name(join_name(namespace, raw_enum.name()).as_ref())
                .expect("enum not found")
        })
    }

    /// Gets the nested extension fields defined within this message.
    ///
    /// Note this only returns extensions defined nested within this message. See
    /// [`MessageDescriptor::extensions`] to get fields defined anywhere that extend this message.
    pub fn child_extensions(&self) -> impl ExactSizeIterator<Item = ExtensionDescriptor> + '_ {
        let pool = self.parent_pool();
        let namespace = self.full_name();
        let raw_message = self.descriptor_proto();
        raw_message.extension.iter().map(move |raw_extension| {
            pool.get_extension_by_name(join_name(namespace, raw_extension.name()).as_ref())
                .expect("extension not found")
        })
    }

    /// Gets an iterator over all extensions to this message defined in the parent [`DescriptorPool`].
    ///
    /// Note this iterates over extension fields defined anywhere which extend this message. See
    /// [`MessageDescriptor::child_extensions`] to just get extensions defined nested within this message.
    pub fn extensions(&self) -> impl ExactSizeIterator<Item = ExtensionDescriptor> + '_ {
        self.inner()
            .extensions
            .iter()
            .map(|&index| ExtensionDescriptor {
                pool: self.parent_pool().clone(),
                index,
            })
    }

    /// Gets a [`FieldDescriptor`] with the given number, or `None` if no such field exists.
    pub fn get_field(&self, number: u32) -> Option<FieldDescriptor> {
        self.inner()
            .field_numbers
            .get(&number)
            .map(|&index| FieldDescriptor {
                message: self.clone(),
                index,
            })
    }

    /// Gets a [`FieldDescriptor`] with the given name, or `None` if no such field exists.
    pub fn get_field_by_name(&self, name: &str) -> Option<FieldDescriptor> {
        self.inner()
            .field_names
            .get(name)
            .map(|&index| FieldDescriptor {
                message: self.clone(),
                index,
            })
    }

    /// Gets a [`FieldDescriptor`] with the given JSON name, or `None` if no such field exists.
    pub fn get_field_by_json_name(&self, json_name: &str) -> Option<FieldDescriptor> {
        self.inner()
            .field_json_names
            .get(json_name)
            .map(|&index| FieldDescriptor {
                message: self.clone(),
                index,
            })
    }

    /// Returns `true` if this is an auto-generated message type to
    /// represent the entry type for a map field.
    //
    /// If this method returns `true`, [`fields`][Self::fields] is guaranteed to
    /// yield the following two fields:
    ///
    /// * A "key" field with a field number of 1
    /// * A "value" field with a field number of 2
    ///
    /// See [`map_entry_key_field`][MessageDescriptor::map_entry_key_field] and
    /// [`map_entry_value_field`][MessageDescriptor::map_entry_value_field] for more a convenient way
    /// to get these fields.
    pub fn is_map_entry(&self) -> bool {
        self.raw()
            .options
            .as_ref()
            .map(|o| o.value.map_entry())
            .unwrap_or(false)
    }

    /// If this is a [map entry](MessageDescriptor::is_map_entry), returns a [`FieldDescriptor`] for the key.
    ///
    /// # Panics
    ///
    /// This method may panic if [`is_map_entry`][MessageDescriptor::is_map_entry] returns `false`.
    pub fn map_entry_key_field(&self) -> FieldDescriptor {
        debug_assert!(self.is_map_entry());
        self.get_field(MAP_ENTRY_KEY_NUMBER)
            .expect("map entry should have key field")
    }

    /// If this is a [map entry](MessageDescriptor::is_map_entry), returns a [`FieldDescriptor`] for the value.
    ///
    /// # Panics
    ///
    /// This method may panic if [`is_map_entry`][MessageDescriptor::is_map_entry] returns `false`.
    pub fn map_entry_value_field(&self) -> FieldDescriptor {
        debug_assert!(self.is_map_entry());
        self.get_field(MAP_ENTRY_VALUE_NUMBER)
            .expect("map entry should have value field")
    }

    /// Gets an iterator over reserved field number ranges in this message.
    pub fn reserved_ranges(&self) -> impl ExactSizeIterator<Item = Range<u32>> + '_ {
        self.raw()
            .reserved_range
            .iter()
            .map(|n| (n.start() as u32)..(n.end() as u32))
    }

    /// Gets an iterator over reserved field names in this message.
    pub fn reserved_names(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.raw().reserved_name.iter().map(|n| n.as_ref())
    }

    /// Gets an iterator over extension field number ranges in this message.
    pub fn extension_ranges(&self) -> impl ExactSizeIterator<Item = Range<u32>> + '_ {
        self.raw()
            .extension_range
            .iter()
            .map(|n| (n.start() as u32)..(n.end() as u32))
    }

    /// Gets an extension to this message by its number, or `None` if no such extension exists.
    pub fn get_extension(&self, number: u32) -> Option<ExtensionDescriptor> {
        self.extensions().find(|ext| ext.number() == number)
    }

    /// Gets an extension to this message by its full name (e.g. `my.package.my_extension`), or `None` if no such extension exists.
    pub fn get_extension_by_full_name(&self, name: &str) -> Option<ExtensionDescriptor> {
        self.extensions().find(|ext| ext.full_name() == name)
    }

    /// Gets an extension to this message by its JSON name (e.g. `[my.package.my_extension]`), or `None` if no such extension exists.
    pub fn get_extension_by_json_name(&self, name: &str) -> Option<ExtensionDescriptor> {
        self.extensions().find(|ext| ext.json_name() == name)
    }

    fn inner(&self) -> &MessageDescriptorInner {
        &self.pool.inner.messages[self.index as usize]
    }

    fn raw(&self) -> &types::DescriptorProto {
        find_message_proto(self.raw_file(), self.path())
    }

    fn raw_file(&self) -> &types::FileDescriptorProto {
        &self.pool.inner.files[self.inner().id.file as usize].raw
    }
}

impl fmt::Debug for MessageDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessageDescriptor")
            .field("name", &self.name())
            .field("full_name", &self.full_name())
            .field("is_map_entry", &self.is_map_entry())
            .field("fields", &debug_fmt_iter(self.fields()))
            .field("oneofs", &debug_fmt_iter(self.oneofs()))
            .finish()
    }
}

impl FieldDescriptor {
    /// Gets a reference to the [`DescriptorPool`] this field is defined in.
    pub fn parent_pool(&self) -> &DescriptorPool {
        self.message.parent_pool()
    }

    /// Gets the [`FileDescriptor`] this field is defined in.
    pub fn parent_file(&self) -> FileDescriptor {
        self.message.parent_file()
    }

    /// Gets a reference to the [`MessageDescriptor`] this field is defined in.
    pub fn parent_message(&self) -> &MessageDescriptor {
        &self.message
    }

    /// Gets the short name of the message type, e.g. `my_field`.
    pub fn name(&self) -> &str {
        self.inner().id.name()
    }

    /// Gets the full name of the message field, e.g. `my.package.MyMessage.my_field`.
    pub fn full_name(&self) -> &str {
        self.inner().id.full_name()
    }

    /// Gets the path where this message field is defined within the [`FileDescriptorProto`][FileDescriptorProto], e.g. `[4, 0, 2, 0]`.
    ///
    /// See [`path`][prost_types::source_code_info::Location::path] for more details on the structure of the path.
    pub fn path(&self) -> &[i32] {
        &self.inner().id.path
    }

    /// Gets a reference to the raw [`FieldDescriptorProto`] wrapped by this [`FieldDescriptor`].
    pub fn field_descriptor_proto(&self) -> &FieldDescriptorProto {
        &self.parent_message().descriptor_proto().field[*self.path().last().unwrap() as usize]
    }

    /// Decodes the options defined for this [`FieldDescriptor`], including any extension options.
    pub fn options(&self) -> DynamicMessage {
        decode_options(
            self.parent_pool(),
            "google.protobuf.FieldOptions",
            &self.raw().options,
        )
    }

    /// Gets the unique number for this message field.
    pub fn number(&self) -> u32 {
        self.inner().number
    }

    /// Gets the name used for JSON serialization.
    ///
    /// This is usually the camel-cased form of the field name, unless
    /// another value is set in the proto file.
    pub fn json_name(&self) -> &str {
        &self.inner().json_name
    }

    /// Whether this field is encoded using the proto2 group encoding.
    pub fn is_group(&self) -> bool {
        matches!(self.inner().kind, KindIndex::Group(_))
    }

    /// Whether this field is a list type.
    ///
    /// Equivalent to checking that the cardinality is `Repeated` and that
    /// [`is_map`][Self::is_map] returns `false`.
    pub fn is_list(&self) -> bool {
        self.cardinality() == Cardinality::Repeated && !self.is_map()
    }

    /// Whether this field is a map type.
    ///
    /// Equivalent to checking that the cardinality is `Repeated` and that
    /// the field type is a message where [`is_map_entry`][MessageDescriptor::is_map_entry]
    /// returns `true`.
    pub fn is_map(&self) -> bool {
        self.cardinality() == Cardinality::Repeated
            && match self.kind() {
                Kind::Message(message) => message.is_map_entry(),
                _ => false,
            }
    }

    /// Whether this field is a list encoded using [packed encoding](https://developers.google.com/protocol-buffers/docs/encoding#packed).
    pub fn is_packed(&self) -> bool {
        self.inner().is_packed
    }

    /// The cardinality of this field.
    pub fn cardinality(&self) -> Cardinality {
        self.inner().cardinality
    }

    /// Whether this field supports distinguishing between an unpopulated field and
    /// the default value.
    ///
    /// For proto2 messages this returns `true` for all non-repeated fields.
    /// For proto3 this returns `true` for message fields, and fields contained
    /// in a `oneof`.
    pub fn supports_presence(&self) -> bool {
        self.inner().supports_presence
    }

    /// Gets the [`Kind`] of this field.
    pub fn kind(&self) -> Kind {
        Kind::new(self.parent_pool(), self.inner().kind)
    }

    /// Gets a [`OneofDescriptor`] representing the oneof containing this field,
    /// or `None` if this field is not contained in a oneof.
    pub fn containing_oneof(&self) -> Option<OneofDescriptor> {
        self.inner().oneof.map(|index| OneofDescriptor {
            message: self.message.clone(),
            index,
        })
    }

    pub(crate) fn default_value(&self) -> Option<&Value> {
        self.inner().default.as_ref()
    }

    pub(crate) fn is_packable(&self) -> bool {
        self.inner().kind.is_packable()
    }

    fn inner(&self) -> &FieldDescriptorInner {
        &self.message.inner().fields[self.index as usize]
    }

    fn raw(&self) -> &types::FieldDescriptorProto {
        &self.message.raw().field[self.index as usize]
    }
}

impl fmt::Debug for FieldDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FieldDescriptor")
            .field("name", &self.name())
            .field("full_name", &self.full_name())
            .field("json_name", &self.json_name())
            .field("number", &self.number())
            .field("kind", &self.kind())
            .field("cardinality", &self.cardinality())
            .field(
                "containing_oneof",
                &self.containing_oneof().map(|o| o.name().to_owned()),
            )
            .field("default_value", &self.default_value())
            .field("is_group", &self.is_group())
            .field("is_list", &self.is_list())
            .field("is_map", &self.is_map())
            .field("is_packed", &self.is_packed())
            .field("supports_presence", &self.supports_presence())
            .finish()
    }
}

impl ExtensionDescriptor {
    /// Gets a reference to the [`DescriptorPool`] this extension field is defined in.
    pub fn parent_pool(&self) -> &DescriptorPool {
        &self.pool
    }

    /// Gets the [`FileDescriptor`] this extension field is defined in.
    pub fn parent_file(&self) -> FileDescriptor {
        FileDescriptor {
            pool: self.pool.clone(),
            index: self.inner().id.file,
        }
    }

    /// Gets the parent message type if this extension is defined within another message, or `None` otherwise.
    ///
    /// Note this just corresponds to where the extension is defined in the proto file. See [`containing_message`][ExtensionDescriptor::containing_message]
    /// for the message this field extends.
    pub fn parent_message(&self) -> Option<MessageDescriptor> {
        self.inner().parent.map(|index| MessageDescriptor {
            pool: self.pool.clone(),
            index,
        })
    }

    /// Gets the short name of the extension field type, e.g. `my_extension`.
    pub fn name(&self) -> &str {
        self.inner().id.name()
    }

    /// Gets the full name of the extension field, e.g. `my.package.ParentMessage.my_extension`.
    ///
    /// Note this includes the name of the parent message if any, not the message this field extends.
    pub fn full_name(&self) -> &str {
        self.inner().id.full_name()
    }

    /// Gets the name of the package this extension field is defined in, e.g. `my.package`.
    ///
    /// If no package name is set, an empty string is returned.
    pub fn package_name(&self) -> &str {
        self.raw_file().package()
    }

    /// Gets the path where this extension field is defined within the [`FileDescriptorProto`][FileDescriptorProto], e.g. `[7, 0]`.
    ///
    /// See [`path`][prost_types::source_code_info::Location::path] for more details on the structure of the path.
    pub fn path(&self) -> &[i32] {
        &self.inner().id.path
    }

    /// Gets a reference to the [`FileDescriptorProto`] in which this extension is defined.
    pub fn parent_file_descriptor_proto(&self) -> &FileDescriptorProto {
        &self.pool.inner.files[self.inner().id.file as usize].prost
    }

    /// Gets a reference to the raw [`FieldDescriptorProto`] wrapped by this [`ExtensionDescriptor`].
    pub fn field_descriptor_proto(&self) -> &FieldDescriptorProto {
        let file = self.parent_file_descriptor_proto();
        let path = self.path();
        debug_assert_ne!(path.len(), 0);
        debug_assert_eq!(path.len() % 2, 0);
        if path.len() == 2 {
            debug_assert_eq!(path[0], tag::file::EXTENSION);
            &file.extension[path[1] as usize]
        } else {
            let message = find_message_proto_prost(file, &path[..path.len() - 2]);
            debug_assert_eq!(path[path.len() - 2], tag::message::EXTENSION);
            &message.extension[path[path.len() - 1] as usize]
        }
    }

    /// Decodes the options defined for this [`ExtensionDescriptor`], including any extension options.
    pub fn options(&self) -> DynamicMessage {
        decode_options(
            self.parent_pool(),
            "google.protobuf.FieldOptions",
            &self.raw().options,
        )
    }

    /// Gets the number for this extension field.
    pub fn number(&self) -> u32 {
        self.inner().number
    }

    /// Gets the name used for JSON serialization of this extension field, e.g. `[my.package.ParentMessage.my_field]`.
    pub fn json_name(&self) -> &str {
        &self.inner().json_name
    }

    /// Whether this field is encoded using the proto2 group encoding.
    pub fn is_group(&self) -> bool {
        matches!(self.inner().kind, KindIndex::Group(_))
    }

    /// Whether this field is a list type.
    ///
    /// Equivalent to checking that the cardinality is `Repeated` and that
    /// [`is_map`][Self::is_map] returns `false`.
    pub fn is_list(&self) -> bool {
        self.cardinality() == Cardinality::Repeated && !self.is_map()
    }

    /// Whether this field is a map type.
    ///
    /// Equivalent to checking that the cardinality is `Repeated` and that
    /// the field type is a message where [`is_map_entry`][MessageDescriptor::is_map_entry]
    /// returns `true`.
    pub fn is_map(&self) -> bool {
        self.cardinality() == Cardinality::Repeated
            && match self.kind() {
                Kind::Message(message) => message.is_map_entry(),
                _ => false,
            }
    }

    /// Whether this field is a list encoded using [packed encoding](https://developers.google.com/protocol-buffers/docs/encoding#packed).
    pub fn is_packed(&self) -> bool {
        self.inner().is_packed
    }

    /// The cardinality of this field.
    pub fn cardinality(&self) -> Cardinality {
        self.inner().cardinality
    }

    /// Whether this extension supports distinguishing between an unpopulated field and
    /// the default value.
    ///
    /// This is equivalent to `cardinality() != Cardinality::Repeated`
    pub fn supports_presence(&self) -> bool {
        self.cardinality() != Cardinality::Repeated
    }

    /// Gets the [`Kind`] of this field.
    pub fn kind(&self) -> Kind {
        Kind::new(&self.pool, self.inner().kind)
    }

    /// Gets the containing message that this field extends.
    pub fn containing_message(&self) -> MessageDescriptor {
        MessageDescriptor {
            pool: self.pool.clone(),
            index: self.inner().extendee,
        }
    }

    pub(crate) fn default_value(&self) -> Option<&Value> {
        self.inner().default.as_ref()
    }

    pub(crate) fn is_packable(&self) -> bool {
        self.inner().kind.is_packable()
    }

    fn inner(&self) -> &ExtensionDescriptorInner {
        &self.pool.inner.extensions[self.index as usize]
    }

    fn raw(&self) -> &types::FieldDescriptorProto {
        let file = self.raw_file();
        let path = self.path();
        debug_assert_ne!(path.len(), 0);
        debug_assert_eq!(path.len() % 2, 0);
        if path.len() == 2 {
            debug_assert_eq!(path[0], tag::file::EXTENSION);
            &file.extension[path[1] as usize]
        } else {
            let message = find_message_proto(file, &path[..path.len() - 2]);
            debug_assert_eq!(path[path.len() - 2], tag::message::EXTENSION);
            &message.extension[path[path.len() - 1] as usize]
        }
    }

    fn raw_file(&self) -> &types::FileDescriptorProto {
        &self.pool.inner.files[self.inner().id.file as usize].raw
    }
}

impl fmt::Debug for ExtensionDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExtensionDescriptor")
            .field("name", &self.name())
            .field("full_name", &self.full_name())
            .field("json_name", &self.json_name())
            .field("number", &self.number())
            .field("kind", &self.kind())
            .field("cardinality", &self.cardinality())
            .field(
                "containing_message",
                &self.containing_message().name().to_owned(),
            )
            .field("default_value", &self.default_value())
            .field("is_group", &self.is_group())
            .field("is_list", &self.is_list())
            .field("is_map", &self.is_map())
            .field("is_packed", &self.is_packed())
            .field("supports_presence", &self.supports_presence())
            .finish()
    }
}

impl EnumDescriptor {
    /// Gets a reference to the [`DescriptorPool`] this enum type is defined in.
    pub fn parent_pool(&self) -> &DescriptorPool {
        &self.pool
    }

    /// Gets the [`FileDescriptor`] this enum type is defined in.
    pub fn parent_file(&self) -> FileDescriptor {
        FileDescriptor {
            pool: self.pool.clone(),
            index: self.inner().id.file,
        }
    }

    /// Gets the parent message type if this enum type is nested inside a another message, or `None` otherwise
    pub fn parent_message(&self) -> Option<MessageDescriptor> {
        self.inner().parent.map(|index| MessageDescriptor {
            pool: self.pool.clone(),
            index,
        })
    }

    /// Gets the short name of the enum type, e.g. `MyEnum`.
    pub fn name(&self) -> &str {
        self.inner().id.name()
    }

    /// Gets the full name of the enum, e.g. `my.package.MyEnum`.
    pub fn full_name(&self) -> &str {
        self.inner().id.full_name()
    }

    /// Gets the name of the package this enum type is defined in, e.g. `my.package`.
    ///
    /// If no package name is set, an empty string is returned.
    pub fn package_name(&self) -> &str {
        self.raw_file().package()
    }

    /// Gets the path where this enum type is defined within the [`FileDescriptorProto`][FileDescriptorProto], e.g. `[5, 0]`.
    ///
    /// See [`path`][prost_types::source_code_info::Location::path] for more details on the structure of the path.
    pub fn path(&self) -> &[i32] {
        &self.inner().id.path
    }

    /// Gets a reference to the [`FileDescriptorProto`] in which this enum is defined.
    pub fn parent_file_descriptor_proto(&self) -> &FileDescriptorProto {
        &self.pool.inner.files[self.inner().id.file as usize].prost
    }

    /// Gets a reference to the raw [`EnumDescriptorProto`] wrapped by this [`EnumDescriptor`].
    pub fn enum_descriptor_proto(&self) -> &EnumDescriptorProto {
        let file = self.parent_file_descriptor_proto();
        let path = self.path();
        debug_assert_ne!(path.len(), 0);
        debug_assert_eq!(path.len() % 2, 0);
        if path.len() == 2 {
            debug_assert_eq!(path[0], tag::file::ENUM_TYPE);
            &file.enum_type[path[1] as usize]
        } else {
            let message = find_message_proto_prost(file, &path[..path.len() - 2]);
            debug_assert_eq!(path[path.len() - 2], tag::message::ENUM_TYPE);
            &message.enum_type[path[path.len() - 1] as usize]
        }
    }

    /// Decodes the options defined for this [`EnumDescriptor`], including any extension options.
    pub fn options(&self) -> DynamicMessage {
        decode_options(
            self.parent_pool(),
            "google.protobuf.EnumOptions",
            &self.raw().options,
        )
    }

    /// Gets the default value for the enum type.
    pub fn default_value(&self) -> EnumValueDescriptor {
        EnumValueDescriptor {
            parent: self.clone(),
            index: 0,
        }
    }

    /// Gets a [`EnumValueDescriptor`] for the enum value with the given name, or `None` if no such value exists.
    pub fn get_value_by_name(&self, name: &str) -> Option<EnumValueDescriptor> {
        self.inner()
            .value_names
            .get(name)
            .map(|&index| EnumValueDescriptor {
                parent: self.clone(),
                index,
            })
    }

    /// Gets a [`EnumValueDescriptor`] for the enum value with the given number, or `None` if no such value exists.
    ///
    /// If the enum is defined with the `allow_alias` option and has multiple values with the given number, it is
    /// unspecified which one will be returned.
    pub fn get_value(&self, number: i32) -> Option<EnumValueDescriptor> {
        self.inner()
            .value_numbers
            .binary_search_by(|(l, _)| l.cmp(&number))
            .ok()
            .map(|index| EnumValueDescriptor {
                parent: self.clone(),
                index: self.inner().value_numbers[index].1,
            })
    }

    /// Gets an iterator yielding a [`EnumValueDescriptor`] for each value in this enum.
    pub fn values(&self) -> impl ExactSizeIterator<Item = EnumValueDescriptor> + '_ {
        self.inner()
            .value_numbers
            .iter()
            .map(|&(_, index)| EnumValueDescriptor {
                parent: self.clone(),
                index,
            })
    }

    /// Gets an iterator over reserved value number ranges in this enum.
    pub fn reserved_ranges(&self) -> impl ExactSizeIterator<Item = RangeInclusive<i32>> + '_ {
        self.raw()
            .reserved_range
            .iter()
            .map(|n| n.start()..=n.end())
    }

    /// Gets an iterator over reserved value names in this enum.
    pub fn reserved_names(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.raw().reserved_name.iter().map(|n| n.as_ref())
    }

    fn inner(&self) -> &EnumDescriptorInner {
        &self.pool.inner.enums[self.index as usize]
    }

    fn raw(&self) -> &types::EnumDescriptorProto {
        find_enum_proto(self.raw_file(), self.path())
    }

    fn raw_file(&self) -> &types::FileDescriptorProto {
        &self.pool.inner.files[self.inner().id.file as usize].raw
    }
}

impl fmt::Debug for EnumDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnumDescriptor")
            .field("name", &self.name())
            .field("full_name", &self.full_name())
            .field("default_value", &self.default_value())
            .field("values", &debug_fmt_iter(self.values()))
            .finish()
    }
}

impl EnumValueDescriptor {
    /// Gets a reference to the [`DescriptorPool`] this enum value is defined in.
    pub fn parent_pool(&self) -> &DescriptorPool {
        self.parent.parent_pool()
    }

    /// Gets the [`FileDescriptor`] this enum value is defined in.
    pub fn parent_file(&self) -> FileDescriptor {
        self.parent.parent_file()
    }

    /// Gets a reference to the [`EnumDescriptor`] this enum value is defined in.
    pub fn parent_enum(&self) -> &EnumDescriptor {
        &self.parent
    }

    /// Gets the short name of the enum value, e.g. `MY_VALUE`.
    pub fn name(&self) -> &str {
        self.inner().id.name()
    }

    /// Gets the full name of the enum value, e.g. `my.package.MY_VALUE`.
    pub fn full_name(&self) -> &str {
        self.inner().id.full_name()
    }

    /// Gets the path where this enum value is defined within the [`FileDescriptorProto`][FileDescriptorProto], e.g. `[5, 0, 2, 0]`.
    ///
    /// See [`path`][prost_types::source_code_info::Location::path] for more details on the structure of the path.
    pub fn path(&self) -> &[i32] {
        &self.inner().id.path
    }

    /// Gets a reference to the raw [`EnumValueDescriptorProto`] wrapped by this [`EnumValueDescriptor`].
    pub fn enum_value_descriptor_proto(&self) -> &EnumValueDescriptorProto {
        &self.parent.enum_descriptor_proto().value[self.index as usize]
    }

    /// Decodes the options defined for this [`EnumValueDescriptor`], including any extension options.
    pub fn options(&self) -> DynamicMessage {
        decode_options(
            self.parent_pool(),
            "google.protobuf.EnumValueOptions",
            &self.raw().options,
        )
    }

    /// Gets the number representing this enum value.
    pub fn number(&self) -> i32 {
        self.inner().number
    }

    fn inner(&self) -> &EnumValueDescriptorInner {
        &self.parent.inner().values[self.index as usize]
    }

    fn raw(&self) -> &types::EnumValueDescriptorProto {
        &self.parent.raw().value[self.index as usize]
    }
}

impl fmt::Debug for EnumValueDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnumValueDescriptor")
            .field("name", &self.number())
            .field("full_name", &self.full_name())
            .field("number", &self.number())
            .finish()
    }
}

impl OneofDescriptor {
    /// Gets a reference to the [`DescriptorPool`] this oneof is defined in.
    pub fn parent_pool(&self) -> &DescriptorPool {
        self.message.parent_pool()
    }

    /// Gets the [`FileDescriptor`] this oneof is defined in.
    pub fn parent_file(&self) -> FileDescriptor {
        self.message.parent_file()
    }

    /// Gets a reference to the [`MessageDescriptor`] this oneof is defined in.
    pub fn parent_message(&self) -> &MessageDescriptor {
        &self.message
    }

    /// Gets the short name of the oneof, e.g. `my_oneof`.
    pub fn name(&self) -> &str {
        self.inner().id.name()
    }

    /// Gets the full name of the oneof, e.g. `my.package.MyMessage.my_oneof`.
    pub fn full_name(&self) -> &str {
        self.inner().id.full_name()
    }

    /// Gets the path where this oneof is defined within the [`FileDescriptorProto`][FileDescriptorProto], e.g. `[4, 0, 8, 0]`.
    ///
    /// See [`path`][prost_types::source_code_info::Location::path] for more details on the structure of the path.
    pub fn path(&self) -> &[i32] {
        &self.inner().id.path
    }

    /// Gets a reference to the raw [`OneofDescriptorProto`] wrapped by this [`OneofDescriptor`].
    pub fn oneof_descriptor_proto(&self) -> &OneofDescriptorProto {
        &self.message.descriptor_proto().oneof_decl[self.index as usize]
    }

    /// Decodes the options defined for this [`OneofDescriptorProto`], including any extension options.
    pub fn options(&self) -> DynamicMessage {
        decode_options(
            self.parent_pool(),
            "google.protobuf.OneofOptions",
            &self.raw().options,
        )
    }

    /// Gets an iterator yielding a [`FieldDescriptor`] for each field of the parent message this oneof contains.
    pub fn fields(&self) -> impl ExactSizeIterator<Item = FieldDescriptor> + '_ {
        self.inner().fields.iter().map(|&index| FieldDescriptor {
            message: self.parent_message().clone(),
            index,
        })
    }

    fn inner(&self) -> &OneofDescriptorInner {
        &self.message.inner().oneofs[self.index as usize]
    }

    fn raw(&self) -> &types::OneofDescriptorProto {
        &self.message.raw().oneof_decl[self.index as usize]
    }
}

impl fmt::Debug for OneofDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OneofDescriptor")
            .field("name", &self.name())
            .field("full_name", &self.full_name())
            .field("fields", &debug_fmt_iter(self.fields()))
            .finish()
    }
}

impl ServiceDescriptor {
    /// Create a new [`ServiceDescriptor`] referencing the service at `index` within the given [`DescriptorPool`].
    ///
    /// # Panics
    ///
    /// Panics if `index` is out-of-bounds.
    pub fn new(pool: DescriptorPool, index: usize) -> Self {
        debug_assert!(index < pool.services().len());
        ServiceDescriptor {
            pool,
            index: to_index(index),
        }
    }

    /// Returns the index of this [`ServiceDescriptor`] within the parent [`DescriptorPool`].
    pub fn index(&self) -> usize {
        self.index as usize
    }

    /// Gets a reference to the [`DescriptorPool`] this service is defined in.
    pub fn parent_pool(&self) -> &DescriptorPool {
        &self.pool
    }

    /// Gets the [`FileDescriptor`] this service is defined in.
    pub fn parent_file(&self) -> FileDescriptor {
        FileDescriptor {
            pool: self.pool.clone(),
            index: self.inner().id.file,
        }
    }

    /// Gets the short name of the service, e.g. `MyService`.
    pub fn name(&self) -> &str {
        self.inner().id.name()
    }

    /// Gets the full name of the service, e.g. `my.package.Service`.
    pub fn full_name(&self) -> &str {
        self.inner().id.full_name()
    }

    /// Gets the name of the package this service is defined in, e.g. `my.package`.
    ///
    /// If no package name is set, an empty string is returned.
    pub fn package_name(&self) -> &str {
        self.raw_file().package()
    }

    /// Gets the path where this service is defined within the [`FileDescriptorProto`][FileDescriptorProto], e.g. `[6, 0]`.
    ///
    /// See [`path`][prost_types::source_code_info::Location::path] for more details on the structure of the path.
    pub fn path(&self) -> &[i32] {
        &self.inner().id.path
    }

    /// Gets a reference to the [`FileDescriptorProto`] in which this service is defined.
    pub fn parent_file_descriptor_proto(&self) -> &FileDescriptorProto {
        &self.pool.inner.files[self.inner().id.file as usize].prost
    }

    /// Gets a reference to the raw [`ServiceDescriptorProto`] wrapped by this [`ServiceDescriptor`].
    pub fn service_descriptor_proto(&self) -> &ServiceDescriptorProto {
        let path = self.path();
        debug_assert!(!path.is_empty());
        &self.parent_file_descriptor_proto().service[*path.last().unwrap() as usize]
    }

    /// Decodes the options defined for this [`ServiceDescriptorProto`], including any extension options.
    pub fn options(&self) -> DynamicMessage {
        decode_options(
            self.parent_pool(),
            "google.protobuf.ServiceOptions",
            &self.raw().options,
        )
    }

    /// Gets an iterator yielding a [`MethodDescriptor`] for each method defined in this service.
    pub fn methods(&self) -> impl ExactSizeIterator<Item = MethodDescriptor> + '_ {
        indices(&self.inner().methods).map(|index| MethodDescriptor {
            service: self.clone(),
            index,
        })
    }

    fn inner(&self) -> &ServiceDescriptorInner {
        &self.pool.inner.services[self.index as usize]
    }

    fn raw(&self) -> &types::ServiceDescriptorProto {
        let path = self.path();
        debug_assert!(!path.is_empty());
        &self.raw_file().service[*path.last().unwrap() as usize]
    }

    fn raw_file(&self) -> &types::FileDescriptorProto {
        &self.pool.inner.files[self.inner().id.file as usize].raw
    }
}

impl fmt::Debug for ServiceDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceDescriptor")
            .field("name", &self.name())
            .field("full_name", &self.full_name())
            .field("index", &self.index())
            .field("methods", &debug_fmt_iter(self.methods()))
            .finish()
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
        MethodDescriptor {
            service,
            index: to_index(index),
        }
    }

    /// Gets the index of the method within the parent [`ServiceDescriptor`].
    pub fn index(&self) -> usize {
        self.index as usize
    }

    /// Gets a reference to the [`ServiceDescriptor`] this method is defined in.
    pub fn parent_service(&self) -> &ServiceDescriptor {
        &self.service
    }

    /// Gets a reference to the [`DescriptorPool`] this method is defined in.
    pub fn parent_pool(&self) -> &DescriptorPool {
        self.service.parent_pool()
    }

    /// Gets the [`FileDescriptor`] this method is defined in.
    pub fn parent_file(&self) -> FileDescriptor {
        self.service.parent_file()
    }

    /// Gets the short name of the method, e.g. `method`.
    pub fn name(&self) -> &str {
        self.inner().id.name()
    }

    /// Gets the full name of the method, e.g. `my.package.MyService.my_method`.
    pub fn full_name(&self) -> &str {
        self.inner().id.full_name()
    }

    /// Gets the path where this method is defined within the [`FileDescriptorProto`][FileDescriptorProto], e.g. `[6, 0, 2, 0]`.
    ///
    /// See [`path`][prost_types::source_code_info::Location::path] for more details on the structure of the path.
    pub fn path(&self) -> &[i32] {
        &self.inner().id.path
    }

    /// Gets a reference to the raw [`MethodDescriptorProto`] wrapped by this [`MethodDescriptor`].
    pub fn method_descriptor_proto(&self) -> &MethodDescriptorProto {
        &self.service.service_descriptor_proto().method[self.index as usize]
    }

    /// Decodes the options defined for this [`MethodDescriptorProto`], including any extension options.
    pub fn options(&self) -> DynamicMessage {
        decode_options(
            self.parent_pool(),
            "google.protobuf.MethodOptions",
            &self.raw().options,
        )
    }

    /// Gets the [`MessageDescriptor`] for the input type of this method.
    pub fn input(&self) -> MessageDescriptor {
        MessageDescriptor {
            pool: self.parent_pool().clone(),
            index: self.inner().input,
        }
    }

    /// Gets the [`MessageDescriptor`] for the output type of this method.
    pub fn output(&self) -> MessageDescriptor {
        MessageDescriptor {
            pool: self.parent_pool().clone(),
            index: self.inner().output,
        }
    }

    /// Returns `true` if the client streams multiple messages.
    pub fn is_client_streaming(&self) -> bool {
        self.raw().client_streaming()
    }

    /// Returns `true` if the server streams multiple messages.
    pub fn is_server_streaming(&self) -> bool {
        self.raw().server_streaming()
    }

    fn inner(&self) -> &MethodDescriptorInner {
        &self.service.inner().methods[self.index as usize]
    }

    fn raw(&self) -> &types::MethodDescriptorProto {
        &self.service.raw().method[self.index as usize]
    }
}

impl fmt::Debug for MethodDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MethodDescriptor")
            .field("name", &self.name())
            .field("full_name", &self.full_name())
            .field("index", &self.index())
            .field("input", &self.input())
            .field("output", &self.output())
            .field("is_client_streaming", &self.is_client_streaming())
            .field("is_server_streaming", &self.is_server_streaming())
            .finish()
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

fn indices<T>(f: &Vec<T>) -> Range<DescriptorIndex> {
    0..to_index(f.len())
}

fn join_name<'a>(namespace: &str, name: &'a str) -> Cow<'a, str> {
    if namespace.is_empty() {
        Cow::Borrowed(name)
    } else {
        Cow::Owned(format!("{}.{}", namespace, name))
    }
}

fn decode_options<T>(
    pool: &DescriptorPool,
    name: &str,
    option: &Option<Options<T>>,
) -> DynamicMessage {
    let message_desc = pool
        .get_message_by_name(name)
        .unwrap_or_else(|| DescriptorPool::global().get_message_by_name(name).unwrap());

    let bytes = option
        .as_ref()
        .map(|o| o.encoded.as_slice())
        .unwrap_or_default();
    DynamicMessage::decode(message_desc, bytes).unwrap()
}

fn find_message_proto_prost<'a>(
    file: &'a FileDescriptorProto,
    path: &[i32],
) -> &'a DescriptorProto {
    debug_assert_ne!(path.len(), 0);
    debug_assert_eq!(path.len() % 2, 0);

    let mut message: Option<&'a DescriptorProto> = None;
    for part in path.chunks(2) {
        match part[0] {
            tag::file::MESSAGE_TYPE => message = Some(&file.message_type[part[1] as usize]),
            tag::message::NESTED_TYPE => {
                message = Some(&message.unwrap().nested_type[part[1] as usize])
            }
            _ => panic!("invalid message path"),
        }
    }

    message.unwrap()
}
