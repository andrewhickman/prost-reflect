use prost::Message;
use prost_reflect::ReflectMessage;

const FILE_DESCRIPTOR_SET_BYTES: &'static [u8] = include_bytes!("file_descriptor_set.bin");

#[derive(Message, ReflectMessage)]
#[prost_reflect(file_descriptor_set_bytes = "FILE_DESCRIPTOR_SET_BYTES")]
#[prost_reflect(message_name = "package.MyMessage")]
pub struct MyNestedMessage {}

fn main() {
    assert_eq!(
        MyNestedMessage {}.descriptor().full_name(),
        "package.MyMessage"
    );
}
