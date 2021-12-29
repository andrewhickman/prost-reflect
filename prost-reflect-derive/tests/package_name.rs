use prost::Message;
use prost_reflect::ReflectMessage;

#[derive(Message, ReflectMessage)]
#[prost_reflect(
    file_descriptor_set_path = "file_descriptor_set.bin",
    package_name = "package"
)]
pub struct MyMessage {}

fn main() {
    assert_eq!(MyMessage {}.descriptor().full_name(), "package.MyMessage");
}
