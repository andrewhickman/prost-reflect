use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
};

use proptest::{prelude::*, test_runner::TestCaseError};
use prost::Message;
use prost_reflect::{DeserializeOptions, DynamicMessage, SerializeOptions};
use serde_json::json;

use crate::{to_dynamic, ComplexType, ScalarArrays, Scalars, WellKnownTypes, TEST_FILE_DESCRIPTOR};

#[test]
fn serialize_scalars() {
    let value = to_json(
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
            bytes: b"i\xa6\xbem\xb6\xffX".to_vec(),
        },
        ".test.Scalars",
    );

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
    let inf = to_json(
        &Scalars {
            float: f32::INFINITY,
            double: f64::INFINITY,
            ..Default::default()
        },
        ".test.Scalars",
    );
    let neg_inf = to_json(
        &Scalars {
            float: f32::NEG_INFINITY,
            double: f64::NEG_INFINITY,
            ..Default::default()
        },
        ".test.Scalars",
    );
    let nan = to_json(
        &Scalars {
            float: f32::NAN,
            double: f64::NAN,
            ..Default::default()
        },
        ".test.Scalars",
    );

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
    let value = to_json(&Scalars::default(), ".test.Scalars");

    assert_eq!(value, json!({}));
}

#[test]
fn serialize_array() {
    let value = to_json(
        &ScalarArrays {
            double: vec![1.1, 2.2],
            ..Default::default()
        },
        ".test.ScalarArrays",
    );

    assert_eq!(
        value,
        json!({
            "double": vec![1.1, 2.2],
        })
    );
}

#[test]
fn serialize_complex_type() {
    let value = to_json(
        &ComplexType {
            string_map: HashMap::from([
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
            int_map: HashMap::from([
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
            my_enum: vec![0, 1, 2, 3],
        },
        ".test.ComplexType",
    );

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
            "myEnum": ["DEFAULT", "FOO", 2, "BAR"],
        })
    );
}

#[test]
fn serialize_well_known_types() {
    let value = to_json(
        &WellKnownTypes {
            timestamp: Some(prost_types::Timestamp {
                seconds: 63_108_020,
                nanos: 21_000_000,
            }),
            duration: Some(prost_types::Duration {
                seconds: 1,
                nanos: 340_012,
            }),
            r#struct: Some(prost_types::Struct {
                fields: BTreeMap::from([
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
        },
        ".test.WellKnownTypes",
    );

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
        ".test.Scalars",
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
            my_enum: vec![0, 1, 2, 3],
            ..Default::default()
        },
        ".test.ComplexType",
        &SerializeOptions::new().use_proto_field_name(true),
    );

    assert_eq!(
        value,
        json!({
            "my_enum": ["DEFAULT", "FOO", 2, "BAR"],
        })
    );
}

#[test]
fn serialize_use_enum_numbers() {
    let value = to_json_with_options(
        &ComplexType {
            my_enum: vec![0, 1, 2, 3],
            ..Default::default()
        },
        ".test.ComplexType",
        &SerializeOptions::new().use_enum_numbers(true),
    );

    assert_eq!(
        value,
        json!({
            "myEnum": [0, 1, 2, 3],
        })
    );
}

#[test]
fn serialize_emit_unpopulated_fields() {
    let value = to_json_with_options(
        &ComplexType {
            string_map: HashMap::from([(
                "1".to_owned(),
                Scalars {
                    ..Default::default()
                },
            )]),
            int_map: HashMap::default(),
            nested: None,
            my_enum: vec![],
        },
        ".test.ComplexType",
        &SerializeOptions::new().emit_unpopulated_fields(true),
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
            "nested": null,
            "myEnum": [],
        })
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
        ".test.Scalars",
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
        ".test.Scalars",
    );
    let neg_inf: Scalars = from_json(
        json!({
            "double": "-Infinity",
            "float": "-Infinity",
        }),
        ".test.Scalars",
    );
    let nan: Scalars = from_json(
        json!({
            "double": "NaN",
            "float": "NaN",
        }),
        ".test.Scalars",
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
    let value: Scalars = from_json(json!({}), ".test.Scalars");

    assert_eq!(value, Scalars::default());
}

#[test]
#[should_panic(expected = "unrecognized field name 'unknown_field'")]
fn deserialize_deny_unknown_fields() {
    from_json_with_options::<Scalars>(
        json!({
            "unknown_field": 123,
        }),
        ".test.Scalars",
        &DeserializeOptions::new(),
    );
}

