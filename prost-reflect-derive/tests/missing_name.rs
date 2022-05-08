use prost_reflect_derive::ReflectMessage;

#[derive(ReflectMessage)]
#[prost_reflect(descriptor_pool = "DESCRIPTOR_POOL")]
pub struct MyMessage {}

fn main() {}
