use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    iter::FromIterator,
};

use proptest::{prelude::*, test_runner::TestCaseError};
use prost::Message;
use prost_reflect::{DeserializeOptions, DynamicMessage, ReflectMessage, SerializeOptions};
use serde_json::json;

use crate::{
    arbitrary, message_with_oneof, test_file_descriptor, ComplexType, MessageWithAliasedEnum,
    MessageWithOneof, Point, ScalarArrays, Scalars, WellKnownTypes,
};

#[test]
fn serialize_scalars() {
    let value = to_json(&Scalars {
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
        bytes: b"i\xa6\xbem\xb6\xffX".to_vec(),
    });

    assert_eq!(
        value,
        json!({
            "double": 1.1,
            "float": 2.2f32,
            "int32": 3,
            "int64": "4",
            "uint32": 5,
            "uint64": "6",
            "sint32": 7,
            "sint64": "8",
            "fixed32": 9,
            "fixed64": "10",
            "sfixed32": 11,
            "sfixed64": "12",
            "bool": true,
            "string": "5",
            "bytes": "aaa+bbb/WA==",
        })
    );
}

#[test]
fn serialize_scalars_float_extrema() {
    let inf = to_json(&Scalars {
        float: f32::INFINITY,
        double: f64::INFINITY,
        ..Default::default()
    });
    let neg_inf = to_json(&Scalars {
        float: f32::NEG_INFINITY,
        double: f64::NEG_INFINITY,
        ..Default::default()
    });
    let nan = to_json(&Scalars {
        float: f32::NAN,
        double: f64::NAN,
        ..Default::default()
    });

    assert_eq!(
        inf,
        json!({
            "double": "Infinity",
            "float": "Infinity",
        })
    );
    assert_eq!(
        neg_inf,
        json!({
            "double": "-Infinity",
            "float": "-Infinity",
        })
    );
    assert_eq!(
        nan,
        json!({
            "double": "NaN",
            "float": "NaN",
        })
    );
}

#[test]
fn serialize_scalars_default() {
    let value = to_json(&Scalars::default());

    assert_eq!(value, json!({}));
}

#[test]
fn serialize_array() {
    let value = to_json(&ScalarArrays {
        double: vec![1.1, 2.2],
        ..Default::default()
    });

    assert_eq!(
        value,
        json!({
            "double": vec![1.1, 2.2],
        })
    );
}

#[test]
fn serialize_complex_type() {
    let value = to_json(&ComplexType {
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
    });

    assert_eq!(
        value,
        json!({
            "stringMap": {
                "1": {
                    "double": 1.1,
                    "float": 2.2f32,
                    "int32": 3,
                },
                "2": {
                    "int64": "4",
                    "uint32": 5,
                    "uint64": "6",
                },
            },
            "intMap": {
                "3": {
                    "sint32": 7,
                    "sint64": "8",
                    "fixed32": 9,
                },
                "4": {
                    "sint64": "8",
                    "fixed32": 9,
                    "fixed64": "10",
                },
            },
            "nested": {
                "sfixed32": 11,
                "sfixed64": "12",
                "bool": true,
                "string": "5",
                "bytes": "Ng==",
            },
            "myEnum": ["DEFAULT", "FOO", 2, "BAR", "NEG"],
            "optionalEnum": "FOO",
        })
    );
}

#[test]
fn serialize_well_known_types() {
    let value = to_json(&WellKnownTypes {
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
    });

    assert_eq!(
        value,
        json!({
            "timestamp": "1972-01-01T10:00:20.021Z",
            "duration": "1.000340012s",
            "struct": {
                "number": 42.0,
                "null": null,
            },
            "float": 42.1f32,
            "double": 12.4,
            "int32": 1,
            "int64": "-2",
            "uint32": 3,
            "uint64": "4",
            "bool": false,
            "string": "hello",
            "bytes": "aGVsbG8=",
            "mask": "fieldOne,fieldTwo.b.d",
            "list": ["foo", false],
            "empty": {}
        })
    );
}

