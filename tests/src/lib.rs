#![cfg(test)]

use std::fmt::Debug;

use once_cell::sync::Lazy;
use prost::Message;
use prost_dynamic::{DynamicMessage, FileDescriptor};
use prost_types::FileDescriptorSet;

include!(concat!(env!("OUT_DIR"), "/test.rs"));

pub static TEST_FILE_DESCRIPTOR: Lazy<FileDescriptor> = Lazy::new(|| {
    FileDescriptor::new(
        FileDescriptorSet::decode(
            include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.bin")).as_ref(),
        )
        .unwrap(),
    )
    .unwrap()
});

#[test]
fn roundtrip_scalars() {
    roundtrip(
        &Scalars {
            double: 1.1,
            float: 2.2,
            int32: 3,
            int64: 4,
            uint32: 5,
            uint64: 6,
            sint32: 7,
            sint64: 8,
            fixed32: 9,
            fixed64: 10,
            sfixed32: 11,
            sfixed64: 12,
            r#bool: true,
            string: "5".to_owned(),
            bytes: b"6".to_vec(),
        },
        ".test.Scalars",
    );
}

fn roundtrip<T>(message: &T, message_name: &str)
where
    T: PartialEq + Debug + Message + Default,
{
    let prost_bytes = message.encode_to_vec();

    let mut dynamic_message = DynamicMessage::new(
        TEST_FILE_DESCRIPTOR
            .get_message_by_name(message_name)
            .expect("message not found"),
    );
    dynamic_message.merge(prost_bytes.as_slice()).unwrap();
    let dynamic_bytes = dynamic_message.encode_to_vec();

    let roundtripped_message = T::decode(dynamic_bytes.as_slice()).unwrap();
    assert_eq!(message, &roundtripped_message);
}
