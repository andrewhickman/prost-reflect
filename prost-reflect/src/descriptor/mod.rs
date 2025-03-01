mod api;
mod build;
mod error;
mod global;
mod tag;
#[cfg(test)]
mod tests;
pub(crate) mod types;

pub use self::error::DescriptorError;
use self::types::{DescriptorProto, EnumDescriptorProto};

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    convert::TryInto,
    fmt,
    ops::Range,
    sync::Arc,
};

use crate::{descriptor::types::FileDescriptorProto, Value};

pub(crate) const MAP_ENTRY_KEY_NUMBER: u32 = 1;
pub(crate) const MAP_ENTRY_VALUE_NUMBER: u32 = 2;

pub(crate) const RESERVED_MESSAGE_FIELD_NUMBERS: Range<i32> = 19_000..20_000;
pub(crate) const VALID_MESSAGE_FIELD_NUMBERS: Range<i32> = 1..536_870_912;

/// Cardinality determines whether a field is optional, required, or repeated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cardinality {
    /// The field appears zero or one times.
    Optional,
    /// The field appears exactly one time. This cardinality is invalid with Proto3.
    Required,
    /// The field appears zero or more times.
    Repeated,
}

/// The syntax of a proto file.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Syntax {
    /// The `proto2` syntax.
    Proto2,
    /// The `proto3` syntax.
    Proto3,
}

/// The type of a protobuf message field.
#[derive(Clone, PartialEq, Eq)]
pub enum Kind {
    /// The protobuf `double` type.
    Double,
    /// The protobuf `float` type.
    Float,
    /// The protobuf `int32` type.
    Int32,
    /// The protobuf `int64` type.
    Int64,
    /// The protobuf `uint32` type.
    Uint32,
    /// The protobuf `uint64` type.
    Uint64,
    /// The protobuf `sint32` type.
    Sint32,
    /// The protobuf `sint64` type.
    Sint64,
    /// The protobuf `fixed32` type.
    Fixed32,
    /// The protobuf `fixed64` type.
    Fixed64,
    /// The protobuf `sfixed32` type.
    Sfixed32,
    /// The protobuf `sfixed64` type.
    Sfixed64,
    /// The protobuf `bool` type.
    Bool,
    /// The protobuf `string` type.
    String,
    /// The protobuf `bytes` type.
    Bytes,
    /// A protobuf message type.
    Message(MessageDescriptor),
    /// A protobuf enum type.
    Enum(EnumDescriptor),
}

#[derive(Copy, Clone)]
enum KindIndex {
    Double,
    Float,
    Int32,
    Int64,
    Uint32,
    Uint64,
    Sint32,
    Sint64,
    Fixed32,
    Fixed64,
    Sfixed32,
    Sfixed64,
    Bool,
    String,
    Bytes,
    Message(MessageIndex),
    Enum(EnumIndex),
    Group(MessageIndex),
}

type DescriptorIndex = u32;
type FileIndex = DescriptorIndex;
type ServiceIndex = DescriptorIndex;
type MethodIndex = DescriptorIndex;
type MessageIndex = DescriptorIndex;
type FieldIndex = DescriptorIndex;
type OneofIndex = DescriptorIndex;
type ExtensionIndex = DescriptorIndex;
type EnumIndex = DescriptorIndex;
type EnumValueIndex = DescriptorIndex;

/// A `DescriptorPool` is a collection of related descriptors. Typically it will be created from
/// a [`FileDescriptorSet`][prost_types::FileDescriptorSet] output by the protobuf compiler
/// (see [`DescriptorPool::from_file_descriptor_set`]) but it may also be built up by adding files individually.
///
/// Methods like [`MessageDescriptor::extensions`] will be scoped to just the files contained within the parent
/// `DescriptorPool`.
///
/// This type uses reference counting internally so it is cheap to clone. Modifying an instance of a
/// pool will not update any existing clones of the instance.
#[derive(Clone, Default)]
pub struct DescriptorPool {
    inner: Arc<DescriptorPoolInner>,
}

#[derive(Clone, Default)]
struct DescriptorPoolInner {
    names: HashMap<Box<str>, Definition>,
    file_names: HashMap<Box<str>, FileIndex>,
    files: Vec<FileDescriptorInner>,
    messages: Vec<MessageDescriptorInner>,
    enums: Vec<EnumDescriptorInner>,
    extensions: Vec<ExtensionDescriptorInner>,
    services: Vec<ServiceDescriptorInner>,
}

#[derive(Clone)]
struct Identity {
    file: FileIndex,
    path: Box<[i32]>,
    full_name: Box<str>,
    name_index: usize,
}