#[test]
fn deserialize_allow_unknown_fields() {
    let value = from_json_with_options::<Scalars>(
        json!({
            "unknown_field": 123,
        }),
        ".test.Scalars",
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
        ".test.Scalars",
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
        ".test.Scalars",
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
            "myEnum": ["DEFAULT", "FOO", 2, "BAR"],
        }),
        ".test.ComplexType",
    );

    assert_eq!(
        value,
        ComplexType {
            string_map: HashMap::from([
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
            int_map: HashMap::from([
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
            my_enum: vec![0, 1, 2, 3],
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
                fields: BTreeMap::from([
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

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        .. ProptestConfig::default()
    })]

    #[test]
    fn roundtrip_arb_scalars(message: Scalars) {
        roundtrip_json(&message, ".test.Scalars")?;
    }

    #[test]
    fn roundtrip_arb_scalars_options(message: Scalars) {
        roundtrip_json_with_options(
            &message,
            ".test.Scalars",
            &SerializeOptions::new()
                .stringify_64_bit_integers(false)
                .use_enum_numbers(true)
                .use_proto_field_name(true)
                .emit_unpopulated_fields(true),
            &DeserializeOptions::new()
                .deny_unknown_fields(true)
        )?;
    }

    #[test]
    fn roundtrip_arb_scalar_arrays(message: ScalarArrays) {
        roundtrip_json(&message, ".test.ScalarArrays")?;
    }

    #[test]
    fn roundtrip_arb_scalar_arrays_options(message: ScalarArrays) {
        roundtrip_json_with_options(
            &message,
            ".test.ScalarArrays",
            &SerializeOptions::new()
                .stringify_64_bit_integers(false)
                .use_enum_numbers(true)
                .use_proto_field_name(true)
                .emit_unpopulated_fields(true),
            &DeserializeOptions::new()
                .deny_unknown_fields(true)
        )?;
    }

    #[test]
    fn roundtrip_arb_complex_type(message: ComplexType) {
        roundtrip_json(&message, ".test.ComplexType")?;
    }

    #[test]
    fn roundtrip_arb_complex_type_options(message: ComplexType) {
        roundtrip_json_with_options(
            &message,
            ".test.ComplexType",
            &SerializeOptions::new()
                .stringify_64_bit_integers(false)
                .use_enum_numbers(true)
                .use_proto_field_name(true)
                .emit_unpopulated_fields(true),
            &DeserializeOptions::new()
                .deny_unknown_fields(true)
        )?;
    }

    #[test]
    fn roundtrip_arb_well_known_types(message: WellKnownTypes) {
        roundtrip_json(&message, ".test.WellKnownTypes")?;
    }

    #[test]
    fn roundtrip_arb_well_known_types_options(message: WellKnownTypes) {
        roundtrip_json_with_options(
            &message,
            ".test.WellKnownTypes",
            &SerializeOptions::new()
                .stringify_64_bit_integers(false)
                .use_enum_numbers(true)
                .use_proto_field_name(true)
                .emit_unpopulated_fields(true),
            &DeserializeOptions::new()
                .deny_unknown_fields(true)
        )?;
    }
}

fn to_json<T>(message: &T, message_name: &str) -> serde_json::Value
where
    T: PartialEq + Debug + Message + Default,
{
    to_json_with_options(message, message_name, &Default::default())
}

fn to_json_with_options<T>(
    message: &T,
    message_name: &str,
    options: &SerializeOptions,
) -> serde_json::Value
where
    T: PartialEq + Debug + Message + Default,
{
    to_dynamic(message, message_name)
        .serialize_with_options(serde_json::value::Serializer, options)
        .unwrap()
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
        TEST_FILE_DESCRIPTOR
            .get_message_by_name(message_name)
            .unwrap(),
        json,
        options,
    )
    .unwrap()
    .to_message()
    .unwrap()
}

fn roundtrip_json<T>(message: &T, message_name: &str) -> Result<(), TestCaseError>
where
    T: PartialEq + Debug + Message + Default,
{
    roundtrip_json_with_options(
        message,
        message_name,
        &Default::default(),
        &Default::default(),
    )
}

fn roundtrip_json_with_options<T>(
    message: &T,
    message_name: &str,
    ser_options: &SerializeOptions,
    de_options: &DeserializeOptions,
) -> Result<(), TestCaseError>
where
    T: PartialEq + Debug + Message + Default,
{
    let json = to_json_with_options(message, message_name, ser_options);
    let roundtripped_message = from_json_with_options(json, message_name, de_options);
    prop_assert_eq!(message, &roundtripped_message);
    Ok(())
}
