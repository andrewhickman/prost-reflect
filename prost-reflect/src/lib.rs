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
//! function. To decode a protobuf byte stream into an instance of this type, create a default
//! value for the [`MessageDescriptor`] instance and merge into it:
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
//! let mut dynamic_message = DynamicMessage::new(message_descriptor);
//! dynamic_message.merge(b"\x08\x96\x01".as_ref());
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
use prost_types::FileDescriptorSet;
use prost_reflect::{DynamicMessage, FileDescriptor, Value};
use serde_json::de::Deserializer;

let file_descriptor_set = FileDescriptorSet::decode(include_bytes!("file_descriptor_set.bin").as_ref()).unwrap();
let file_descriptor = FileDescriptor::new(file_descriptor_set).unwrap();
let message_descriptor = file_descriptor.get_message_by_name("package.MyMessage").unwrap();

let json = r#"{ "foo": 150 }"#;
let mut deserializer = Deserializer::from_str(json);
let dynamic_message = DynamicMessage::deserialize(message_descriptor, &mut deserializer).unwrap();
deserializer.end().unwrap();

assert_eq!(dynamic_message.get_field_by_name("foo").unwrap().as_ref(), &Value::I32(150));
```
"##
)]
#![warn(missing_debug_implementations, missing_docs)]

#[cfg(feature = "serde1")]
extern crate serde1 as serde;

mod descriptor;
mod dynamic;
mod reflect;

pub use self::descriptor::{
    DescriptorError, EnumDescriptor, EnumValueDescriptor, FieldDescriptor, FileDescriptor, Kind,
    MessageDescriptor, MethodDescriptor, OneofDescriptor, ServiceDescriptor,
};
pub use self::dynamic::{DynamicMessage, MapKey, Value};
pub use self::reflect::ReflectMessage;

#[cfg(feature = "serde")]
pub use self::dynamic::{DeserializeOptions, SerializeOptions};
