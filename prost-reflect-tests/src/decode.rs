use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    iter::FromIterator,
};

use proptest::{prelude::*, test_runner::TestCaseError};
use prost::{bytes::Bytes, encoding::WireType, Message};
use prost_reflect::{DynamicMessage, MapKey, ReflectMessage, Value};
use prost_types::FileDescriptorSet;

use crate::{
    proto::{
        contains_group, message_with_oneof, ComplexType, ContainsGroup, MessageWithOneof,
        ScalarArrays, Scalars, WellKnownTypes,
    },
    test_file_descriptor,
};

#[test]
fn clear_message() {
    let mut dynamic = Scalars {
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
    }
    .transcode_to_dynamic();

    dynamic.clear();

    assert!(!dynamic.has_field_by_name("double"));
    assert!(!dynamic.has_field_by_name("float"));
    assert!(!dynamic.has_field_by_name("int32"));
    assert!(!dynamic.has_field_by_name("int64"));
    assert!(!dynamic.has_field_by_name("uint32"));
    assert!(!dynamic.has_field_by_name("uint64"));
    assert!(!dynamic.has_field_by_name("sint32"));
    assert!(!dynamic.has_field_by_name("sint64"));
    assert!(!dynamic.has_field_by_name("fixed32"));
    assert!(!dynamic.has_field_by_name("fixed64"));
    assert!(!dynamic.has_field_by_name("sfixed32"));
    assert!(!dynamic.has_field_by_name("sfixed64"));
    assert!(!dynamic.has_field_by_name("bool"));
    assert!(!dynamic.has_field_by_name("string"));
    assert!(!dynamic.has_field_by_name("bytes"));

    let encoded_bytes = dynamic.encode_to_vec();
    assert!(encoded_bytes.is_empty());
}

#[test]
#[should_panic(expected = "InvalidType")]
fn set_field_validates_type() {
    let mut dynamic = {
        let message = &Scalars::default();
        message.transcode_to_dynamic()
    };

    dynamic.set_field_by_name("double", Value::U32(5));
}

#[test]
fn try_set_field_validates_type() {
    let mut dynamic = {
        let message = &Scalars::default();
        message.transcode_to_dynamic()
    };

    assert_eq!(
        dynamic
            .try_set_field_by_name("double", Value::U32(5))
            .unwrap_err()
            .to_string(),
        "expected a value of type 'double', but found '5'"
    );
}

#[test]
fn decode_scalars() {
    let dynamic = Scalars {
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
    }
    .transcode_to_dynamic();

    assert_eq!(
        dynamic.get_field_by_name("double").unwrap().as_f64(),
        Some(1.1)
    );
    assert_eq!(
        dynamic.get_field_by_name("float").unwrap().as_f32(),
        Some(2.2)
    );
    assert_eq!(
        dynamic.get_field_by_name("int32").unwrap().as_i32(),
        Some(3)
    );
    assert_eq!(
        dynamic.get_field_by_name("int64").unwrap().as_i64(),
        Some(4)
    );
    assert_eq!(
        dynamic.get_field_by_name("uint32").unwrap().as_u32(),
        Some(5)
    );
    assert_eq!(
        dynamic.get_field_by_name("uint64").unwrap().as_u64(),
        Some(6)
    );
    assert_eq!(
        dynamic.get_field_by_name("sint32").unwrap().as_i32(),
        Some(7)
    );
    assert_eq!(
        dynamic.get_field_by_name("sint64").unwrap().as_i64(),
        Some(8)
    );
    assert_eq!(
        dynamic.get_field_by_name("fixed32").unwrap().as_u32(),
        Some(9)
    );
    assert_eq!(
        dynamic.get_field_by_name("fixed64").unwrap().as_u64(),
        Some(10)
    );
    assert_eq!(
        dynamic.get_field_by_name("sfixed32").unwrap().as_i32(),
        Some(11)
    );
    assert_eq!(
        dynamic.get_field_by_name("sfixed64").unwrap().as_i64(),
        Some(12)
    );
    assert_eq!(
        dynamic.get_field_by_name("bool").unwrap().as_bool(),
        Some(true)
    );
    assert_eq!(
        dynamic.get_field_by_name("string").unwrap().as_str(),
        Some("5")
    );
    assert_eq!(
        dynamic.get_field_by_name("bytes").unwrap().as_bytes(),
        Some(&Bytes::from_static(b"6"))
    );
}