#[test]
fn serialize_no_stringify_64_bit_integers() {
    let value = to_json_with_options(
        &Scalars {
            int32: 3,
            int64: -4,
            uint32: 5,
            uint64: 6,
            sint32: 7,
            sint64: -8,
            fixed32: 9,
            fixed64: 10,
            sfixed32: 11,
            sfixed64: -12,
            ..Default::default()
        },
        &SerializeOptions::new().stringify_64_bit_integers(false),
    );

    assert_eq!(
        value,
        json!({
            "int32": 3,
            "int64": -4,
            "uint32": 5,
            "uint64": 6,
            "sint32": 7,
            "sint64": -8,
            "fixed32": 9,
            "fixed64": 10,
            "sfixed32": 11,
            "sfixed64": -12,
        })
    );
}

#[test]
fn serialize_use_proto_field_name() {
    let value = to_json_with_options(
        &ComplexType {
            my_enum: vec![0, 1, 2, 3, -4],
            ..Default::default()
        },
        &SerializeOptions::new().use_proto_field_name(true),
    );

    assert_eq!(
        value,
        json!({
            "my_enum": ["DEFAULT", "FOO", 2, "BAR", "NEG"],
        })
    );
}

#[test]
fn serialize_use_enum_numbers() {
    let value = to_json_with_options(
        &ComplexType {
            my_enum: vec![0, 1, 2, 3, -4],
            ..Default::default()
        },
        &SerializeOptions::new().use_enum_numbers(true),
    );

    assert_eq!(
        value,
        json!({
            "myEnum": [0, 1, 2, 3, -4],
        })
    );
}

#[test]
fn serialize_skip_default_fields() {
    let value = to_json_with_options(
        &ComplexType {
            string_map: HashMap::from_iter([(
                "1".to_owned(),
                Scalars {
                    ..Default::default()
                },
            )]),
            int_map: HashMap::default(),
            nested: None,
            my_enum: vec![],
            optional_enum: 0,
        },
        &SerializeOptions::new().skip_default_fields(false),
    );

    assert_eq!(
        value,
        json!({
            "stringMap": {
                "1": {
                    "double": 0.0,
                    "float": 0.0,
                    "int32": 0,
                    "int64": "0",
                    "uint32": 0,
                    "uint64": "0",
                    "sint32": 0,
                    "sint64": "0",
                    "fixed32": 0,
                    "fixed64": "0",
                    "sfixed32": 0,
                    "sfixed64": "0",
                    "bool": false,
                    "string": "",
                    "bytes": "",
                },
            },
            "intMap": {},
            "myEnum": [],
            "optionalEnum": "DEFAULT"
        })
    );
}

#[test]
fn serialize_string_skip_default_fields() {
    let value = Point::default();
    let mut dynamic = DynamicMessage::new(value.descriptor());
    dynamic.transcode_from(&value).unwrap();
    let mut s = serde_json::Serializer::new(vec![]);

    dynamic
        .serialize_with_options(&mut s, &SerializeOptions::new().skip_default_fields(false))
        .unwrap();

    assert_eq!(
        String::from_utf8(s.into_inner()).unwrap(),
        "{\"latitude\":0,\"longitude\":0}"
    );
}

#[test]
fn deserialize_scalars() {
    let value: Scalars = from_json(
        json!({
            "double": 1.1,
            "float": 2.2f32,
            "int32": 3,
            "int64": "4",
            "uint32": 5,
            "uint64": "6",
            "sint32": 7,
            "sint64": "8",
            "fixed32": 9,
            "fixed64": "10",
            "sfixed32": 11,
            "sfixed64": "12",
            "bool": true,
            "string": "5",
            "bytes": "aaa+bbb/WA==",
        }),
        "test.Scalars",
    );

    assert_eq!(
        value,
        Scalars {
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
            bytes: b"i\xa6\xbem\xb6\xffX".to_vec(),
        },
    );
}

#[test]
fn deserialize_scalars_float_extrema() {
    let inf: Scalars = from_json(
        json!({
            "double": "Infinity",
            "float": "Infinity",
        }),
        "test.Scalars",
    );
    let neg_inf: Scalars = from_json(
        json!({
            "double": "-Infinity",
            "float": "-Infinity",
        }),
        "test.Scalars",
    );
    let nan: Scalars = from_json(
        json!({
            "double": "NaN",
            "float": "NaN",
        }),
        "test.Scalars",
    );

    assert_eq!(
        inf,
        Scalars {
            float: f32::INFINITY,
            double: f64::INFINITY,
            ..Default::default()
        },
    );
    assert_eq!(
        neg_inf,
        Scalars {
            float: f32::NEG_INFINITY,
            double: f64::NEG_INFINITY,
            ..Default::default()
        },
    );
    assert!(nan.float.is_nan());
    assert!(nan.double.is_nan());
}

