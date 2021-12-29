use prost_reflect_derive::ReflectMessage;

#[derive(ReflectMessage)]
#[prost_reflect(file_descriptor = "FILE_DESCRIPTOR", message_name = "msg")]
pub enum MyMessage {}

fn main() {}
