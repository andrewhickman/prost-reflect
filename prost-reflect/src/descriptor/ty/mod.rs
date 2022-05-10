mod build;
#[cfg(test)]
mod tests;

use std::{
    collections::{
        hash_map::{self, HashMap},
        BTreeMap,
    },
    convert::TryInto,
    fmt,
    ops::{Range, RangeInclusive},
};

use prost::encoding::WireType;
use prost_types::{
    field_descriptor_proto, DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto,
    FieldDescriptorProto, FileDescriptorProto, OneofDescriptorProto,
};

use crate::descriptor::{
    debug_fmt_iter, make_full_name, parse_name, parse_namespace, DescriptorError, DescriptorPool,
    FileDescriptor, MAP_ENTRY_KEY_NUMBER, MAP_ENTRY_VALUE_NUMBER,
};

use super::{EnumIndex, EnumValueIndex, ExtensionIndex, FileIndex, MessageIndex, OneofIndex};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) struct TypeId(field_descriptor_proto::Type, u32);

#[derive(Clone, Default)]
pub(super) struct TypeMap {
    named_types: HashMap<Box<str>, TypeId>,
    messages: Vec<MessageDescriptorInner>,
    enums: Vec<EnumDescriptorInner>,
    extensions: Vec<ExtensionDescriptorInner>,
}

/// A protobuf message definition.
#[derive(Clone, PartialEq, Eq)]
pub struct MessageDescriptor {
    pool: DescriptorPool,
    index: MessageIndex,
}

#[derive(Clone)]
struct MessageDescriptorInner {
    full_name: Box<str>,
    file: FileIndex,
    parent: ParentKind,
    is_map_entry: bool,
    fields: BTreeMap<u32, FieldDescriptorInner>,
    field_names: HashMap<Box<str>, u32>,
    field_json_names: HashMap<Box<str>, u32>,
    oneof_decls: Box<[OneofDescriptorInner]>,
    extensions: Vec<ExtensionIndex>,
}

/// A oneof field in a protobuf message.
#[derive(Clone, PartialEq, Eq)]
pub struct OneofDescriptor {
    message: MessageDescriptor,
    index: OneofIndex,
}

#[derive(Clone)]
struct OneofDescriptorInner {
    name: Box<str>,
    full_name: Box<str>,
    fields: Vec<u32>,
}

/// A protobuf message definition.
#[derive(Clone, PartialEq, Eq)]
pub struct FieldDescriptor {
    message: MessageDescriptor,
    field: u32,
}

#[derive(Clone)]
struct FieldDescriptorInner {
    name: Box<str>,
    full_name: Box<str>,
    json_name: Box<str>,
    is_group: bool,
    cardinality: Cardinality,
    is_packed: bool,
    supports_presence: bool,
    default_value: Option<crate::Value>,
    oneof_index: Option<usize>,
    ty: TypeId,
}

/// A protobuf extension field definition.
#[derive(Clone, PartialEq, Eq)]
pub struct ExtensionDescriptor {
    pool: DescriptorPool,
    index: ExtensionIndex,
}

#[derive(Clone)]
pub struct ExtensionDescriptorInner {
    field: FieldDescriptorInner,
    number: u32,
    file: FileIndex,
    parent: ParentKind,
    extendee: TypeId,
    json_name: Box<str>,
}

/// A protobuf enum type.
#[derive(Clone, PartialEq, Eq)]
pub struct EnumDescriptor {
    pool: DescriptorPool,
    index: EnumIndex,
}

#[derive(Clone)]
struct EnumDescriptorInner {
    full_name: Box<str>,
    file: FileIndex,
    parent: ParentKind,
    value_names: HashMap<Box<str>, EnumValueIndex>,
    values: Vec<EnumValueDescriptorInner>,
    default_value: EnumValueIndex,
}

/// A value in a protobuf enum type.
#[derive(Clone, PartialEq, Eq)]
pub struct EnumValueDescriptor {
    parent: EnumDescriptor,
    index: EnumValueIndex,
}

#[derive(Clone)]
struct EnumValueDescriptorInner {
    name: Box<str>,
    number: i32,
    full_name: Box<str>,
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

#[derive(Copy, Clone, Debug)]
enum ParentKind {
    File,
    Message { index: MessageIndex },
}

impl MessageDescriptor {
    pub(in crate::descriptor) fn new(pool: DescriptorPool, ty: TypeId) -> Self {
        debug_assert_eq!(ty.0, field_descriptor_proto::Type::Message);
        MessageDescriptor { pool, index: ty.1 }
    }