#[test]
fn deserialize_scalars_empty() {
    let value: Scalars = from_json(json!({}), "test.Scalars");

    assert_eq!(value, Scalars::default());
}

#[test]
#[should_panic(expected = "unrecognized field name 'unknown_field'")]
fn deserialize_deny_unknown_fields() {
    from_json_with_options::<Scalars>(
        json!({
            "unknown_field": 123,
        }),
        "test.Scalars",
        &DeserializeOptions::new(),
    );
}

#[test]
fn deserialize_allow_unknown_fields() {
    let value = from_json_with_options::<Scalars>(
        json!({
            "unknown_field": 123,
        }),
        "test.Scalars",
        &DeserializeOptions::new().deny_unknown_fields(false),
    );

    assert_eq!(value, Default::default());
}

#[test]
fn deserialize_scalars_null() {
    let value: Scalars = from_json(
        json!({
            "double": null,
            "float": null,
            "int32": null,
            "int64": null,
            "uint32": null,
            "uint64": null,
            "sint32": null,
            "sint64": null,
            "fixed32": null,
            "fixed64": null,
            "sfixed32": null,
            "sfixed64": null,
            "bool": null,
            "string": null,
            "bytes": null,
        }),
        "test.Scalars",
    );

    assert_eq!(value, Scalars::default());
}

#[test]
fn deserialize_scalars_alt() {
    let value: Scalars = from_json(
        json!({
            "double": "1.1",
            "float": "2.2",
            "int32": "3",
            "int64": 4,
            "uint32": "5",
            "uint64": 6,
            "sint32": "7",
            "sint64": 8,
            "fixed32": "9",
            "fixed64": 10,
            "sfixed32": "11",
            "sfixed64": 12,
            "bool": true,
            "string": "5",
            "bytes": "aaa-bbb_WA==",
        }),
        "test.Scalars",
    );

    assert_eq!(
        value,
        Scalars {
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
            bytes: b"i\xa6\xbem\xb6\xffX".to_vec(),
        },
    );
}

#[test]
fn deserialize_aliased_enum() {
    let value1: MessageWithAliasedEnum = from_json(
        json!({
            "aliased": "A"
        }),
        "test.MessageWithAliasedEnum",
    );
    let value2: MessageWithAliasedEnum = from_json(
        json!({
            "aliased": "B"
        }),
        "test.MessageWithAliasedEnum",
    );

    assert_eq!(value1, MessageWithAliasedEnum { aliased: 1 },);
    assert_eq!(value2, MessageWithAliasedEnum { aliased: 1 },);
}

#[test]
fn deserialize_array() {
    let value: ScalarArrays = from_json(
        json!({
            "double": [1.1, 2.2],
        }),
        ".test.ScalarArrays",
    );

    assert_eq!(
        value,
        ScalarArrays {
            double: vec![1.1, 2.2],
            ..Default::default()
        },
    );
}

#[test]
fn deserialize_complex_type() {
    let value: ComplexType = from_json(
        json!({
            "stringMap": {
                "1": {
                    "double": 1.1,
                    "float": 2.2f32,
                    "int32": 3,
                },
                "2": {
                    "int64": "4",
                    "uint32": 5,
                    "uint64": "6",
                },
            },
            "intMap": {
                "3": {
                    "sint32": 7,
                    "sint64": "8",
                    "fixed32": 9,
                },
                "4": {
                    "sint64": "8",
                    "fixed32": 9,
                    "fixed64": "10",
                },
            },
            "nested": {
                "sfixed32": 11,
                "sfixed64": "12",
                "bool": true,
                "string": "5",
                "bytes": "Ng==",
            },
            "myEnum": ["DEFAULT", "FOO", 2, "BAR", "NEG"],
            "optionalEnum": "FOO",
        }),
        ".test.ComplexType",
    );

    assert_eq!(
        value,
        ComplexType {
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
        }
    );
}

