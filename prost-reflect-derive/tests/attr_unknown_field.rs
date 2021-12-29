use prost_reflect_derive::ReflectMessage;

#[derive(ReflectMessage)]
#[prost_reflect(foo = 123)]
pub struct MyMessage {}

fn main() {}
