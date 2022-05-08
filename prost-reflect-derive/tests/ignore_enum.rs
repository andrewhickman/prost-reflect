use prost_reflect_derive::ReflectMessage;

#[derive(ReflectMessage)]
#[prost_reflect(descriptor_pool = "DESCRIPTOR_POOL", message_name = "msg")]
pub enum MyMessage {}

fn main() {}
