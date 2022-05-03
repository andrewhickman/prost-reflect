This crate provides support for dynamic protobuf messages. These are useful when the
protobuf type definition is not known ahead of time.

The main entry points into the API of this crate are:

- [`DescriptorPool`] wraps a [`FileDescriptorSet`][prost_types::FileDescriptorSet] output by 
  the protobuf compiler to provide an API for inspecting type definitions.
- [`DynamicMessage`] provides encoding, decoding and reflection of an arbitrary protobuf 
  message definition described by a [`MessageDescriptor`].