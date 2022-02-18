`DynamicMessage` does not implement [`Default`] since it needs a message descriptor to
function. To decode a protobuf byte stream into an instance of this type, use [`DynamicMessage::decode`]
to create a default value for the `MessageDescriptor` instance and merge into it:

```rust
use prost::Message;
use prost_types::FileDescriptorSet;
use prost_reflect::{DynamicMessage, FileDescriptor, Value};

let file_descriptor_set = FileDescriptorSet::decode(include_bytes!("file_descriptor_set.bin").as_ref()).unwrap();
let file_descriptor = FileDescriptor::new(file_descriptor_set).unwrap();
let message_descriptor = file_descriptor.get_message_by_name("package.MyMessage").unwrap();

let dynamic_message = DynamicMessage::decode(message_descriptor, b"\x08\x96\x01".as_ref()).unwrap();

assert_eq!(dynamic_message.get_field_by_name("foo").unwrap().as_ref(), &Value::I32(150));
```