#[derive(Clone, Debug)]
struct Definition {
    file: FileIndex,
    path: Box<[i32]>,
    kind: DefinitionKind,
}

#[derive(Copy, Clone, Debug)]
enum DefinitionKind {
    Package,
    Message(MessageIndex),
    Field(MessageIndex),
    Oneof(MessageIndex),
    Service(ServiceIndex),
    Method(ServiceIndex),
    Enum(EnumIndex),
    EnumValue(EnumIndex),
    Extension(ExtensionIndex),
}

/// A single source file containing protobuf messages and services.
#[derive(Clone, PartialEq, Eq)]
pub struct FileDescriptor {
    pool: DescriptorPool,
    index: FileIndex,
}

#[derive(Clone)]
struct FileDescriptorInner {
    syntax: Syntax,
    raw: FileDescriptorProto,
    prost: prost_types::FileDescriptorProto,
    dependencies: Vec<FileIndex>,
    transitive_dependencies: HashSet<FileIndex>,
}

/// A protobuf message definition.
#[derive(Clone, PartialEq, Eq)]
pub struct MessageDescriptor {
    pool: DescriptorPool,
    index: MessageIndex,
}

#[derive(Clone)]
struct MessageDescriptorInner {
    id: Identity,
    parent: Option<MessageIndex>,
    extensions: Vec<ExtensionIndex>,
    fields: Vec<FieldDescriptorInner>,
    field_numbers: BTreeMap<u32, FieldIndex>,
    field_names: HashMap<Box<str>, FieldIndex>,
    field_json_names: HashMap<Box<str>, FieldIndex>,
    oneofs: Vec<OneofDescriptorInner>,
}

/// A oneof field in a protobuf message.
#[derive(Clone, PartialEq, Eq)]
pub struct OneofDescriptor {
    message: MessageDescriptor,
    index: OneofIndex,
}

#[derive(Clone)]
struct OneofDescriptorInner {
    id: Identity,
    fields: Vec<FieldIndex>,
}

/// A protobuf message definition.
#[derive(Clone, PartialEq, Eq)]
pub struct FieldDescriptor {
    message: MessageDescriptor,
    index: FieldIndex,
}

#[derive(Clone)]
struct FieldDescriptorInner {
    id: Identity,
    number: u32,
    json_name: Box<str>,
    kind: KindIndex,
    oneof: Option<OneofIndex>,
    is_packed: bool,
    supports_presence: bool,
    cardinality: Cardinality,
    default: Option<Value>,
}

/// A protobuf extension field definition.
#[derive(Clone, PartialEq, Eq)]
pub struct ExtensionDescriptor {
    pool: DescriptorPool,
    index: ExtensionIndex,
}

#[derive(Clone)]
pub struct ExtensionDescriptorInner {
    id: Identity,
    parent: Option<MessageIndex>,
    number: u32,
    json_name: Box<str>,
    extendee: MessageIndex,
    kind: KindIndex,
    is_packed: bool,
    cardinality: Cardinality,
    default: Option<Value>,
}

/// A protobuf enum type.
#[derive(Clone, PartialEq, Eq)]
pub struct EnumDescriptor {
    pool: DescriptorPool,
    index: EnumIndex,
}

#[derive(Clone)]
struct EnumDescriptorInner {
    id: Identity,
    parent: Option<MessageIndex>,
    values: Vec<EnumValueDescriptorInner>,
    value_numbers: Vec<(i32, EnumValueIndex)>,
    value_names: HashMap<Box<str>, EnumValueIndex>,
    allow_alias: bool,
}

/// A value in a protobuf enum type.
#[derive(Clone, PartialEq, Eq)]
pub struct EnumValueDescriptor {
    parent: EnumDescriptor,
    index: EnumValueIndex,
}

#[derive(Clone)]
struct EnumValueDescriptorInner {
    id: Identity,
    number: i32,
}

/// A protobuf service definition.
#[derive(Clone, PartialEq, Eq)]
pub struct ServiceDescriptor {
    pool: DescriptorPool,
    index: ServiceIndex,
}

#[derive(Clone)]
struct ServiceDescriptorInner {
    id: Identity,
    methods: Vec<MethodDescriptorInner>,
}

/// A method definition for a [`ServiceDescriptor`].
#[derive(Clone, PartialEq, Eq)]
pub struct MethodDescriptor {
    service: ServiceDescriptor,
    index: MethodIndex,
}

#[derive(Clone)]
struct MethodDescriptorInner {
    id: Identity,
    input: MessageIndex,
    output: MessageIndex,
}

