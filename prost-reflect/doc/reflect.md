The [`ReflectMessage`] trait provides a `.descriptor()` method to get type information for a message. By default it is just implemented for `DynamicMessage`.

When the `reflect-well-known-types` feature is enabled, it is implemented for the well-known-types provided by [`prost-types`](https://docs.rs/prost-types/0.10.0/prost_types).

When the `derive` feature is enabled, it can be derived for [`Message`][prost::Message] implementations. The
derive macro takes the following parameters:

| Name            | Value |
|-----------------|-------|
| descriptor_pool | An expression that resolves to a [`DescriptorPool`] containing the message type. The descriptor should be cached to avoid re-building it. |
| message_name    | The full name of the message, used to look it up within [`DescriptorPool`]. |

```rust
use prost::Message;
use prost_reflect::{DescriptorPool, ReflectMessage};
use once_cell::sync::Lazy;

static DESCRIPTOR_POOL: Lazy<DescriptorPool>
    = Lazy::new(|| DescriptorPool::decode(include_bytes!("file_descriptor_set.bin").as_ref()).unwrap());

#[derive(Message, ReflectMessage)]
#[prost_reflect(descriptor_pool = "DESCRIPTOR_POOL", message_name = "package.MyMessage")]
pub struct MyMessage {}

let message = MyMessage {};
assert_eq!(message.descriptor().full_name(), "package.MyMessage");
```

If you are using `prost-build`, the [`prost-reflect-build`] crate provides helpers to generate `ReflectMessage` implementations:

```rust,no_run
prost_reflect_build::Builder::new()
    .compile_protos(&["src/package.proto"], &["src"])
    .unwrap();
```