#[test]
fn decode_scalar_arrays() {
    let dynamic = ScalarArrays {
        double: vec![1.1, 2.2],
        float: vec![3.3f32, 4.4f32],
        int32: vec![5, -6],
        int64: vec![7, -8],
        uint32: vec![9, 10],
        uint64: vec![11, 12],
        sint32: vec![13, -14],
        sint64: vec![15, -16],
        fixed32: vec![17, 18],
        fixed64: vec![19, 20],
        sfixed32: vec![21, -22],
        sfixed64: vec![23, -24],
        r#bool: vec![true, false],
        string: vec!["25".to_owned(), "26".to_owned()],
        bytes: vec![b"27".to_vec(), b"28".to_vec()],
    }
    .transcode_to_dynamic();

    assert_eq!(
        dynamic.get_field_by_name("double").unwrap().as_list(),
        Some([Value::F64(1.1), Value::F64(2.2)].as_ref())
    );
    assert_eq!(
        dynamic.get_field_by_name("float").unwrap().as_list(),
        Some([Value::F32(3.3f32), Value::F32(4.4f32)].as_ref())
    );
    assert_eq!(
        dynamic.get_field_by_name("int32").unwrap().as_list(),
        Some([Value::I32(5), Value::I32(-6)].as_ref()),
    );
    assert_eq!(
        dynamic.get_field_by_name("int64").unwrap().as_list(),
        Some([Value::I64(7), Value::I64(-8)].as_ref()),
    );
    assert_eq!(
        dynamic.get_field_by_name("uint32").unwrap().as_list(),
        Some([Value::U32(9), Value::U32(10)].as_ref()),
    );
    assert_eq!(
        dynamic.get_field_by_name("uint64").unwrap().as_list(),
        Some([Value::U64(11), Value::U64(12)].as_ref()),
    );
    assert_eq!(
        dynamic.get_field_by_name("sint32").unwrap().as_list(),
        Some([Value::I32(13), Value::I32(-14)].as_ref()),
    );
    assert_eq!(
        dynamic.get_field_by_name("sint64").unwrap().as_list(),
        Some([Value::I64(15), Value::I64(-16)].as_ref()),
    );
    assert_eq!(
        dynamic.get_field_by_name("fixed32").unwrap().as_list(),
        Some([Value::U32(17), Value::U32(18)].as_ref()),
    );
    assert_eq!(
        dynamic.get_field_by_name("fixed64").unwrap().as_list(),
        Some([Value::U64(19), Value::U64(20)].as_ref()),
    );
    assert_eq!(
        dynamic.get_field_by_name("sfixed32").unwrap().as_list(),
        Some([Value::I32(21), Value::I32(-22)].as_ref()),
    );
    assert_eq!(
        dynamic.get_field_by_name("sfixed64").unwrap().as_list(),
        Some([Value::I64(23), Value::I64(-24)].as_ref()),
    );
    assert_eq!(
        dynamic.get_field_by_name("bool").unwrap().as_list(),
        Some([Value::Bool(true), Value::Bool(false)].as_ref()),
    );
    assert_eq!(
        dynamic.get_field_by_name("string").unwrap().as_list(),
        Some(
            [
                Value::String("25".to_owned()),
                Value::String("26".to_owned())
            ]
            .as_ref()
        ),
    );
    assert_eq!(
        dynamic.get_field_by_name("bytes").unwrap().as_list(),
        Some(
            [
                Value::Bytes(Bytes::from_static(b"27")),
                Value::Bytes(Bytes::from_static(b"28"))
            ]
            .as_ref()
        ),
    );
}

