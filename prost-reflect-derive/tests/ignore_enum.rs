use prost_reflect_derive::ReflectMessage;

#[derive(ReflectMessage)]
#[prost_reflect(descriptor_pool = "FILE_DESCRIPTOR", message_name = "msg")]
pub enum MyMessage {}

fn main() {}
