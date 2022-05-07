use prost_reflect_derive::ReflectMessage;

#[derive(ReflectMessage)]
#[prost_reflect(descriptor_pool = "FILE_DESCRIPTOR")]
pub struct MyMessage {}

fn main() {}
