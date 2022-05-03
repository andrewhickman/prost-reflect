use once_cell::sync::Lazy;
use prost::Message;
use prost_reflect::{DescriptorPool, ReflectMessage};

static FILE_DESCRIPTOR: Lazy<DescriptorPool> = Lazy::new(|| {
    DescriptorPool::decode(include_bytes!("file_descriptor_set.bin").as_ref()).unwrap()
});

#[derive(Message, ReflectMessage)]
#[prost_reflect(
    file_descriptor = "FILE_DESCRIPTOR",
    message_name = "package.MyMessage"
)]
pub struct MyMessage {}

fn main() {
    assert_eq!(MyMessage {}.descriptor().full_name(), "package.MyMessage");
}
