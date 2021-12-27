use std::fmt::Debug;

use prost::Message;
use serde::de::DeserializeSeed;
use serde_json::json;

use crate::{to_dynamic, Scalars, TEST_FILE_DESCRIPTOR};

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

fn to_json<T>(message: &T, message_name: &str) -> serde_json::Value
where
    T: PartialEq + Debug + Message + Default,
{
    serde_json::to_value(&to_dynamic(message, message_name)).unwrap()
}

fn from_json<T>(json: serde_json::Value, message_name: &str) -> T
where
    T: PartialEq + Debug + Message + Default,
{
    TEST_FILE_DESCRIPTOR
        .get_message_by_name(message_name)
        .unwrap()
        .deserialize(json)
        .unwrap()
        .to_message()
        .unwrap()
}
