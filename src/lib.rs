//! This crate provides support for dynamic protobuf messages. These are useful when the
//! protobuf type definition is not known ahead of time.

#![deny(missing_debug_implementations, missing_docs)]
#![allow(dead_code)]

mod descriptor;
mod dynamic;
mod unknown;

pub use self::descriptor::{
    Descriptor, DescriptorError, FileDescriptor, MethodDescriptor, ServiceDescriptor,
};
pub use self::unknown::{UnknownField, UnknownFieldSet};
