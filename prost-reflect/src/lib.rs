//! This crate provides support for dynamic protobuf messages. These are useful when the
//! protobuf type definition is not known ahead of time.
//!
//! The main entry points into the API of this crate are:
//! - [`FileDescriptor`] wraps a [`FileDescriptorSet`][prost_types::FileDescriptorSet] output
//!   by the protobuf compiler to provide an API for inspecting type definitions.
//! - [`DynamicMessage`] provides encoding, decoding and reflection of an arbitrary protobuf
//!   message definition described by a [`MessageDescriptor`].
//!
//! # Example - decoding
//!
//! [`DynamicMessage`] does not implement [`Default`] since it needs a message descriptor to
//! function. To decode a protobuf byte stream into an instance of this type, use [`DynamicMessage::decode`]
//! to create a default value for the [`MessageDescriptor`] instance and merge into it:
//!
//! ```
//! use prost::Message;
//! use prost_types::FileDescriptorSet;
//! use prost_reflect::{DynamicMessage, FileDescriptor, Value};
//!
//! let file_descriptor_set = FileDescriptorSet::decode(include_bytes!("file_descriptor_set.bin").as_ref()).unwrap();
//! let file_descriptor = FileDescriptor::new(file_descriptor_set).unwrap();
//! let message_descriptor = file_descriptor.get_message_by_name("package.MyMessage").unwrap();
//!
//! let dynamic_message = DynamicMessage::decode(message_descriptor, b"\x08\x96\x01".as_ref()).unwrap();
//!
//! assert_eq!(dynamic_message.get_field_by_name("foo").unwrap().as_ref(), &Value::I32(150));
//! ```
#![cfg_attr(
    feature = "serde",
    doc = r##"
# Example - JSON mapping

When the `serde` feature is enabled, `DynamicMessage` can be deserialized to and from the
[canonical JSON mapping](https://developers.google.com/protocol-buffers/docs/proto3#json)
defined for protobuf messages.

```
use prost::Message;
use prost_reflect::{DynamicMessage, FileDescriptor, Value};
use serde_json::de::Deserializer;

let file_descriptor = FileDescriptor::decode(include_bytes!("file_descriptor_set.bin").as_ref()).unwrap();
let message_descriptor = file_descriptor.get_message_by_name("package.MyMessage").unwrap();

let json = r#"{ "foo": 150 }"#;
let mut deserializer = Deserializer::from_str(json);
let dynamic_message = DynamicMessage::deserialize(message_descriptor, &mut deserializer).unwrap();
deserializer.end().unwrap();

assert_eq!(dynamic_message.get_field_by_name("foo").unwrap().as_ref(), &Value::I32(150));
```
"##
)]
#![cfg_attr(
    feature = "derive",
    doc = r##"
# Deriving [`ReflectMessage`]

The [`ReflectMessage`] trait provides a `.descriptor()` method to get type information for a message.

When the `derive` feature is enabled, it can be derived for [`Message`][prost::Message] implementations. The
derive macro takes the following parameters:

| Name            | Value |
|-----------------|-------|
| file_descriptor | An expression that resolves to a [`FileDescriptor`] containing the message type. The descriptor should be cached to avoid re-building it. |
| message_name    | The full name of the message, used to look it up within [`FileDescriptor`]. |

```
use prost::Message;
use prost_reflect::{FileDescriptor, ReflectMessage};
use once_cell::sync::Lazy;

static FILE_DESCRIPTOR: Lazy<FileDescriptor>
    = Lazy::new(|| FileDescriptor::decode(include_bytes!("file_descriptor_set.bin").as_ref()).unwrap());

#[derive(Message, ReflectMessage)]
#[prost_reflect(file_descriptor = "FILE_DESCRIPTOR", message_name = "package.MyMessage")]
pub struct MyMessage {}

let message = MyMessage {};
assert_eq!(message.descriptor().full_name(), "package.MyMessage");
```

If you are using `prost-build`, it can be configured to generate [`ReflectMessage`] implementations
for messages:

```rust,no_run
use prost_build::Config;

Config::new()
    .file_descriptor_set_path("file_descriptor_set.bin")
    .type_attribute(".package.MyMessage", "#[derive(::prost_reflect::ReflectMessage)]")
    .type_attribute(".package.MyMessage", "#[prost_reflect(file_descriptor = \"FILE_DESCRIPTOR\", message_name = \"package.MyMessage\")]")
    .compile_protos(&["src/package.proto"], &["src"])
    .unwrap();
```
"##
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_debug_implementations, missing_docs)]
#![doc(html_root_url = "https://docs.rs/prost-reflect/0.5.3/")]

#[cfg(feature = "serde1")]
extern crate serde1 as serde;

mod descriptor;
mod dynamic;
mod reflect;

pub use self::descriptor::{
    Cardinality, DescriptorError, EnumDescriptor, EnumValueDescriptor, ExtensionDescriptor,
    FieldDescriptor, FileDescriptor, Kind, MessageDescriptor, MethodDescriptor, OneofDescriptor,
    ServiceDescriptor,
};
pub use self::dynamic::{DynamicMessage, MapKey, Value};
pub use self::reflect::ReflectMessage;

#[cfg(feature = "serde")]
pub use self::dynamic::{DeserializeOptions, SerializeOptions};

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use prost_reflect_derive::ReflectMessage;