#[test]
fn decode_complex_type() {
    let dynamic = ComplexType {
        string_map: HashMap::from_iter([
            (
                "1".to_owned(),
                Scalars {
                    double: 1.1,
                    float: 2.2,
                    int32: 3,
                    ..Default::default()
                },
            ),
            (
                "2".to_owned(),
                Scalars {
                    int64: 4,
                    uint32: 5,
                    uint64: 6,
                    ..Default::default()
                },
            ),
        ]),
        int_map: HashMap::from_iter([
            (
                3,
                Scalars {
                    sint32: 7,
                    sint64: 8,
                    fixed32: 9,
                    ..Default::default()
                },
            ),
            (
                4,
                Scalars {
                    sint64: 8,
                    fixed32: 9,
                    fixed64: 10,
                    ..Default::default()
                },
            ),
        ]),
        nested: Some(Scalars {
            sfixed32: 11,
            sfixed64: 12,
            r#bool: true,
            string: "5".to_owned(),
            bytes: b"6".to_vec(),
            ..Default::default()
        }),
        my_enum: vec![0, 1, 2, 3, -4],
        optional_enum: 1,
        enum_map: HashMap::from_iter([(1, 1), (2, 2)]),
    }
    .transcode_to_dynamic();

    fn empty_scalars() -> DynamicMessage {
        DynamicMessage::new(
            test_file_descriptor()
                .get_message_by_name(".test.Scalars")
                .unwrap(),
        )
    }

    assert_eq!(
        dynamic.get_field_by_name("string_map").unwrap().as_map(),
        Some(&HashMap::from_iter([
            (MapKey::String("1".to_owned()), {
                let mut msg = empty_scalars();
                msg.set_field_by_name("double", Value::F64(1.1));
                msg.set_field_by_name("float", Value::F32(2.2));
                msg.set_field_by_name("int32", Value::I32(3));
                Value::Message(msg)
            }),
            (MapKey::String("2".to_owned()), {
                let mut msg = empty_scalars();
                msg.set_field_by_name("int64", Value::I64(4));
                msg.set_field_by_name("uint32", Value::U32(5));
                msg.set_field_by_name("uint64", Value::U64(6));
                Value::Message(msg)
            })
        ])),
    );
    assert_eq!(
        dynamic.get_field_by_name("int_map").unwrap().as_map(),
        Some(&HashMap::from_iter([
            (MapKey::I32(3), {
                let mut msg = empty_scalars();
                msg.set_field_by_name("sint32", Value::I32(7));
                msg.set_field_by_name("sint64", Value::I64(8));
                msg.set_field_by_name("fixed32", Value::U32(9));
                Value::Message(msg)
            }),
            (MapKey::I32(4), {
                let mut msg = empty_scalars();
                msg.set_field_by_name("sint64", Value::I64(8));
                msg.set_field_by_name("fixed32", Value::U32(9));
                msg.set_field_by_name("fixed64", Value::U64(10));
                Value::Message(msg)
            })
        ])),
    );
    assert_eq!(
        dynamic.get_field_by_name("nested").unwrap().as_message(),
        Some(&{
            let mut msg = empty_scalars();
            msg.set_field_by_name("sfixed32", Value::I32(11));
            msg.set_field_by_name("sfixed64", Value::I64(12));
            msg.set_field_by_name("bool", Value::Bool(true));
            msg.set_field_by_name("string", Value::String("5".to_owned()));
            msg.set_field_by_name("bytes", Value::Bytes(Bytes::from_static(b"6")));
            msg
        })
    );
    assert_eq!(
        dynamic.get_field_by_name("my_enum").unwrap().as_list(),
        Some(
            [
                Value::EnumNumber(0),
                Value::EnumNumber(1),
                Value::EnumNumber(2),
                Value::EnumNumber(3),
                Value::EnumNumber(-4),
            ]
            .as_ref()
        ),
    );
    assert_eq!(
        dynamic
            .get_field_by_name("optional_enum")
            .unwrap()
            .as_enum_number(),
        Some(1),
    );
    assert_eq!(
        dynamic.get_field_by_name("enum_map").unwrap().as_map(),
        Some(&HashMap::from_iter([
            (MapKey::I32(1), Value::EnumNumber(1)),
            (MapKey::I32(2), Value::EnumNumber(2)),
        ])),
    );
}

