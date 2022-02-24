use std::{collections::BTreeMap, iter::FromIterator};

use criterion::{criterion_group, criterion_main, Criterion};
use prost::Message;
use prost_reflect::{DynamicMessage, ReflectMessage};
use prost_reflect_tests::WellKnownTypes;

fn sample_wkt() -> WellKnownTypes {
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
}

fn decode_wkt(c: &mut Criterion) {
    let value = sample_wkt().transcode_to_dynamic();

    c.bench_function("decode_wkt", |b| b.iter(|| value.encode_to_vec()));
}

fn encode_wkt(c: &mut Criterion) {
    let value = sample_wkt().encode_to_vec();
    let desc = prost_reflect_tests::test_file_descriptor()
        .get_message_by_name("test.WellKnownTypes")
        .unwrap();

    c.bench_function("encode_wkt", |b| {
        b.iter(|| DynamicMessage::decode(desc.clone(), value.as_slice()))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(500);
    targets = decode_wkt, encode_wkt
}
criterion_main!(benches);