#[test]
fn deserialize_well_known_types() {
    let value: WellKnownTypes = from_json(
        json!({
            "timestamp": "1972-01-01T10:00:20.021Z",
            "duration": "1.000340012s",
            "struct": {
                "number": 42.0,
                "null": null,
            },
            "float": 42.1f32,
            "double": 12.4,
            "int32": 1,
            "int64": "-2",
            "uint32": 3,
            "uint64": "4",
            "bool": false,
            "string": "hello",
            "bytes": "aGVsbG8=",
            "mask": "fieldOne,fieldTwo.b.d",
            "list": ["foo", false],
            "empty": {}
        }),
        ".test.WellKnownTypes",
    );

    assert_eq!(
        value,
        WellKnownTypes {
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
        }
    );
}

#[test]
fn serialize_any() {
    let json = wkt_to_json(
        &prost_types::Any {
            type_url: "type.googleapis.com/test.Point".to_owned(),
            value: Point {
                longitude: 1,
                latitude: 2,
            }
            .encode_to_vec(),
        },
        "google.protobuf.Any",
    );

    assert_eq!(
        json,
        json!({
            "@type": "type.googleapis.com/test.Point",
            "longitude": 1,
            "latitude": 2,
        })
    );
}

#[test]
fn serialize_any_wkt() {
    let json = wkt_to_json(
        &prost_types::Any {
            type_url: "type.googleapis.com/google.protobuf.Int32Value".to_owned(),
            value: 5i32.encode_to_vec(),
        },
        "google.protobuf.Any",
    );

    assert_eq!(
        json,
        json!({
            "@type": "type.googleapis.com/google.protobuf.Int32Value",
            "value": 5,
        })
    );
}

#[test]
fn deserialize_any() {
    let value: prost_types::Any = from_json(
        json!({
            "@type": "type.googleapis.com/test.Point",
            "longitude": 1,
            "latitude": 2,
        }),
        "google.protobuf.Any",
    );

    assert_eq!(
        value,
        prost_types::Any {
            type_url: "type.googleapis.com/test.Point".to_owned(),
            value: Point {
                longitude: 1,
                latitude: 2,
            }
            .encode_to_vec(),
        }
    );
}

#[test]
fn deserialize_any_buffer_fields() {
    let value: prost_types::Any = from_json(
        json!({
            "longitude": 1,
            "latitude": 2,
            "@type": "type.googleapis.com/test.Point",
        }),
        "google.protobuf.Any",
    );

    assert_eq!(
        value,
        prost_types::Any {
            type_url: "type.googleapis.com/test.Point".to_owned(),
            value: Point {
                longitude: 1,
                latitude: 2,
            }
            .encode_to_vec(),
        }
    );
}

#[test]
fn deserialize_any_wkt() {
    let value: prost_types::Any = from_json(
        json!({
            "@type": "type.googleapis.com/google.protobuf.Int32Value",
            "value": 5,
        }),
        "google.protobuf.Any",
    );

    assert_eq!(
        value,
        prost_types::Any {
            type_url: "type.googleapis.com/google.protobuf.Int32Value".to_owned(),
            value: 5i32.encode_to_vec(),
        }
    );
}

#[test]
#[should_panic(expected = "unrecognized field name 'unknown'")]
fn deserialize_any_deny_unknown_fields() {
    from_json::<prost_types::Any>(
        json!({
            "@type": "type.googleapis.com/google.protobuf.Int32Value",
            "value": 5,
            "unknown": "hello",
        }),
        "google.protobuf.Any",
    );
}

#[test]
#[should_panic(expected = "unrecognized field name 'unknown'")]
fn deserialize_any_deny_unknown_buffered_fields() {
    from_json::<prost_types::Any>(
        json!({
            "value": 5,
            "unknown": "hello",
            "@type": "type.googleapis.com/google.protobuf.Int32Value",
        }),
        "google.protobuf.Any",
    );
}

#[test]
fn deserialize_any_allow_unknown_fields() {
    let value: prost_types::Any = from_json_with_options(
        json!({
            "value": 5,
            "unknown": "hello",
            "@type": "type.googleapis.com/google.protobuf.Int32Value",
        }),
        "google.protobuf.Any",
        &DeserializeOptions::new().deny_unknown_fields(false),
    );

    assert_eq!(
        value,
        prost_types::Any {
            type_url: "type.googleapis.com/google.protobuf.Int32Value".to_owned(),
            value: 5i32.encode_to_vec(),
        }
    );
}

