use std::time::SystemTime;
use std::{fmt::Write, iter::FromIterator};

use proptest::prelude::*;
use prost_types::{value::Kind, Duration, FieldMask, ListValue, Struct, Timestamp, Value};

pub fn timestamp() -> impl Strategy<Value = Timestamp> {
    any::<SystemTime>().prop_map(Into::into)
}

prop_compose! {
    pub fn duration()(
        seconds in (-315_576_000_000i64..=315_576_000_000),
        nanos in (-999_999_999i32..=999_999_999),
    ) -> Duration {
        let mut duration = Duration { seconds, nanos };
        duration.normalize();
        duration
    }
}

pub fn struct_() -> impl Strategy<Value = Struct> {
    struct_inner(value().boxed())
}

pub fn struct_inner(value_strat: BoxedStrategy<Value>) -> impl Strategy<Value = Struct> {
    prop::collection::btree_map(any::<String>(), value_strat, 0..4)
        .prop_map(|fields| Struct { fields })
}

pub fn list() -> impl Strategy<Value = ListValue> {
    list_inner(value().boxed())
}

pub fn list_inner(value_strat: BoxedStrategy<Value>) -> impl Strategy<Value = ListValue> {
    prop::collection::vec(value_strat, 0..4).prop_map(|values| ListValue { values })
}

fn arb_finite_float() -> impl Strategy<Value = f64> {
    use prop::num::f64::*;
    POSITIVE | NEGATIVE | NORMAL | SUBNORMAL | ZERO
}

pub fn value() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Kind::NullValue(0)),
        arb_finite_float().prop_map(Kind::NumberValue),
        any::<String>().prop_map(Kind::StringValue),
        prop::bool::ANY.prop_map(Kind::BoolValue),
    ]
    .prop_map(|kind| Value { kind: Some(kind) })
    .prop_recursive(4, 10, 4, |value| {
        prop_oneof![
            list_inner(value.clone()).prop_map(Kind::ListValue),
            struct_inner(value).prop_map(Kind::StructValue)
        ]
        .prop_map(|kind| Value { kind: Some(kind) })
    })
}

pub fn mask() -> impl Strategy<Value = FieldMask> {
    let parts = prop::collection::vec("([a-z]{1,3}_){0,3}[a-z]{1,3}", 1..4);
    let paths = prop::collection::vec(parts, 0..4);
    paths.prop_map(|paths| FieldMask {
        paths: paths
            .into_iter()
            .map(|parts| {
                let mut parts = parts.into_iter();
                let mut path = parts.next().unwrap();
                for part in parts {
                    write!(path, ".{part}").unwrap();
                }
                path
            })
            .collect(),
    })
}

pub fn json() -> impl Strategy<Value = String> {
    fn arb_json_key() -> impl Strategy<Value = String> {
        // Use real field names to make the deserialization error test more interesting
        prop_oneof![
            2 => Just("float".to_owned()),
            2 => Just("double".to_owned()),
            2 => Just("int32".to_owned()),
            2 => Just("int64".to_owned()),
            2 => Just("uint32".to_owned()),
            2 => Just("uint64".to_owned()),
            2 => Just("bool".to_owned()),
            2 => Just("string".to_owned()),
            2 => Just("bytes".to_owned()),
            1 => Just("string_map".to_owned()),
            1 => Just("int_map".to_owned()),
            1 => Just("nested".to_owned()),
            1 => Just("my_enum".to_owned()),
            1 => Just("optional_enum".to_owned()),
            1 => Just("timestamp".to_owned()),
            1 => Just("duration".to_owned()),
            1 => Just("struct".to_owned()),
            1 => Just("mask".to_owned()),
            1 => Just("list".to_owned()),
            1 => Just("null".to_owned()),
            1 => Just("empty".to_owned()),
            1 => Just("sint32".to_owned()),
            1 => Just("sint64".to_owned()),
            1 => Just("fixed32".to_owned()),
            1 => Just("fixed64".to_owned()),
            1 => Just("sfixed32".to_owned()),
            1 => Just("sfixed64".to_owned()),
        ]
    }

    fn arb_json_value() -> impl Strategy<Value = serde_json::Value> {
        let leaf = prop_oneof![
            Just(serde_json::Value::Null),
            any::<bool>().prop_map(serde_json::Value::from),
            any::<f64>().prop_map(serde_json::Value::from),
            ".*".prop_map(serde_json::Value::from),
        ];
        leaf.prop_recursive(4, 32, 4, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..4).prop_map(serde_json::Value::Array),
                prop::collection::hash_map(arb_json_key(), inner, 0..4)
                    .prop_map(|map| serde_json::Map::from_iter(map).into()),
            ]
        })
    }

    prop::collection::hash_map(arb_json_key(), arb_json_value(), 0..10)
        .prop_map(|map| serde_json::Value::Object(map.into_iter().collect()).to_string())
}