#[test]
fn decode_default_values() {
    let dynamic = DynamicMessage::new(
        test_file_descriptor()
            .get_message_by_name(".test2.DefaultValues")
            .unwrap(),
    );

    assert_eq!(
        dynamic.get_field_by_name("double").unwrap().as_f64(),
        Some(1.1)
    );
    assert_eq!(
        dynamic.get_field_by_name("float").unwrap().as_f32(),
        Some(2.2)
    );
    assert_eq!(
        dynamic.get_field_by_name("int32").unwrap().as_i32(),
        Some(-3)
    );
    assert_eq!(
        dynamic.get_field_by_name("int64").unwrap().as_i64(),
        Some(4)
    );
    assert_eq!(
        dynamic.get_field_by_name("uint32").unwrap().as_u32(),
        Some(5)
    );
    assert_eq!(
        dynamic.get_field_by_name("uint64").unwrap().as_u64(),
        Some(6)
    );
    assert_eq!(
        dynamic.get_field_by_name("sint32").unwrap().as_i32(),
        Some(-7)
    );
    assert_eq!(
        dynamic.get_field_by_name("sint64").unwrap().as_i64(),
        Some(8)
    );
    assert_eq!(
        dynamic.get_field_by_name("fixed32").unwrap().as_u32(),
        Some(9)
    );
    assert_eq!(
        dynamic.get_field_by_name("fixed64").unwrap().as_u64(),
        Some(10)
    );
    assert_eq!(
        dynamic.get_field_by_name("sfixed32").unwrap().as_i32(),
        Some(-11)
    );
    assert_eq!(
        dynamic.get_field_by_name("sfixed64").unwrap().as_i64(),
        Some(12)
    );
    assert_eq!(
        dynamic.get_field_by_name("bool").unwrap().as_bool(),
        Some(true)
    );
    assert_eq!(
        dynamic.get_field_by_name("string").unwrap().as_str(),
        Some("hello")
    );
    assert_eq!(
        dynamic.get_field_by_name("bytes").unwrap().as_bytes(),
        Some(&Bytes::from_static(
            b"\0\x01\x07\x08\x0C\n\r\t\x0B\\\'\"\xFE"
        ))
    );
    assert_eq!(
        dynamic
            .get_field_by_name("defaulted_enum")
            .unwrap()
            .as_enum_number(),
        Some(3)
    );
    assert_eq!(
        dynamic.get_field_by_name("enum").unwrap().as_enum_number(),
        Some(2)
    );
}

#[test]
fn set_oneof() {
    let mut dynamic = DynamicMessage::new(
        test_file_descriptor()
            .get_message_by_name(".test.MessageWithOneof")
            .unwrap(),
    );

    assert_eq!(
        dynamic.descriptor().oneofs().next().unwrap().name(),
        "test_oneof"
    );

    assert!(!dynamic.has_field_by_name("oneof_field_1"));
    assert!(!dynamic.has_field_by_name("oneof_field_2"));

    dynamic.set_field_by_name("oneof_field_1", Value::String("hello".to_owned()));
    assert!(dynamic.has_field_by_name("oneof_field_1"));
    assert!(!dynamic.has_field_by_name("oneof_field_2"));

    dynamic.set_field_by_name("oneof_field_2", Value::I32(5));
    assert!(dynamic.has_field_by_name("oneof_field_2"));
    assert!(!dynamic.has_field_by_name("oneof_field_1"));
}