#[test]
fn deserialize_duration_fraction_digits() {
    let value: prost_types::Duration = from_json(json!("1.00034s"), "google.protobuf.Duration");

    assert_eq!(
        value,
        prost_types::Duration {
            seconds: 1,
            nanos: 340_000,
        }
    );
}

#[test]
fn deserialize_duration_out_of_range() {
    let value: prost_types::Duration =
        from_json(json!("-15.000340123s"), "google.protobuf.Duration");

    assert_eq!(
        value,
        prost_types::Duration {
            seconds: -15,
            nanos: -340_123,
        }
    );
}

#[test]
#[should_panic(expected = "duration out of range")]
fn deserialize_negative_duration() {
    from_json::<prost_types::Duration>(
        json!("-18446744073709551615.000340123s"),
        "google.protobuf.Duration",
    );
}

#[test]
fn ints_allow_trailing_zeros() {
    let json = r#"{
        "int32": -1.000,
        "uint32": 2.000,
        "int64": -3.000,
        "uint64": 4.000
    }"#;

    let mut s = serde_json::de::Deserializer::from_str(json);
    let dynamic_message = DynamicMessage::deserialize(
        test_file_descriptor()
            .get_message_by_name("test.Scalars")
            .unwrap(),
        &mut s,
    )
    .unwrap();
    s.end().unwrap();

    assert_eq!(
        dynamic_message.transcode_to::<Scalars>().unwrap(),
        Scalars {
            int32: -1,
            uint32: 2,
            int64: -3,
            uint64: 4,
            ..Default::default()
        }
    );
}

#[test]
#[should_panic(expected = "expected integer value")]
fn ints_deny_fractional() {
    let json = r#"{
        "int32": -1.01,
    }"#;

    let mut s = serde_json::de::Deserializer::from_str(json);
    let _ = DynamicMessage::deserialize(
        test_file_descriptor()
            .get_message_by_name("test.Scalars")
            .unwrap(),
        &mut s,
    )
    .unwrap();
    s.end().unwrap();
}

#[test]
fn null_in_oneof() {
    let json = json!({ "oneofNull": null });

    let value: MessageWithOneof = from_json(json, "test.MessageWithOneof");
    assert_eq!(
        value.test_oneof,
        Some(message_with_oneof::TestOneof::OneofNull(0))
    );
}

#[test]
#[should_panic(expected = "multiple fields provided for oneof 'test_oneof'")]
fn duplicate_oneof_field() {
    let json = json!({
        "oneofField1": "hello",
        "oneofNull": null,
    });

    let _: MessageWithOneof = from_json(json, "test.MessageWithOneof");
}

#[test]
fn roundtrip_oneof_field_with_options() {
    roundtrip_json_with_options(
        &MessageWithOneof::default(),
        &SerializeOptions::new().skip_default_fields(false),
        &DeserializeOptions::new(),
    )
    .unwrap();
}

#[test]
fn value_null_in_oneof() {
    let json = json!({ "oneofValueNull": null });

    let value: MessageWithOneof = from_json(json, "test.MessageWithOneof");
    assert_eq!(
        value.test_oneof,
        Some(message_with_oneof::TestOneof::OneofValueNull(
            prost_types::Value {
                kind: Some(prost_types::value::Kind::NullValue(0)),
            }
        )),
    );
}

#[test]
fn null_old_format() {
    let json = json!({ "null": "NULL_VALUE" });

    let value: WellKnownTypes = from_json(json, "test.WellKnownTypes");
    assert_eq!(
        value,
        WellKnownTypes {
            null: 0,
            ..Default::default()
        }
    );
}

#[test]
#[should_panic(expected = "float value out of range")]
fn float_out_of_range() {
    let json = json!({ "float": -3.502823e+38 });

    let _: Scalars = from_json(json, "test.Scalars");
}

#[test]
fn bytes_forgiving_decode() {
    let json = json!({ "bytes": "-_" });

    let scalars: Scalars = from_json(json, "test.Scalars");
    assert_eq!(
        scalars,
        Scalars {
            bytes: b"\xfb".to_vec(),
            ..Default::default()
        }
    );
}

