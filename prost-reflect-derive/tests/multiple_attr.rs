use once_cell::sync::Lazy;
use prost::Message;
use prost_reflect::{DescriptorPool, ReflectMessage};

static FILE_DESCRIPTOR: Lazy<DescriptorPool> = Lazy::new(|| {
    DescriptorPool::decode(include_bytes!("file_descriptor_set.bin").as_ref()).unwrap()
});

#[derive(Message, ReflectMessage)]
#[prost_reflect(descriptor_pool = "FILE_DESCRIPTOR")]
#[prost_reflect(message_name = "package.MyMessage")]
pub struct MyNestedMessage {}

fn main() {
    assert_eq!(
        MyNestedMessage {}.descriptor().full_name(),
        "package.MyMessage"
    );
}
