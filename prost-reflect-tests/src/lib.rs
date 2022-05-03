use once_cell::sync::Lazy;
use prost_reflect::DescriptorPool;

#[cfg(test)]
mod arbitrary;
#[cfg(test)]
mod decode;
#[cfg(test)]
mod desc;
#[cfg(test)]
mod json;

include!(concat!(env!("OUT_DIR"), "/test.rs"));
include!(concat!(env!("OUT_DIR"), "/test2.rs"));

const FILE_DESCRIPTOR_SET_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.bin"));

static TEST_FILE_DESCRIPTOR: Lazy<DescriptorPool> =
    Lazy::new(|| DescriptorPool::decode(FILE_DESCRIPTOR_SET_BYTES).unwrap());

pub fn test_file_descriptor() -> DescriptorPool {
    TEST_FILE_DESCRIPTOR.clone()
}