#[test]
fn duration_fractional_digits() {
    assert_eq!(
        to_json(&prost_types::Duration {
            seconds: 1,
            nanos: 0,
        }),
        json!("1s"),
    );
    assert_eq!(
        to_json(&prost_types::Duration {
            seconds: 1,
            nanos: 123000000,
        }),
        json!("1.123s"),
    );
    assert_eq!(
        to_json(&prost_types::Duration {
            seconds: 1,
            nanos: 123456000,
        }),
        json!("1.123456s"),
    );
    assert_eq!(
        to_json(&prost_types::Duration {
            seconds: 1,
            nanos: 123456789,
        }),
        json!("1.123456789s"),
    );
}

#[test]
#[should_panic(expected = "timestamp out of range")]
fn serialize_timestamp_seconds_out_of_range() {
    to_json(&WellKnownTypes {
        timestamp: Some(prost_types::Timestamp {
            seconds: 253402300800,
            nanos: 1,
        }),
        ..Default::default()
    });
}

#[test]
#[should_panic(expected = "timestamp out of range")]
fn deserialize_timestamp_seconds_out_of_range() {
    let _: prost_types::Timestamp =
        from_json(json!("0000-01-01T00:00:00Z"), "google.protobuf.Timestamp");
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        .. ProptestConfig::default()
    })]

    #[test]
    fn roundtrip_arb_scalars(message: Scalars) {
        roundtrip_json(&message)?;
    }

    #[test]
    fn roundtrip_arb_scalars_options(message: Scalars) {
        roundtrip_json_with_options(
            &message,
            &SerializeOptions::new()
                .stringify_64_bit_integers(false)
                .use_enum_numbers(true)
                .use_proto_field_name(true)
                .skip_default_fields(false),
            &DeserializeOptions::new()
                .deny_unknown_fields(true)
        )?;
    }

    #[test]
    fn deserialize_error_scalars(json in arbitrary::json()) {
        let _ = try_from_json_string_with_options(&json, ".test.Scalars", &DeserializeOptions::default());
        let _ = try_from_json_string_with_options(&json, ".test.Scalars", &DeserializeOptions::default().deny_unknown_fields(false));
    }

    #[test]
    fn roundtrip_arb_scalar_arrays(message: ScalarArrays) {
        roundtrip_json(&message)?;
    }

    #[test]
    fn roundtrip_arb_scalar_arrays_options(message: ScalarArrays) {
        roundtrip_json_with_options(
            &message,
            &SerializeOptions::new()
                .stringify_64_bit_integers(false)
                .use_enum_numbers(true)
                .use_proto_field_name(true)
                .skip_default_fields(false),
            &DeserializeOptions::new()
                .deny_unknown_fields(true)
        )?;
    }

    #[test]
    fn deserialize_error_scalar_arrays(json in arbitrary::json()) {
        let _ = try_from_json_string_with_options(&json, ".test.ScalarArrays", &DeserializeOptions::default());
        let _ = try_from_json_string_with_options(&json, ".test.ScalarArrays", &DeserializeOptions::default().deny_unknown_fields(false));
    }

    #[test]
    fn roundtrip_arb_complex_type(message: ComplexType) {
        roundtrip_json(&message)?;
    }

    #[test]
    fn roundtrip_arb_complex_type_options(message: ComplexType) {
        roundtrip_json_with_options(
            &message,
            &SerializeOptions::new()
                .stringify_64_bit_integers(false)
                .use_enum_numbers(true)
                .use_proto_field_name(true)
                .skip_default_fields(false),
            &DeserializeOptions::new()
                .deny_unknown_fields(true)
        )?;
    }

    #[test]
    fn deserialize_error_complex_type(json in arbitrary::json()) {
        let _ = try_from_json_string_with_options(&json, ".test.ComplexType", &DeserializeOptions::default());
        let _ = try_from_json_string_with_options(&json, ".test.ComplexType", &DeserializeOptions::default().deny_unknown_fields(false));
    }

    #[test]
    fn roundtrip_arb_well_known_types(message: WellKnownTypes) {
        roundtrip_json(&message)?;
    }

    #[test]
    fn roundtrip_arb_well_known_types_options(message: WellKnownTypes) {
        roundtrip_json_with_options(
            &message,
            &SerializeOptions::new()
                .stringify_64_bit_integers(false)
                .use_enum_numbers(true)
                .use_proto_field_name(true)
                .skip_default_fields(false),
            &DeserializeOptions::new()
                .deny_unknown_fields(true)
        )?;
    }

    #[test]
    fn deserialize_error_well_known_types(json in arbitrary::json()) {
        let _ = try_from_json_string_with_options(&json, ".test.WellKnownTypes", &DeserializeOptions::default());
        let _ = try_from_json_string_with_options(&json, ".test.WellKnownTypes", &DeserializeOptions::default().deny_unknown_fields(false));
    }
}