#[test]
fn set_oneof_to_default() {
    let mut dynamic = DynamicMessage::new(
        test_file_descriptor()
            .get_message_by_name(".test.MessageWithOneof")
            .unwrap(),
    );

    assert_eq!(
        dynamic.descriptor().oneofs().next().unwrap().name(),
        "test_oneof"
    );

    assert!(!dynamic.has_field_by_name("oneof_field_1"));
    assert!(!dynamic.has_field_by_name("oneof_field_2"));

    dynamic.set_field_by_name("oneof_field_1", Value::String("".to_owned()));
    assert!(dynamic.has_field_by_name("oneof_field_1"));
    assert!(!dynamic.has_field_by_name("oneof_field_2"));

    dynamic.set_field_by_name("oneof_field_2", Value::I32(0));
    assert!(dynamic.has_field_by_name("oneof_field_2"));
    assert!(!dynamic.has_field_by_name("oneof_field_1"));
}

#[test]
fn roundtrip_scalars() {
    roundtrip(&Scalars {
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
    })
    .unwrap();
}

#[test]
fn roundtrip_scalar_arrays() {
    roundtrip(&ScalarArrays {
        double: vec![1.1, 2.2],
        float: vec![3.3f32, 4.4f32],
        int32: vec![5, -6],
        int64: vec![7, -8],
        uint32: vec![9, 10],
        uint64: vec![11, 12],
        sint32: vec![13, -14],
        sint64: vec![15, -16],
        fixed32: vec![17, 18],
        fixed64: vec![19, 20],
        sfixed32: vec![21, -22],
        sfixed64: vec![23, 24],
        r#bool: vec![true, false],
        string: vec!["25".to_owned(), "26".to_owned()],
        bytes: vec![b"27".to_vec(), b"28".to_vec()],
    })
    .unwrap();
}

#[test]
fn roundtrip_complex_type() {
    roundtrip(&ComplexType {
        string_map: HashMap::from_iter([
            (
                "1".to_owned(),
                Scalars {
                    double: 1.1,
                    float: 2.2,
                    int32: 3,
                    ..Default::default()
                },
            ),
            (
                "2".to_owned(),
                Scalars {
                    int64: 4,
                    uint32: 5,
                    uint64: 6,
                    ..Default::default()
                },
            ),
        ]),
        int_map: HashMap::from_iter([
            (
                3,
                Scalars {
                    sint32: 7,
                    sint64: 8,
                    fixed32: 9,
                    ..Default::default()
                },
            ),
            (
                4,
                Scalars {
                    sint64: 8,
                    fixed32: 9,
                    fixed64: 10,
                    ..Default::default()
                },
            ),
        ]),
        nested: Some(Scalars {
            sfixed32: 11,
            sfixed64: 12,
            r#bool: true,
            string: "5".to_owned(),
            bytes: b"6".to_vec(),
            ..Default::default()
        }),
        my_enum: vec![0, 1, 2, 3, -4],
        optional_enum: 1,
        enum_map: HashMap::from_iter([(1, 1), (2, 2)]),
    })
    .unwrap();
}