    pub(in crate::descriptor) fn iter(
        pool: &DescriptorPool,
    ) -> impl ExactSizeIterator<Item = Self> + '_ {
        pool.inner
            .type_map
            .messages()
            .map(move |ty| MessageDescriptor::new(pool.clone(), ty))
    }

    pub(in crate::descriptor) fn try_get_by_name(
        pool: &DescriptorPool,
        name: &str,
    ) -> Option<Self> {
        let ty = pool.inner.type_map.get_by_name(name)?;
        if !ty.is_message() {
            return None;
        }
        Some(MessageDescriptor::new(pool.clone(), ty))
    }

    /// Gets a reference to the [`DescriptorPool`] this message is defined in.
    pub fn parent_pool(&self) -> &DescriptorPool {
        &self.pool
    }

    /// Gets the [`FileDescriptor`] this message is defined in.
    pub fn parent_file(&self) -> FileDescriptor {
        FileDescriptor::new(self.pool.clone(), self.inner().file as _)
    }

    /// Gets the parent message type if this message type is nested inside a another message, or `None` otherwise
    pub fn parent_message(&self) -> Option<MessageDescriptor> {
        self.inner()
            .parent
            .as_message()
            .map(|ty| MessageDescriptor::new(self.pool.clone(), ty))
    }

    /// Gets the short name of the message type, e.g. `MyMessage`.
    pub fn name(&self) -> &str {
        parse_name(self.full_name())
    }

    /// Gets the full name of the message type, e.g. `my.package.MyMessage`.
    pub fn full_name(&self) -> &str {
        &self.inner().full_name
    }

    /// Gets the name of the package this message type is defined in, e.g. `my.package`.
    ///
    /// If no package name is set, an empty string is returned.
    pub fn package_name(&self) -> &str {
        self.parent_file_descriptor_proto().package()
    }

    /// Gets a reference to the [`FileDescriptorProto`] in which this message is defined.
    pub fn parent_file_descriptor_proto(&self) -> &FileDescriptorProto {
        get_file_descriptor_proto(&self.pool, self.inner().file)
    }

    /// Gets a reference to the raw [`DescriptorProto`] wrapped by this [`MessageDescriptor`].
    pub fn descriptor_proto(&self) -> &DescriptorProto {
        find_message_descriptor_proto(self.parent_pool(), self.inner().file, self.index)
    }