#[test]
fn roundtrip_file_descriptor_set() {
    roundtrip_json(test_file_descriptor().file_descriptor_set()).unwrap();
}

#[test]
fn roundtrip_file_descriptor_set_with_options() {
    roundtrip_json_with_options(
        test_file_descriptor().file_descriptor_set(),
        &SerializeOptions::new()
            .stringify_64_bit_integers(false)
            .use_enum_numbers(true)
            .use_proto_field_name(true)
            .skip_default_fields(false),
        &DeserializeOptions::new().deny_unknown_fields(true),
    )
    .unwrap();
}

fn to_json<T>(message: &T) -> serde_json::Value
where
    T: PartialEq + Debug + ReflectMessage + Default,
{
    to_json_with_options(message, &Default::default())
}

fn to_json_with_options<T>(message: &T, options: &SerializeOptions) -> serde_json::Value
where
    T: PartialEq + Debug + ReflectMessage + Default,
{
    message
        .transcode_to_dynamic()
        .serialize_with_options(serde_json::value::Serializer, options)
        .unwrap()
}

fn to_json_string_with_options<T>(message: &T, options: &SerializeOptions) -> String
where
    T: PartialEq + Debug + ReflectMessage + Default,
{
    let mut ser = serde_json::Serializer::new(Vec::new());
    message
        .transcode_to_dynamic()
        .serialize_with_options(&mut ser, options)
        .unwrap();
    String::from_utf8(ser.into_inner()).unwrap()
}

fn wkt_to_json<T>(message: &T, message_name: &str) -> serde_json::Value
where
    T: Message,
{
    let mut dynamic_message = DynamicMessage::new(
        test_file_descriptor()
            .get_message_by_name(message_name)
            .unwrap(),
    );
    dynamic_message.transcode_from(message).unwrap();
    serde_json::to_value(&dynamic_message).unwrap()
}

fn from_json<T>(json: serde_json::Value, message_name: &str) -> T
where
    T: PartialEq + Debug + Message + Default,
{
    from_json_with_options(json, message_name, &Default::default())
}

fn from_json_with_options<T>(
    json: serde_json::Value,
    message_name: &str,
    options: &DeserializeOptions,
) -> T
where
    T: PartialEq + Debug + Message + Default,
{
    DynamicMessage::deserialize_with_options(
        test_file_descriptor()
            .get_message_by_name(message_name)
            .unwrap(),
        json,
        options,
    )
    .unwrap()
    .transcode_to()
    .unwrap()
}

fn try_from_json_string_with_options(
    json: &str,
    message_name: &str,
    options: &DeserializeOptions,
) -> serde_json::Result<DynamicMessage> {
    let mut de = serde_json::Deserializer::from_str(json);
    let message = DynamicMessage::deserialize_with_options(
        test_file_descriptor()
            .get_message_by_name(message_name)
            .unwrap(),
        &mut de,
        options,
    )?;
    de.end().unwrap();

    Ok(message)
}

fn from_json_string_with_options<T>(
    json: &str,
    message_name: &str,
    options: &DeserializeOptions,
) -> T
where
    T: PartialEq + Debug + Message + Default,
{
    try_from_json_string_with_options(json, message_name, options)
        .unwrap()
        .transcode_to()
        .unwrap()
}

fn roundtrip_json<T>(message: &T) -> Result<(), TestCaseError>
where
    T: PartialEq + Debug + ReflectMessage + Default,
{
    roundtrip_json_with_options(message, &Default::default(), &Default::default())
}

fn roundtrip_json_with_options<T>(
    message: &T,
    ser_options: &SerializeOptions,
    de_options: &DeserializeOptions,
) -> Result<(), TestCaseError>
where
    T: PartialEq + Debug + ReflectMessage + Default,
{
    let json = to_json_string_with_options(message, ser_options);
    let roundtripped_message =
        from_json_string_with_options(&json, message.descriptor().full_name(), de_options);
    prop_assert_eq!(message, &roundtripped_message);
    Ok(())
}
