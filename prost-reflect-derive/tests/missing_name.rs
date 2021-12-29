use prost_reflect_derive::ReflectMessage;

#[derive(ReflectMessage)]
#[prost_reflect(file_descriptor = "FILE_DESCRIPTOR")]
pub struct MyMessage {}

fn main() {}