impl Identity {
    fn new(file: FileIndex, path: &[i32], full_name: &str, name: &str) -> Identity {
        debug_assert!(full_name.ends_with(name));
        let name_index = full_name.len() - name.len();
        debug_assert!(name_index == 0 || full_name.as_bytes()[name_index - 1] == b'.');
        Identity {
            file,
            path: path.into(),
            full_name: full_name.into(),
            name_index,
        }
    }

    fn full_name(&self) -> &str {
        &self.full_name
    }

    fn name(&self) -> &str {
        &self.full_name[self.name_index..]
    }
}

impl KindIndex {
    fn is_packable(&self) -> bool {
        match self {
            KindIndex::Double
            | KindIndex::Float
            | KindIndex::Int32
            | KindIndex::Int64
            | KindIndex::Uint32
            | KindIndex::Uint64
            | KindIndex::Sint32
            | KindIndex::Sint64
            | KindIndex::Fixed32
            | KindIndex::Fixed64
            | KindIndex::Sfixed32
            | KindIndex::Sfixed64
            | KindIndex::Bool
            | KindIndex::Enum(_) => true,
            KindIndex::String | KindIndex::Bytes | KindIndex::Message(_) | KindIndex::Group(_) => {
                false
            }
        }
    }

    fn is_message(&self) -> bool {
        matches!(self, KindIndex::Message(_) | KindIndex::Group(_))
    }
}

impl fmt::Debug for KindIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KindIndex::Double => write!(f, "double"),
            KindIndex::Float => write!(f, "float"),
            KindIndex::Int32 => write!(f, "int32"),
            KindIndex::Int64 => write!(f, "int64"),
            KindIndex::Uint32 => write!(f, "uint32"),
            KindIndex::Uint64 => write!(f, "uint64"),
            KindIndex::Sint32 => write!(f, "sint32"),
            KindIndex::Sint64 => write!(f, "sint64"),
            KindIndex::Fixed32 => write!(f, "fixed32"),
            KindIndex::Fixed64 => write!(f, "fixed64"),
            KindIndex::Sfixed32 => write!(f, "sfixed32"),
            KindIndex::Sfixed64 => write!(f, "sfixed64"),
            KindIndex::Bool => write!(f, "bool"),
            KindIndex::String => write!(f, "string"),
            KindIndex::Bytes => write!(f, "bytes"),
            KindIndex::Message(_) | KindIndex::Group(_) => write!(f, "message"),
            KindIndex::Enum(_) => write!(f, "enum"),
        }
    }
}

impl DefinitionKind {
    fn is_parent(&self) -> bool {
        match self {
            DefinitionKind::Package => true,
            DefinitionKind::Message(_) => true,
            DefinitionKind::Field(_) => false,
            DefinitionKind::Oneof(_) => false,
            DefinitionKind::Service(_) => true,
            DefinitionKind::Method(_) => false,
            DefinitionKind::Enum(_) => true,
            DefinitionKind::EnumValue(_) => false,
            DefinitionKind::Extension(_) => false,
        }
    }
}

impl DescriptorPoolInner {
    fn get_by_name(&self, name: &str) -> Option<&Definition> {
        let name = name.strip_prefix('.').unwrap_or(name);
        self.names.get(name)
    }
}

fn to_index(i: usize) -> DescriptorIndex {
    i.try_into().expect("index too large")
}

fn find_message_proto<'a>(file: &'a FileDescriptorProto, path: &[i32]) -> &'a DescriptorProto {
    debug_assert_ne!(path.len(), 0);
    debug_assert_eq!(path.len() % 2, 0);

    let mut message: Option<&'a types::DescriptorProto> = None;
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

fn find_enum_proto<'a>(file: &'a FileDescriptorProto, path: &[i32]) -> &'a EnumDescriptorProto {
    debug_assert_ne!(path.len(), 0);
    debug_assert_eq!(path.len() % 2, 0);
    if path.len() == 2 {
        debug_assert_eq!(path[0], tag::file::ENUM_TYPE);
        &file.enum_type[path[1] as usize]
    } else {
        let message = find_message_proto(file, &path[..path.len() - 2]);
        debug_assert_eq!(path[path.len() - 2], tag::message::ENUM_TYPE);
        &message.enum_type[path[path.len() - 1] as usize]
    }
}

#[test]
fn assert_descriptor_send_sync() {
    fn test_send_sync<T: Send + Sync>() {}

    test_send_sync::<DescriptorPool>();
    test_send_sync::<Kind>();
    test_send_sync::<DescriptorError>();
}