#[test]
fn roundtrip_well_known_types() {
    roundtrip(&WellKnownTypes {
        timestamp: Some(prost_types::Timestamp {
            seconds: 63_108_020,
            nanos: 21_000_000,
        }),
        duration: Some(prost_types::Duration {
            seconds: 1,
            nanos: 340_012,
        }),
        r#struct: Some(prost_types::Struct {
            fields: BTreeMap::from_iter([
                (
                    "number".to_owned(),
                    prost_types::Value {
                        kind: Some(prost_types::value::Kind::NumberValue(42.)),
                    },
                ),
                (
                    "null".to_owned(),
                    prost_types::Value {
                        kind: Some(prost_types::value::Kind::NullValue(0)),
                    },
                ),
            ]),
        }),
        float: Some(42.1),
        double: Some(12.4),
        int32: Some(1),
        int64: Some(-2),
        uint32: Some(3),
        uint64: Some(4),
        bool: Some(false),
        string: Some("hello".to_owned()),
        bytes: Some(b"hello".to_vec()),
        mask: Some(prost_types::FieldMask {
            paths: vec!["field_one".to_owned(), "field_two.b.d".to_owned()],
        }),
        list: Some(prost_types::ListValue {
            values: vec![
                prost_types::Value {
                    kind: Some(prost_types::value::Kind::StringValue("foo".to_owned())),
                },
                prost_types::Value {
                    kind: Some(prost_types::value::Kind::BoolValue(false)),
                },
            ],
        }),
        null: 0,
        empty: Some(()),
    })
    .unwrap();
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        .. ProptestConfig::default()
    })]

    #[test]
    fn roundtrip_arb_scalars(message: Scalars) {
        roundtrip(&message)?;
    }

    #[test]
    fn roundtrip_arb_scalar_arrays(message: ScalarArrays) {
        roundtrip(&message)?;
    }

    #[test]
    fn roundtrip_arb_complex_type(message: ComplexType) {
        roundtrip(&message)?;
    }

    #[test]
    fn roundtrip_arb_well_known_types(message: WellKnownTypes) {
        roundtrip(&message)?;
    }
}

#[test]
fn unpacked_fields_accept_packed_bytes() {
    let desc = test_file_descriptor()
        .get_message_by_name("test2.UnpackedScalarArray")
        .unwrap();
    assert!(desc.get_field_by_name("unpacked_double").unwrap().is_list());
    assert!(!desc
        .get_field_by_name("unpacked_double")
        .unwrap()
        .is_packed());
    let mut message = DynamicMessage::new(desc);
    message
        .merge(
            [
                0o322, 0o2, b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0o232, 0o231, 0o231, 0o231, 0o231, 0o231,
                0o271, b'?', 0o377, 0o377, 0o377, 0o377, 0o377, 0o377, 0o357, 0o177, 0, 0, 0, 0, 0,
                0, 0o20, 0,
            ]
            .as_ref(),
        )
        .unwrap();

    assert_eq!(
        message
            .get_field_by_name("unpacked_double")
            .unwrap()
            .as_list(),
        Some([
            Value::F64(0.0),
            Value::F64(0.1),
            Value::F64(179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0),
            Value::F64(0.000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000022250738585072014),
        ].as_ref())
    );
}

#[test]
fn unknown_fields_are_roundtripped() {
    const BYTES: &[u8] = b"\x08\x96\x01";

    let desc = test_file_descriptor()
        .get_message_by_name("google.protobuf.Empty")
        .unwrap();
    let mut message = DynamicMessage::new(desc);
    message.merge(BYTES).unwrap();

    assert_eq!(&message.encode_to_vec(), BYTES);

    let unknown_fields = message.unknown_fields().cloned().collect::<Vec<_>>();
    assert_eq!(unknown_fields.len(), 1);
    assert_eq!(unknown_fields[0].number(), 1);
    assert_eq!(unknown_fields[0].wire_type(), WireType::Varint);
    assert_eq!(unknown_fields[0].encoded_len(), 3);
    let mut field_buf = Vec::new();
    unknown_fields[0].encode(&mut field_buf);
    assert_eq!(field_buf, BYTES);

    assert!(message.take_unknown_fields().eq(unknown_fields));
    assert!(message.encode_to_vec().is_empty());
    assert_eq!(message.unknown_fields().count(), 0);
}

#[test]
fn proto3_default_fields_are_not_encoded() {
    let message = ComplexType {
        optional_enum: 0,
        ..Default::default()
    }
    .transcode_to_dynamic();

    assert!(message.encode_to_vec().is_empty());
}

