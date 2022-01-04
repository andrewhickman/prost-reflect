mod build;
mod map;

pub(in crate::descriptor) use self::map::{TypeId, TypeMap};

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

use crate::descriptor::{
    debug_fmt_iter, parse_name, parse_namespace, FileDescriptor, MAP_ENTRY_KEY_NUMBER,
    MAP_ENTRY_VALUE_NUMBER,
};

/// A protobuf message definition.
#[derive(Clone, PartialEq, Eq)]
pub struct MessageDescriptor {
    file_set: FileDescriptor,
    ty: TypeId,
}

struct MessageDescriptorInner {
    full_name: String,
    parent: Option<TypeId>,
    is_map_entry: bool,
    fields: BTreeMap<u32, FieldDescriptorInner>,
    field_names: HashMap<String, u32>,
    field_json_names: HashMap<String, u32>,
    oneof_decls: Box<[OneofDescriptorInner]>,
}

/// A oneof field in a protobuf message.
#[derive(Clone, PartialEq, Eq)]
pub struct OneofDescriptor {
    message: MessageDescriptor,
    index: usize,
}

struct OneofDescriptorInner {
    name: String,
    full_name: String,
    fields: Vec<u32>,
}

/// A protobuf message definition.
#[derive(Clone, PartialEq, Eq)]
pub struct FieldDescriptor {
    message: MessageDescriptor,
    field: u32,
}

struct FieldDescriptorInner {
    name: String,
    full_name: String,
    json_name: String,
    is_group: bool,
    cardinality: Cardinality,
    is_packed: bool,
    supports_presence: bool,
    default_value: Option<crate::Value>,
    oneof_index: Option<usize>,
    ty: TypeId,
}

/// A protobuf enum type.
#[derive(Clone, PartialEq, Eq)]
pub struct EnumDescriptor {
    file_set: FileDescriptor,
    ty: TypeId,
}

struct EnumDescriptorInner {
    full_name: String,
    parent: Option<TypeId>,
    value_names: HashMap<String, i32>,
    values: BTreeMap<i32, EnumValueDescriptorInner>,
    default_value: i32,
}

/// A value in a protobuf enum type.
#[derive(Clone, PartialEq, Eq)]
pub struct EnumValueDescriptor {
    parent: EnumDescriptor,
    number: i32,
}

struct EnumValueDescriptorInner {
    name: String,
    full_name: String,
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

enum Type<'a> {
    Message(&'a MessageDescriptorInner),
    Enum(&'a EnumDescriptorInner),
    Scalar(Scalar),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Scalar {
    Double = 0,
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
}

impl MessageDescriptor {
    pub(in crate::descriptor) fn new(file_set: FileDescriptor, ty: TypeId) -> Self {
        MessageDescriptor { file_set, ty }
    }

    pub(in crate::descriptor) fn iter(
        file_set: &FileDescriptor,
    ) -> impl ExactSizeIterator<Item = Self> + '_ {
        file_set
            .inner
            .type_map
            .messages()
            .map(move |ty| MessageDescriptor {
                file_set: file_set.clone(),
                ty,
            })
    }

    pub(in crate::descriptor) fn try_get_by_name(
        file_set: &FileDescriptor,
        name: &str,
    ) -> Option<Self> {
        let ty = file_set.inner.type_map.try_get_by_name(name)?;
        if !file_set.inner.type_map.get(ty).is_message() {
            return None;
        }
        Some(MessageDescriptor {
            file_set: file_set.clone(),
            ty,
        })
    }

    /// Gets a reference to the [`FileDescriptor`] this message is defined in.
    pub fn parent_file(&self) -> &FileDescriptor {
        &self.file_set
    }

    /// Gets the parent message type if this message type is nested inside a another message, or `None` otherwise
    pub fn parent_message(&self) -> Option<MessageDescriptor> {
        self.message_ty().parent.map(|ty| MessageDescriptor {
            file_set: self.file_set.clone(),
            ty,
        })
    }

    /// Gets the short name of the message type, e.g. `MyMessage`.
    pub fn name(&self) -> &str {
        parse_name(self.full_name())
    }

    /// Gets the full name of the message type, e.g. `my.package.MyMessage`.
    pub fn full_name(&self) -> &str {
        &self.message_ty().full_name
    }

    /// Gets the name of the package this message type is defined in, e.g. `my.package`.
    ///
    /// If no package name is set, an empty string is returned.
    pub fn package_name(&self) -> &str {
        parse_namespace(&self.root_message_ty().full_name)
    }

    /// Gets an iterator yielding a [`FieldDescriptor`] for each field defined in this message.
    pub fn fields(&self) -> impl ExactSizeIterator<Item = FieldDescriptor> + '_ {
        self.message_ty()
            .fields
            .keys()
            .map(move |&field| FieldDescriptor {
                message: self.clone(),
                field,
            })
    }

    /// Gets an iterator yielding a [`OneofDescriptor`] for each oneof field defined in this message.
    pub fn oneofs(&self) -> impl ExactSizeIterator<Item = OneofDescriptor> + '_ {
        (0..self.message_ty().oneof_decls.len()).map(move |index| OneofDescriptor {
            message: self.clone(),
            index,
        })
    }

