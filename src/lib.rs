//! This crate provides support for dynamic protobuf messages. These are useful when the
//! protobuf type definition is not known ahead of time.
//!
//! The main entry points into the API of this crate are:
//! - [`FileDescriptor`] wraps a [`FileDescriptorSet`][prost_types::FileDescriptorSet] output
//!   by the protobuf compiler to provide an API for inspecting type definitions.
//! - [`DynamicMessage`] provides encoding, decoding and reflection of an arbitrary protobuf
//!   message definition described by a [`MessageDescriptor`].

#![warn(missing_debug_implementations, missing_docs)]

#[cfg(feature = "serde1")]
extern crate serde1 as serde;

mod descriptor;
mod dynamic;

pub use self::descriptor::{
    DescriptorError, EnumDescriptor, EnumValueDescriptor, FieldDescriptor, FileDescriptor, Kind,
    MessageDescriptor, MethodDescriptor, OneofDescriptor, ServiceDescriptor,
};
pub use self::dynamic::{DynamicMessage, MapKey, Value};

#[cfg(feature = "serde")]
pub use self::dynamic::{DeserializeOptions, SerializeOptions};
