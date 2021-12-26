//! This crate provides support for dynamic protobuf messages. These are useful when the
//! protobuf type definition is not known ahead of time.

#![warn(missing_debug_implementations, missing_docs)]

mod descriptor;
mod dynamic;

pub use self::descriptor::{
    DescriptorError, EnumDescriptor, EnumValueDescriptor, FieldDescriptor, FileDescriptor, Kind,
    MessageDescriptor, MethodDescriptor, OneofDescriptor, ServiceDescriptor,
};
pub use self::dynamic::{DynamicMessage, MapKey, Value};
