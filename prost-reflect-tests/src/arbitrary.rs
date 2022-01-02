use std::fmt::Write;
use std::time::SystemTime;

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
                    write!(path, ".{}", part).unwrap();
                }
                path
            })
            .collect(),
    })
}
