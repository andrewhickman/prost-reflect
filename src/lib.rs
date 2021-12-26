//! This crate provides support for dynamic protobuf messages. These are useful when the
//! protobuf type definition is not known ahead of time.
//!
//! The main entry points into the API of this crate are:
//!   

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
