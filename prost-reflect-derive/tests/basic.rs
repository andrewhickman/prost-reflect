use prost::Message;
use prost_reflect::ReflectMessage;

#[derive(Message, ReflectMessage)]
#[prost_reflect(
    file_descriptor_path = "file_descriptor_set.bin",
    message_name = "package.MyMessage"
)]
pub struct MyMessage {}

fn main() {
    assert_eq!(MyMessage {}.descriptor().full_name(), "package.MyMessage");
}
