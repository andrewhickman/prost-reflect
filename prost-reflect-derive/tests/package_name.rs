use once_cell::sync::Lazy;
use prost::Message;
use prost_reflect::{FileDescriptor, ReflectMessage};

static FILE_DESCRIPTOR: Lazy<FileDescriptor> = Lazy::new(|| {
    FileDescriptor::decode(include_bytes!("file_descriptor_set.bin").as_ref()).unwrap()
});

#[derive(Message, ReflectMessage)]
#[prost_reflect(file_descriptor = "FILE_DESCRIPTOR", package_name = "package")]
pub struct MyMessage {}

fn main() {
    assert_eq!(MyMessage {}.descriptor().full_name(), "package.MyMessage");
}