#[test]
fn oneof_set_multiple_values() {
    let mut value = Vec::new();
    MessageWithOneof {
        test_oneof: Some(message_with_oneof::TestOneof::OneofField1(
            "hello".to_owned(),
        )),
    }
    .encode(&mut value)
    .unwrap();
    MessageWithOneof {
        test_oneof: Some(message_with_oneof::TestOneof::OneofField2(5)),
    }
    .encode(&mut value)
    .unwrap();

    let dynamic_message = DynamicMessage::decode(
        test_file_descriptor()
            .get_message_by_name("test.MessageWithOneof")
            .unwrap(),
        value.as_ref(),
    )
    .unwrap();

    assert!(!dynamic_message.has_field_by_name("oneof_field_1"));
    assert!(dynamic_message.has_field_by_name("oneof_field_2"));

    assert_eq!(dynamic_message.encode_to_vec().as_slice(), b"\x10\x05");
}

#[test]
fn roundtrip_extension() {
    let message_desc = test_file_descriptor()
        .get_message_by_name("my.package2.MyMessage")
        .unwrap();

    let extension_desc = message_desc.get_extension(113).unwrap();
    assert_eq!(
        message_desc.get_extension_by_json_name(extension_desc.json_name()),
        Some(extension_desc.clone())
    );

    let mut dynamic_message = DynamicMessage::new(message_desc.clone());
    dynamic_message.set_extension(&extension_desc, Value::F64(42.0));
    let bytes = dynamic_message.encode_to_vec();

    let roundtripped_dynamic_message =
        DynamicMessage::decode(message_desc, bytes.as_ref()).unwrap();
    assert!(roundtripped_dynamic_message.has_extension(&extension_desc));
    assert_eq!(
        roundtripped_dynamic_message
            .get_extension(&extension_desc)
            .as_ref(),
        &Value::F64(42.0)
    );
}

#[test]
fn roundtrip_file_descriptor_set() {
    let file: Vec<_> = test_file_descriptor()
        .file_descriptor_protos()
        .cloned()
        .collect();
    roundtrip(&FileDescriptorSet { file }).unwrap();
}

#[test]
fn roundtrip_group() {
    let message = test_file_descriptor()
        .get_message_by_name("test2.ContainsGroup")
        .unwrap();
    assert!(message
        .get_field_by_name("requiredgroup")
        .unwrap()
        .is_group());
    assert!(message
        .get_field_by_name("optionalgroup")
        .unwrap()
        .is_group());
    assert!(message
        .get_field_by_name("repeatedgroup")
        .unwrap()
        .is_group());
    assert!(message
        .get_field_by_name("repeatedgroup")
        .unwrap()
        .is_list());

    roundtrip(&ContainsGroup {
        requiredgroup: Some(contains_group::RequiredGroup {
            a: "bar".to_string(),
            b: None,
        }),
        optionalgroup: Some(contains_group::OptionalGroup {
            c: "foo".to_string(),
            d: Some(-5),
        }),
        repeatedgroup: vec![
            contains_group::RepeatedGroup {
                ..Default::default()
            },
            contains_group::RepeatedGroup {
                e: "hello".to_string(),
                f: Some(10),
            },
        ],
    })
    .unwrap();
}

fn roundtrip<T>(message: &T) -> Result<(), TestCaseError>
where
    T: PartialEq + Debug + ReflectMessage + Default,
{
    let dynamic_message = message.transcode_to_dynamic();
    let roundtripped_message: T = dynamic_message.transcode_to().unwrap();
    prop_assert_eq!(message, &roundtripped_message);

    // Check roundtripping through unknown fields works
    let mut empty_message = DynamicMessage::new(
        test_file_descriptor()
            .get_message_by_name("google.protobuf.Empty")
            .unwrap(),
    );
    empty_message.transcode_from(message).unwrap();
    let unknown_roundtripped_message: T = empty_message.transcode_to().unwrap();
    prop_assert_eq!(
        message,
        &unknown_roundtripped_message,
        "roundtrip through unknown fields failed"
    );

    // Check that transcoding to a new dynamic message is equivalent to just cloning it.
    let mut duplicate_message = DynamicMessage::new(dynamic_message.descriptor());
    duplicate_message.transcode_from(&dynamic_message).unwrap();
    assert_eq!(dynamic_message, duplicate_message);

    Ok(())
}
