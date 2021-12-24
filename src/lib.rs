//! This crate provides support for dynamic protobuf messages. These are useful when the
//! protobuf type definition is not known ahead of time.

// #![deny(missing_debug_implementations, missing_docs)]

mod descriptor;
mod dynamic;

pub use self::descriptor::{
    Descriptor, DescriptorError, FieldDescriptor, FileDescriptor, MethodDescriptor,
    ServiceDescriptor,
};
pub use self::dynamic::{DynamicMessage, DynamicValue, MapKey};
