use prost::Message;
use prost_reflect_derive::ReflectMessage;

#[derive(Message, ReflectMessage)]
#[prost_reflect(file_descriptor_path = "basic.rs", message_name = "msg")]
pub struct MyMessage {}

fn main() {}
