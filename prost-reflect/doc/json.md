When the `serde` feature is enabled, `DynamicMessage` can be deserialized to and from the
[canonical JSON mapping](https://developers.google.com/protocol-buffers/docs/proto3#json)
defined for protobuf messages.

```rust
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