    /// Gets an iterator yielding a [`FieldDescriptor`] for each field defined in this message.
    pub fn fields(&self) -> impl ExactSizeIterator<Item = FieldDescriptor> + '_ {
        self.inner()
            .fields
            .keys()
            .map(move |&field| FieldDescriptor {
                message: self.clone(),
                field,
            })
    }

    /// Gets an iterator yielding a [`OneofDescriptor`] for each oneof field defined in this message.
    pub fn oneofs(&self) -> impl ExactSizeIterator<Item = OneofDescriptor> + '_ {
        (0..self.inner().oneof_decls.len())
            .map(move |index| OneofDescriptor::new(self.clone(), index))
    }

    /// Gets a [`FieldDescriptor`] with the given number, or `None` if no such field exists.
    pub fn get_field(&self, number: u32) -> Option<FieldDescriptor> {
        if self.inner().fields.contains_key(&number) {
            Some(FieldDescriptor {
                message: self.clone(),
                field: number,
            })
        } else {
            None
        }
    }

    /// Gets a [`FieldDescriptor`] with the given name, or `None` if no such field exists.
    pub fn get_field_by_name(&self, name: &str) -> Option<FieldDescriptor> {
        self.inner()
            .field_names
            .get(name)
            .map(|&number| FieldDescriptor {
                message: self.clone(),
                field: number,
            })
    }

    /// Gets a [`FieldDescriptor`] with the given JSON name, or `None` if no such field exists.
    pub fn get_field_by_json_name(&self, json_name: &str) -> Option<FieldDescriptor> {
        self.inner()
            .field_json_names
            .get(json_name)
            .map(|&number| FieldDescriptor {
                message: self.clone(),
                field: number,
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
        self.inner().is_map_entry
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
            .expect("map entry should have key field")
    }

    /// Gets an iterator over reserved field number ranges in this message.
    pub fn reserved_ranges(&self) -> impl ExactSizeIterator<Item = Range<u32>> + '_ {
        self.descriptor_proto()
            .reserved_range
            .iter()
            .map(|n| (n.start() as u32)..(n.end() as u32))
    }

    /// Gets an iterator over reserved field names in this message.
    pub fn reserved_names(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.descriptor_proto()
            .reserved_name
            .iter()
            .map(|n| n.as_ref())
    }

    /// Gets an iterator over extension field number ranges in this message.
    pub fn extension_ranges(&self) -> impl ExactSizeIterator<Item = Range<u32>> + '_ {
        self.descriptor_proto()
            .extension_range
            .iter()
            .map(|n| (n.start() as u32)..(n.end() as u32))
    }

    /// Gets an iterator over extensions to this message.
    ///
    /// Note this iterates over extension fields defined in any file which extend this message, rather than
    /// extensions defined nested within this message.
    pub fn extensions(&self) -> impl ExactSizeIterator<Item = ExtensionDescriptor> + '_ {
        self.inner()
            .extensions
            .iter()
            .map(move |&index| ExtensionDescriptor {
                pool: self.pool.clone(),
                index,
            })
    }

    /// Gets an extension to this message by its number, or `None` if no such extension exists.
    pub fn get_extension(&self, number: u32) -> Option<ExtensionDescriptor> {
        self.extensions().find(|ext| ext.number() == number)
    }

    /// Gets an extension to this message by its JSON name (e.g. `[my.package.my_extension]`), or `None` if no such extension exists.
    pub fn get_extension_by_json_name(&self, name: &str) -> Option<ExtensionDescriptor> {
        self.extensions().find(|ext| ext.json_name() == name)
    }

    fn inner(&self) -> &MessageDescriptorInner {
        self.pool.inner.type_map.get_message(self.index)
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
        &self.inner().name
    }

    /// Gets the full name of the message field, e.g. `my.package.MyMessage.my_field`.
    pub fn full_name(&self) -> &str {
        &self.inner().full_name
    }

    /// Gets a reference to the raw [`FieldDescriptorProto`] wrapped by this [`FieldDescriptor`].
    pub fn field_descriptor_proto(&self) -> &FieldDescriptorProto {
        self.parent_message()
            .descriptor_proto()
            .field
            .iter()
            .find(|field| field.number() as u32 == self.field)
            .expect("field not found")
    }

    /// Gets the unique number for this message field.
    pub fn number(&self) -> u32 {
        self.field
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
        self.inner().is_group
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
        self.inner().ty.to_kind(&self.message.pool)
    }

    /// Gets a [`OneofDescriptor`] representing the oneof containing this field,
    /// or `None` if this field is not contained in a oneof.
    pub fn containing_oneof(&self) -> Option<OneofDescriptor> {
        self.inner()
            .oneof_index
            .map(|index| OneofDescriptor::new(self.message.clone(), index))
    }

    pub(crate) fn default_value(&self) -> Option<&crate::Value> {
        self.inner().default_value.as_ref()
    }

    pub(crate) fn is_packable(&self) -> bool {
        self.inner().ty.is_packable()
    }

    fn inner(&self) -> &FieldDescriptorInner {
        &self.message.inner().fields[&self.field]
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
    pub(in crate::descriptor) fn iter(
        pool: &DescriptorPool,
    ) -> impl ExactSizeIterator<Item = Self> + '_ {
        pool.inner
            .type_map
            .extensions()
            .map(move |index| ExtensionDescriptor {
                pool: pool.clone(),
                index: index.try_into().expect("index too large"),
            })
    }

    /// Gets a reference to the [`DescriptorPool`] this extension field is defined in.
    pub fn parent_pool(&self) -> &DescriptorPool {
        &self.pool
    }

    /// Gets the [`FileDescriptor`] this extension field is defined in.
    pub fn parent_file(&self) -> FileDescriptor {
        FileDescriptor::new(self.pool.clone(), self.inner().file as _)
    }

    /// Gets the parent message type if this extension is defined within another message, or `None` otherwise.
    ///
    /// Note this just corresponds to where the extension was defined in the proto file. See [`containing_message`][ExtensionDescriptor::containing_message]
    /// for the message this field extends.
    pub fn parent_message(&self) -> Option<MessageDescriptor> {
        self.inner()
            .parent
            .as_message()
            .map(|ty| MessageDescriptor::new(self.pool.clone(), ty))
    }

    /// Gets the short name of the extension field type, e.g. `my_extension`.
    pub fn name(&self) -> &str {
        &self.field_inner().name
    }

    /// Gets the full name of the extension field, e.g. `my.package.ParentMessage.my_field`.
    ///
    /// Note this includes the name of the parent message if any, not the message this field extends.
    pub fn full_name(&self) -> &str {
        &self.field_inner().full_name
    }

    /// Gets the name of the package this extension field is defined in, e.g. `my.package`.
    ///
    /// If no package name is set, an empty string is returned.
    pub fn package_name(&self) -> &str {
        self.parent_file_descriptor_proto().package()
    }

    /// Gets a reference to the [`FileDescriptorProto`] in which this extension is defined.
    pub fn parent_file_descriptor_proto(&self) -> &FileDescriptorProto {
        get_file_descriptor_proto(&self.pool, self.inner().file)
    }

    /// Gets a reference to the raw [`FieldDescriptorProto`] wrapped by this [`ExtensionDescriptor`].
    pub fn field_descriptor_proto(&self) -> &FieldDescriptorProto {
        let name = self.name();
        let inner = self.inner();
        match inner.parent {
            ParentKind::File => get_file_descriptor_proto(&self.pool, inner.file)
                .extension
                .iter()
                .find(|extension| extension.name() == name)
                .expect("extension not found"),
            ParentKind::Message {
                index: message_index,
            } => find_message_descriptor_proto(&self.pool, inner.file, message_index)
                .extension
                .iter()
                .find(|extension| extension.name() == name)
                .expect("extension not found"),
        }
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
        self.field_inner().is_group
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
        self.field_inner().is_packed
    }

    /// The cardinality of this field.
    pub fn cardinality(&self) -> Cardinality {
        self.field_inner().cardinality
    }

    /// Whether this field supports distinguishing between an unpopulated field and
    /// the default value.
    ///
    /// For proto2 messages this returns `true` for all non-repeated fields.
    /// For proto3 this returns `true` for message fields, and fields contained
    /// in a `oneof`.
    pub fn supports_presence(&self) -> bool {
        self.field_inner().supports_presence
    }

    /// Gets the [`Kind`] of this field.
    pub fn kind(&self) -> Kind {
        self.field_inner().ty.to_kind(&self.pool)
    }

    /// Gets the containing message that this field extends.
    pub fn containing_message(&self) -> MessageDescriptor {
        MessageDescriptor::new(self.pool.clone(), self.inner().extendee)
    }

    pub(crate) fn default_value(&self) -> Option<&crate::Value> {
        self.field_inner().default_value.as_ref()
    }

    pub(crate) fn is_packable(&self) -> bool {
        self.field_inner().ty.is_packable()
    }

    fn field_inner(&self) -> &FieldDescriptorInner {
        &self.inner().field
    }

    fn inner(&self) -> &ExtensionDescriptorInner {
        self.pool.inner.type_map.get_extension(self.index)
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

impl Kind {
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

    pub(crate) fn wire_type(&self) -> WireType {
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

impl EnumDescriptor {
    pub(in crate::descriptor) fn new(pool: DescriptorPool, ty: TypeId) -> Self {
        debug_assert_eq!(ty.0, field_descriptor_proto::Type::Enum);
        EnumDescriptor { pool, index: ty.1 }
    }

    pub(in crate::descriptor) fn iter(
        pool: &DescriptorPool,
    ) -> impl ExactSizeIterator<Item = Self> + '_ {
        pool.inner
            .type_map
            .enums()
            .map(move |ty| EnumDescriptor::new(pool.clone(), ty))
    }

    pub(in crate::descriptor) fn try_get_by_name(
        pool: &DescriptorPool,
        name: &str,
    ) -> Option<Self> {
        let ty = pool.inner.type_map.get_by_name(name)?;
        if !ty.is_enum() {
            return None;
        }
        Some(EnumDescriptor::new(pool.clone(), ty))
    }

    /// Gets a reference to the [`DescriptorPool`] this enum type is defined in.
    pub fn parent_pool(&self) -> &DescriptorPool {
        &self.pool
    }

    /// Gets the [`FileDescriptor`] this enum type is defined in.
    pub fn parent_file(&self) -> FileDescriptor {
        FileDescriptor::new(self.pool.clone(), self.inner().file as _)
    }

    /// Gets the parent message type if this enum type is nested inside a another message, or `None` otherwise
    pub fn parent_message(&self) -> Option<MessageDescriptor> {
        self.inner()
            .parent
            .as_message()
            .map(|ty| MessageDescriptor::new(self.pool.clone(), ty))
    }

    /// Gets the short name of the enum type, e.g. `MyEnum`.
    pub fn name(&self) -> &str {
        parse_name(self.full_name())
    }

    /// Gets the full name of the enum, e.g. `my.package.MyEnum`.
    pub fn full_name(&self) -> &str {
        &self.inner().full_name
    }

    /// Gets the name of the package this enum type is defined in, e.g. `my.package`.
    ///
    /// If no package name is set, an empty string is returned.
    pub fn package_name(&self) -> &str {
        self.parent_file_descriptor_proto().package()
    }

    /// Gets a reference to the [`FileDescriptorProto`] in which this enum is defined.
    pub fn parent_file_descriptor_proto(&self) -> &FileDescriptorProto {
        get_file_descriptor_proto(&self.pool, self.inner().file)
    }

    /// Gets a reference to the raw [`EnumDescriptorProto`] wrapped by this [`EnumDescriptor`].
    pub fn enum_descriptor_proto(&self) -> &EnumDescriptorProto {
        let name = self.name();
        let inner = self.inner();
        match inner.parent {
            ParentKind::File => get_file_descriptor_proto(&self.pool, inner.file)
                .enum_type
                .iter()
                .find(|extension| extension.name() == name)
                .expect("extension not found"),
            ParentKind::Message {
                index: message_index,
            } => find_message_descriptor_proto(&self.pool, inner.file, message_index)
                .enum_type
                .iter()
                .find(|extension| extension.name() == name)
                .expect("extension not found"),
        }
    }

    /// Gets the default value for the enum type.
    pub fn default_value(&self) -> EnumValueDescriptor {
        EnumValueDescriptor {
            parent: self.clone(),
            index: self.inner().default_value,
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
    /// If the enum was defined with the `allow_alias` option and has multiple values with the given number, it is
    /// unspecified which one will be returned.
    pub fn get_value(&self, number: i32) -> Option<EnumValueDescriptor> {
        match self
            .inner()
            .values
            .binary_search_by_key(&number, |v| v.number)
        {
            Ok(index) => Some(EnumValueDescriptor::new(self.clone(), index)),
            Err(_) => None,
        }
    }

    /// Gets an iterator yielding a [`EnumValueDescriptor`] for each value in this enum.
    pub fn values(&self) -> impl ExactSizeIterator<Item = EnumValueDescriptor> + '_ {
        (0..self.inner().values.len())
            .map(move |index| EnumValueDescriptor::new(self.clone(), index))
    }

    /// Gets an iterator over reserved value number ranges in this enum.
    pub fn reserved_ranges(&self) -> impl ExactSizeIterator<Item = RangeInclusive<i32>> + '_ {
        self.enum_descriptor_proto()
            .reserved_range
            .iter()
            .map(|n| n.start()..=n.end())
    }

    /// Gets an iterator over reserved value names in this enum.
    pub fn reserved_names(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.enum_descriptor_proto()
            .reserved_name
            .iter()
            .map(|n| n.as_ref())
    }

    fn inner(&self) -> &EnumDescriptorInner {
        self.pool.inner.type_map.get_enum(self.index)
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
    fn new(parent: EnumDescriptor, index: usize) -> EnumValueDescriptor {
        EnumValueDescriptor {
            parent,
            index: index.try_into().expect("index too large"),
        }
    }

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
        &self.enum_value_ty().name
    }

    /// Gets the full name of the enum, e.g. `my.package.MY_VALUE`.
    pub fn full_name(&self) -> &str {
        &self.enum_value_ty().full_name
    }

    /// Gets a reference to the raw [`EnumValueDescriptorProto`] wrapped by this [`EnumValueDescriptor`].
    pub fn enum_value_descriptor_proto(&self) -> &EnumValueDescriptorProto {
        self.parent_enum()
            .enum_descriptor_proto()
            .value
            .iter()
            .find(|value| value.name() == self.name())
            .expect("enum value not found")
    }

    /// Gets the number representing this enum value.
    pub fn number(&self) -> i32 {
        self.enum_value_ty().number
    }

    fn enum_value_ty(&self) -> &EnumValueDescriptorInner {
        &self.parent.inner().values[self.index as usize]
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
    fn new(message: MessageDescriptor, index: usize) -> Self {
        OneofDescriptor {
            message,
            index: index.try_into().expect("index too large"),
        }
    }

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
        &self.oneof_ty().name
    }

    /// Gets the full name of the oneof, e.g. `my.package.MyMessage.my_oneof`.
    pub fn full_name(&self) -> &str {
        &self.oneof_ty().full_name
    }

    /// Gets a reference to the raw [`OneofDescriptorProto`] wrapped by this [`OneofDescriptor`].
    pub fn oneof_descriptor_proto(&self) -> &OneofDescriptorProto {
        &self.parent_message().descriptor_proto().oneof_decl[self.index as usize]
    }

    /// Gets an iterator yielding a [`FieldDescriptor`] for each field of the parent message this oneof contains.
    pub fn fields(&self) -> impl ExactSizeIterator<Item = FieldDescriptor> + '_ {
        self.oneof_ty()
            .fields
            .iter()
            .map(move |&field| FieldDescriptor {
                message: self.message.clone(),
                field,
            })
    }

    fn oneof_ty(&self) -> &OneofDescriptorInner {
        &self.message.inner().oneof_decls[self.index as usize]
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

impl TypeMap {
    pub fn shrink_to_fit(&mut self) {
        self.named_types.shrink_to_fit();
        self.messages.shrink_to_fit();
        self.enums.shrink_to_fit();
        self.extensions.shrink_to_fit();
    }

    pub fn try_get_by_name(&self, full_name: &str) -> Result<TypeId, DescriptorError> {
        self.get_by_name(full_name)
            .ok_or_else(|| DescriptorError::type_not_found(full_name))
    }

    pub fn get_by_name(&self, full_name: &str) -> Option<TypeId> {
        let full_name = full_name.strip_prefix('.').unwrap_or(full_name);
        self.named_types.get(full_name).copied()
    }

    pub fn resolve_type_name(
        &self,
        mut namespace: &str,
        type_name: &str,
    ) -> Result<TypeId, DescriptorError> {
        match type_name.strip_prefix('.') {
            Some(full_name) => self.try_get_by_name(full_name),
            None => loop {
                let full_name = make_full_name(namespace, type_name);
                if let Some(ty) = self.get_by_name(&full_name) {
                    break Ok(ty);
                } else if namespace.is_empty() {
                    break Err(DescriptorError::type_not_found(type_name));
                } else {
                    namespace = parse_namespace(namespace);
                }
            },
        }
    }

    fn add_named_type(&mut self, full_name: Box<str>, ty: TypeId) -> Result<(), DescriptorError> {
        let full_name = full_name
            .strip_prefix('.')
            .map(Box::from)
            .unwrap_or(full_name);
        match self.named_types.entry(full_name) {
            hash_map::Entry::Occupied(entry) => {
                Err(DescriptorError::type_already_exists(entry.key()))
            }
            hash_map::Entry::Vacant(entry) => {
                entry.insert(ty);
                Ok(())
            }
        }
    }

    fn get_message(&self, index: MessageIndex) -> &MessageDescriptorInner {
        &self.messages[index as usize]
    }

    fn get_message_mut(&mut self, ty: TypeId) -> &mut MessageDescriptorInner {
        debug_assert_eq!(ty.0, field_descriptor_proto::Type::Message);
        &mut self.messages[ty.1 as usize]
    }

    fn get_enum(&self, index: EnumIndex) -> &EnumDescriptorInner {
        &self.enums[index as usize]
    }

    fn get_extension(&self, index: ExtensionIndex) -> &ExtensionDescriptorInner {
        &self.extensions[index as usize]
    }

    fn messages(&self) -> impl ExactSizeIterator<Item = TypeId> {
        (0..self.messages.len()).map(TypeId::new_message)
    }

    fn enums(&self) -> impl ExactSizeIterator<Item = TypeId> {
        (0..self.enums.len()).map(TypeId::new_enum)
    }

    fn extensions(&self) -> impl ExactSizeIterator<Item = usize> {
        0..self.extensions.len()
    }
}

impl TypeId {
    pub fn new_message(index: usize) -> Self {
        TypeId(
            field_descriptor_proto::Type::Message,
            index.try_into().expect("invalid message index"),
        )
    }

    pub fn new_enum(index: usize) -> Self {
        TypeId(
            field_descriptor_proto::Type::Enum,
            index.try_into().expect("invalid enum index"),
        )
    }

    pub(crate) fn new_scalar(scalar: field_descriptor_proto::Type) -> Self {
        debug_assert!(
            scalar != field_descriptor_proto::Type::Message
                && scalar != field_descriptor_proto::Type::Enum
                && scalar != field_descriptor_proto::Type::Group
        );
        TypeId(scalar, 0)
    }

    pub fn is_message(&self) -> bool {
        self.0 == field_descriptor_proto::Type::Message
    }

    pub fn is_enum(&self) -> bool {
        self.0 == field_descriptor_proto::Type::Enum
    }

    fn is_packable(&self) -> bool {
        match self.0 {
            field_descriptor_proto::Type::Double
            | field_descriptor_proto::Type::Float
            | field_descriptor_proto::Type::Int64
            | field_descriptor_proto::Type::Uint64
            | field_descriptor_proto::Type::Int32
            | field_descriptor_proto::Type::Fixed64
            | field_descriptor_proto::Type::Fixed32
            | field_descriptor_proto::Type::Bool
            | field_descriptor_proto::Type::Uint32
            | field_descriptor_proto::Type::Enum
            | field_descriptor_proto::Type::Sfixed32
            | field_descriptor_proto::Type::Sfixed64
            | field_descriptor_proto::Type::Sint32
            | field_descriptor_proto::Type::Sint64 => true,
            field_descriptor_proto::Type::String
            | field_descriptor_proto::Type::Bytes
            | field_descriptor_proto::Type::Group
            | field_descriptor_proto::Type::Message => false,
        }
    }

    fn to_kind(self, pool: &DescriptorPool) -> Kind {
        match self.0 {
            field_descriptor_proto::Type::Double => Kind::Double,
            field_descriptor_proto::Type::Float => Kind::Float,
            field_descriptor_proto::Type::Int64 => Kind::Int64,
            field_descriptor_proto::Type::Uint64 => Kind::Uint64,
            field_descriptor_proto::Type::Int32 => Kind::Int32,
            field_descriptor_proto::Type::Fixed64 => Kind::Fixed64,
            field_descriptor_proto::Type::Fixed32 => Kind::Fixed32,
            field_descriptor_proto::Type::Bool => Kind::Bool,
            field_descriptor_proto::Type::Uint32 => Kind::Uint32,
            field_descriptor_proto::Type::Sfixed32 => Kind::Sfixed32,
            field_descriptor_proto::Type::Sfixed64 => Kind::Sfixed64,
            field_descriptor_proto::Type::Sint32 => Kind::Sint32,
            field_descriptor_proto::Type::Sint64 => Kind::Sint64,
            field_descriptor_proto::Type::String => Kind::String,
            field_descriptor_proto::Type::Bytes => Kind::Bytes,
            field_descriptor_proto::Type::Enum => {
                Kind::Enum(EnumDescriptor::new(pool.clone(), self))
            }
            field_descriptor_proto::Type::Group | field_descriptor_proto::Type::Message => {
                Kind::Message(MessageDescriptor::new(pool.clone(), self))
            }
        }
    }
}

impl ParentKind {
    fn as_message(&self) -> Option<TypeId> {
        match *self {
            ParentKind::File { .. } => None,
            ParentKind::Message { index } => {
                Some(TypeId(field_descriptor_proto::Type::Message, index))
            }
        }
    }
}

fn get_file_descriptor_proto(pool: &DescriptorPool, index: FileIndex) -> &FileDescriptorProto {
    pool.file_descriptor_protos().nth(index as usize).unwrap()
}

fn find_message_descriptor_proto(
    pool: &DescriptorPool,
    file_index: FileIndex,
    index: MessageIndex,
) -> &DescriptorProto {
    let message = pool.inner.type_map.get_message(index);
    match message.parent {
        ParentKind::File => get_file_descriptor_proto(pool, file_index)
            .message_type
            .iter()
            .find(|ty| ty.name() == parse_name(&message.full_name))
            .expect("message not found"),
        ParentKind::Message {
            index: parent_index,
        } => find_message_descriptor_proto(pool, file_index, parent_index)
            .nested_type
            .iter()
            .find(|ty| ty.name() == parse_name(&message.full_name))
            .expect("message not found"),
    }
}