    /// Gets a [`FieldDescriptor`] with the given number, or `None` if no such field exists.
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

    /// Gets a [`FieldDescriptor`] with the given name, or `None` if no such field exists.
    pub fn get_field_by_name(&self, name: &str) -> Option<FieldDescriptor> {
        self.message_ty()
            .field_names
            .get(name)
            .map(|&number| FieldDescriptor {
                message: self.clone(),
                field: number,
            })
    }

    /// Gets a [`FieldDescriptor`] with the given JSON name, or `None` if no such field exists.
    pub fn get_field_by_json_name(&self, json_name: &str) -> Option<FieldDescriptor> {
        self.message_ty()
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
        self.message_ty().is_map_entry
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

    fn message_ty(&self) -> &MessageDescriptorInner {
        self.file_set.inner.type_map.get(self.ty).unwrap_message()
    }

    fn root_message_ty(&self) -> &MessageDescriptorInner {
        let mut curr = self.message_ty();
        while let Some(parent) = curr.parent {
            curr = self.file_set.inner.type_map.get(parent).unwrap_message()
        }
        curr
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
    /// Gets a reference to the [`FileDescriptor`] this field is defined in.
    pub fn parent_file(&self) -> &FileDescriptor {
        self.message.parent_file()
    }

    /// Gets a reference to the [`MessageDescriptor`] this field is defined in.
    pub fn parent_message(&self) -> &MessageDescriptor {
        &self.message
    }

    /// Gets the short name of the message type, e.g. `my_field`.
    pub fn name(&self) -> &str {
        &self.message_field_ty().name
    }

    /// Gets the full name of the message field, e.g. `my.package.MyMessage.my_field`.
    pub fn full_name(&self) -> &str {
        &self.message_field_ty().full_name
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
        &self.message_field_ty().json_name
    }

    /// Whether this field is encoded using the proto2 group encoding.
    pub fn is_group(&self) -> bool {
        self.message_field_ty().is_group
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
        self.message_field_ty().is_packed
    }

    /// The cardinality of this field.
    pub fn cardinality(&self) -> Cardinality {
        self.message_field_ty().cardinality
    }

    /// Whether this field supports distinguishing between an unpopulated field and
    /// the default value.
    ///
    /// For proto2 messages this returns `true` for all non-repeated fields.
    /// For proto3 this returns `true` for message fields, and fields contained
    /// in a `oneof`.
    pub fn supports_presence(&self) -> bool {
        self.message_field_ty().supports_presence
    }

    /// Gets the [`Kind`] of this field.
    pub fn kind(&self) -> Kind {
        let ty = self.message_field_ty().ty;
        match self.message.file_set.inner.type_map.get(ty) {
            Type::Message(_) => Kind::Message(MessageDescriptor {
                file_set: self.message.file_set.clone(),
                ty,
            }),
            Type::Enum(_) => Kind::Enum(EnumDescriptor {
                file_set: self.message.file_set.clone(),
                ty,
            }),
            Type::Scalar(scalar) => match scalar {
                Scalar::Double => Kind::Double,
                Scalar::Float => Kind::Float,
                Scalar::Int32 => Kind::Int32,
                Scalar::Int64 => Kind::Int64,
                Scalar::Uint32 => Kind::Uint32,
                Scalar::Uint64 => Kind::Uint64,
                Scalar::Sint32 => Kind::Sint32,
                Scalar::Sint64 => Kind::Sint64,
                Scalar::Fixed32 => Kind::Fixed32,
                Scalar::Fixed64 => Kind::Fixed64,
                Scalar::Sfixed32 => Kind::Sfixed32,
                Scalar::Sfixed64 => Kind::Sfixed64,
                Scalar::Bool => Kind::Bool,
                Scalar::String => Kind::String,
                Scalar::Bytes => Kind::Bytes,
            },
        }
    }

    /// Gets a [`OneofDescriptor`] representing the oneof containing this field,
    /// or `None` if this field is not contained in a oneof.
    pub fn containing_oneof(&self) -> Option<OneofDescriptor> {
        self.message_field_ty()
            .oneof_index
            .map(|index| OneofDescriptor {
                message: self.message.clone(),
                index,
            })
    }

    pub(crate) fn default_value(&self) -> Option<&crate::Value> {
        self.message_field_ty().default_value.as_ref()
    }

    pub(crate) fn is_packable(&self) -> bool {
        let ty = self.message_field_ty().ty;
        self.message.file_set.inner.type_map.get(ty).is_packable()
    }

    fn message_field_ty(&self) -> &FieldDescriptorInner {
        &self.message.message_ty().fields[&self.field]
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
    pub(in crate::descriptor) fn iter(
        file_set: &FileDescriptor,
    ) -> impl ExactSizeIterator<Item = Self> + '_ {
        file_set
            .inner
            .type_map
            .enums()
            .map(move |ty| EnumDescriptor {
                file_set: file_set.clone(),
                ty,
            })
    }

    pub(in crate::descriptor) fn try_get_by_name(
        file_set: &FileDescriptor,
        name: &str,
    ) -> Option<Self> {
        let ty = file_set.inner.type_map.try_get_by_name(name)?;
        if !file_set.inner.type_map.get(ty).is_enum() {
            return None;
        }
        Some(EnumDescriptor {
            file_set: file_set.clone(),
            ty,
        })
    }

    /// Gets a reference to the [`FileDescriptor`] this enum type is defined in.
    pub fn parent_file(&self) -> &FileDescriptor {
        &self.file_set
    }

    /// Gets the parent message type if this enum type is nested inside a another message, or `None` otherwise
    pub fn parent_message(&self) -> Option<MessageDescriptor> {
        self.enum_ty().parent.map(|ty| MessageDescriptor {
            file_set: self.file_set.clone(),
            ty,
        })
    }

    /// Gets the short name of the enum type, e.g. `MyEnum`.
    pub fn name(&self) -> &str {
        parse_name(self.full_name())
    }

    /// Gets the full name of the enum, e.g. `my.package.MyEnum`.
    pub fn full_name(&self) -> &str {
        &self.enum_ty().full_name
    }

    /// Gets the name of the package this enum type is defined in, e.g. `my.package`.
    ///
    /// If no package name is set, an empty string is returned.
    pub fn package_name(&self) -> &str {
        match self.root_message_ty() {
            Some(message) => parse_namespace(&message.full_name),
            None => parse_namespace(self.full_name()),
        }
    }

    /// Gets the default value for the enum type.
    pub fn default_value(&self) -> EnumValueDescriptor {
        EnumValueDescriptor {
            parent: self.clone(),
            number: self.enum_ty().default_value,
        }
    }

    /// Gets a [`EnumValueDescriptor`] for the enum value with the given name, or `None` if no such value exists.
    pub fn get_value_by_name(&self, name: &str) -> Option<EnumValueDescriptor> {
        self.enum_ty()
            .value_names
            .get(name)
            .map(|&number| EnumValueDescriptor {
                parent: self.clone(),
                number,
            })
    }

    /// Gets a [`EnumValueDescriptor`] for the enum value with the given number, or `None` if no such value exists.
    pub fn get_value(&self, number: i32) -> Option<EnumValueDescriptor> {
        self.enum_ty()
            .values
            .get(&number)
            .map(|_| EnumValueDescriptor {
                parent: self.clone(),
                number,
            })
    }

    /// Gets an iterator yielding a [`EnumValueDescriptor`] for each value in this enum.
    pub fn values(&self) -> impl ExactSizeIterator<Item = EnumValueDescriptor> + '_ {
        self.enum_ty()
            .values
            .keys()
            .map(move |&number| EnumValueDescriptor {
                parent: self.clone(),
                number,
            })
    }

    fn enum_ty(&self) -> &EnumDescriptorInner {
        self.file_set.inner.type_map.get(self.ty).unwrap_enum()
    }

    fn root_message_ty(&self) -> Option<&MessageDescriptorInner> {
        match self.enum_ty().parent {
            Some(mut curr) => loop {
                let message = self.file_set.inner.type_map.get(curr).unwrap_message();
                if let Some(parent) = message.parent {
                    curr = parent;
                } else {
                    return Some(message);
                }
            },
            None => None,
        }
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
    /// Gets a reference to the [`FileDescriptor`] this enum value is defined in.
    pub fn parent_file(&self) -> &FileDescriptor {
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

    /// Gets the number representing this enum value.
    pub fn number(&self) -> i32 {
        self.number
    }

    fn enum_value_ty(&self) -> &EnumValueDescriptorInner {
        self.parent.enum_ty().values.get(&self.number).unwrap()
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
    /// Gets a reference to the [`FileDescriptor`] this oneof is defined in.
    pub fn parent_file(&self) -> &FileDescriptor {
        self.message.parent_file()
    }

    /// Gets a reference to the [`MessageDescriptor`] this message is defined in.
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
        &self.message.message_ty().oneof_decls[self.index]
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

impl<'a> Type<'a> {
    pub fn is_message(&self) -> bool {
        matches!(self, Type::Message(_))
    }

    fn unwrap_message(&self) -> &'a MessageDescriptorInner {
        match self {
            Type::Message(message) => message,
            _ => panic!("expected message type"),
        }
    }

    pub fn is_enum(&self) -> bool {
        matches!(self, Type::Enum(_))
    }

    fn unwrap_enum(&self) -> &'a EnumDescriptorInner {
        match self {
            Type::Enum(enum_ty) => enum_ty,
            _ => panic!("expected enum type"),
        }
    }

    fn is_packable(&self) -> bool {
        match self {
            Type::Scalar(scalar) => scalar.is_packable(),
            Type::Enum(_) => true,
            _ => false,
        }
    }
}

impl Scalar {
    fn is_packable(&self) -> bool {
        match self {
            Scalar::Double
            | Scalar::Float
            | Scalar::Int32
            | Scalar::Int64
            | Scalar::Uint32
            | Scalar::Uint64
            | Scalar::Sint32
            | Scalar::Sint64
            | Scalar::Fixed32
            | Scalar::Fixed64
            | Scalar::Sfixed32
            | Scalar::Sfixed64
            | Scalar::Bool => true,
            Scalar::String | Scalar::Bytes => false,
        }
    }
}
