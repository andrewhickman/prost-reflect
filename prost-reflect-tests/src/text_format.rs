use std::{
    collections::{BTreeMap, HashMap},
    iter::FromIterator,
};

use prost::Message;
use prost_reflect::{DynamicMessage, ReflectMessage};

use crate::{
    contains_group, test_file_descriptor, ComplexType, ContainsGroup, Point, ScalarArrays, Scalars,
    WellKnownTypes,
};

#[test]
fn scalars() {
    let value = Scalars {
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
    }
    .transcode_to_dynamic();

    assert_eq!(
        value.to_string(),
        r#"double:1.1,float:2.2,int32:3,int64:4,uint32:5,uint64:6,sint32:7,sint64:8,fixed32:9,fixed64:10,sfixed32:11,sfixed64:12,bool:true,string:"5",bytes:"i\246\276m\266\377X""#
    );
    assert_eq!(value.to_string_pretty(), "double: 1.1\nfloat: 2.2\nint32: 3\nint64: 4\nuint32: 5\nuint64: 6\nsint32: 7\nsint64: 8\nfixed32: 9\nfixed64: 10\nsfixed32: 11\nsfixed64: 12\nbool: true\nstring: \"5\"\nbytes: \"i\\246\\276m\\266\\377X\"");
}

#[test]
fn scalars_float_extrema() {
    let inf = Scalars {
        float: f32::INFINITY,
        double: f64::INFINITY,
        ..Default::default()
    }
    .transcode_to_dynamic();
    let neg_inf = Scalars {
        float: f32::NEG_INFINITY,
        double: f64::NEG_INFINITY,
        ..Default::default()
    }
    .transcode_to_dynamic();
    let nan = Scalars {
        float: f32::NAN,
        double: f64::NAN,
        ..Default::default()
    }
    .transcode_to_dynamic();

    assert_eq!(inf.to_string(), "double:inf,float:inf");
    assert_eq!(inf.to_string_pretty(), "double: inf\nfloat: inf");
    assert_eq!(neg_inf.to_string(), "double:-inf,float:-inf");
    assert_eq!(neg_inf.to_string_pretty(), "double: -inf\nfloat: -inf");
    assert_eq!(nan.to_string(), "double:NaN,float:NaN");
    assert_eq!(nan.to_string_pretty(), "double: NaN\nfloat: NaN");
}

#[test]
fn scalars_default() {
    let value = Scalars::default().transcode_to_dynamic();

    assert_eq!(value.to_string(), "");
    assert_eq!(value.to_string_pretty(), "");
}

#[test]
fn scalar_array() {
    let value = ScalarArrays {
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
    }
    .transcode_to_dynamic();

    assert_eq!(value.to_string(), "double:[1.1,2.2],float:[3.3,4.4],int32:[5,-6],int64:[7,-8],uint32:[9,10],uint64:[11,12],sint32:[13,-14],sint64:[15,-16],fixed32:[17,18],fixed64:[19,20],sfixed32:[21,-22],sfixed64:[23,24],bool:[true,false],string:[\"25\",\"26\"],bytes:[\"27\",\"28\"]");
    assert_eq!(value.to_string_pretty(), "double: [1.1, 2.2]\nfloat: [3.3, 4.4]\nint32: [5, -6]\nint64: [7, -8]\nuint32: [9, 10]\nuint64: [11, 12]\nsint32: [13, -14]\nsint64: [15, -16]\nfixed32: [17, 18]\nfixed64: [19, 20]\nsfixed32: [21, -22]\nsfixed64: [23, 24]\nbool: [true, false]\nstring: [\"25\", \"26\"]\nbytes: [\"27\", \"28\"]");
}

#[test]
fn complex_type() {
    let value = ComplexType {
        string_map: HashMap::from_iter([(
            "1".to_owned(),
            Scalars {
                double: 1.1,
                float: 2.2,
                int32: 3,
                ..Default::default()
            },
        )]),
        int_map: HashMap::from_iter([(
            3,
            Scalars {
                sint32: 7,
                sint64: 8,
                fixed32: 9,
                ..Default::default()
            },
        )]),
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
    .transcode_to_dynamic();

    assert_eq!(value.to_string(), "string_map:[{key:\"1\",value{double:1.1,float:2.2,int32:3}}],int_map:[{key:3,value{sint32:7,sint64:8,fixed32:9}}],nested{sfixed32:11,sfixed64:12,bool:true,string:\"5\",bytes:\"6\"},my_enum:[DEFAULT,FOO,2,BAR,NEG],optional_enum:FOO");
    assert_eq!(value.to_string_pretty(), "string_map: [{\n  key: \"1\"\n  value {\n    double: 1.1\n    float: 2.2\n    int32: 3\n  }\n}]\nint_map: [{\n  key: 3\n  value {\n    sint32: 7\n    sint64: 8\n    fixed32: 9\n  }\n}]\nnested {\n  sfixed32: 11\n  sfixed64: 12\n  bool: true\n  string: \"5\"\n  bytes: \"6\"\n}\nmy_enum: [DEFAULT, FOO, 2, BAR, NEG]\noptional_enum: FOO");
}

#[test]
fn well_known_types() {
    let value = WellKnownTypes {
        timestamp: Some(prost_types::Timestamp {
            seconds: 63_108_020,
            nanos: 21_000_000,
        }),
        duration: Some(prost_types::Duration {
            seconds: 1,
            nanos: 340_012,
        }),
        r#struct: Some(prost_types::Struct {
            fields: BTreeMap::from_iter([(
                "number".to_owned(),
                prost_types::Value {
                    kind: Some(prost_types::value::Kind::NumberValue(42.)),
                },
            )]),
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
    .transcode_to_dynamic();

    assert_eq!(value.to_string(), "timestamp{seconds:63108020,nanos:21000000},duration{seconds:1,nanos:340012},struct{fields:[{key:\"number\",value{number_value:42}}]},float{value:42.1},double{value:12.4},int32{value:1},int64{value:-2},uint32{value:3},uint64{value:4},bool{},string{value:\"hello\"},bytes{value:\"hello\"},mask{paths:[\"field_one\",\"field_two.b.d\"]},list{values:[{string_value:\"foo\"},{bool_value:false}]},empty{}");
    assert_eq!(value.to_string_pretty(), "timestamp {\n  seconds: 63108020\n  nanos: 21000000\n}\nduration {\n  seconds: 1\n  nanos: 340012\n}\nstruct {\n  fields: [{\n    key: \"number\"\n    value {\n      number_value: 42\n    }\n  }]\n}\nfloat {\n  value: 42.1\n}\ndouble {\n  value: 12.4\n}\nint32 {\n  value: 1\n}\nint64 {\n  value: -2\n}\nuint32 {\n  value: 3\n}\nuint64 {\n  value: 4\n}\nbool {}\nstring {\n  value: \"hello\"\n}\nbytes {\n  value: \"hello\"\n}\nmask {\n  paths: [\"field_one\", \"field_two.b.d\"]\n}\nlist {\n  values: [{\n    string_value: \"foo\"\n  }, {\n    bool_value: false\n  }]\n}\nempty {}");
}

#[test]
fn empty() {
    let value = ().transcode_to_dynamic();

    assert_eq!(value.to_string(), "");
    assert_eq!(value.to_string_pretty(), "");
}

#[test]
fn any() {
    let value = transcode_any(&prost_types::Any {
        type_url: "type.googleapis.com/test.Point".to_owned(),
        value: Point {
            longitude: 1,
            latitude: 2,
        }
        .encode_to_vec(),
    });

    assert_eq!(
        value.to_string(),
        "[type.googleapis.com/test.Point]{latitude:2,longitude:1}"
    );
    assert_eq!(
        value.to_string_pretty(),
        "[type.googleapis.com/test.Point] {\n  latitude: 2\n  longitude: 1\n}"
    );
}

#[test]
fn any_wkt() {
    let value = transcode_any(&prost_types::Any {
        type_url: "type.googleapis.com/google.protobuf.Int32Value".to_owned(),
        value: 5i32.encode_to_vec(),
    });

    assert_eq!(
        value.to_string(),
        r#"[type.googleapis.com/google.protobuf.Int32Value]{value:5}"#
    );
    assert_eq!(
        value.to_string_pretty(),
        "[type.googleapis.com/google.protobuf.Int32Value] {\n  value: 5\n}"
    );
}

#[test]
fn any_empty() {
    let value = transcode_any(&prost_types::Any {
        type_url: "type.googleapis.com/google.protobuf.Empty".to_owned(),
        value: vec![],
    });

    assert_eq!(
        value.to_string(),
        r#"[type.googleapis.com/google.protobuf.Empty]{}"#
    );
    assert_eq!(
        value.to_string_pretty(),
        "[type.googleapis.com/google.protobuf.Empty] {}"
    );
}

#[test]
fn any_invalid_type_name() {
    let value = transcode_any(&prost_types::Any {
        type_url: "hello".to_owned(),
        value: vec![],
    });

    assert_eq!(value.to_string(), "type_url:\"hello\"",);
    assert_eq!(value.to_string_pretty(), "type_url: \"hello\"",);
}

#[test]
fn any_type_name_not_found() {
    let value = transcode_any(&prost_types::Any {
        type_url: "type.googleapis.com/NotFound".to_owned(),
        value: vec![],
    });

    assert_eq!(
        value.to_string(),
        "type_url:\"type.googleapis.com/NotFound\"",
    );
    assert_eq!(
        value.to_string_pretty(),
        "type_url: \"type.googleapis.com/NotFound\"",
    );
}

#[test]
fn any_invalid_bytes() {
    let value = transcode_any(&prost_types::Any {
        type_url: "type.googleapis.com/google.protobuf.Empty".to_owned(),
        value: b"hello".to_vec(),
    });

    assert_eq!(
        value.to_string(),
        "type_url:\"type.googleapis.com/google.protobuf.Empty\",value:\"hello\"",
    );
    assert_eq!(
        value.to_string_pretty(),
        "type_url: \"type.googleapis.com/google.protobuf.Empty\"\nvalue: \"hello\""
    );
}

#[test]
fn group() {
    let value = ContainsGroup {
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
    }
    .transcode_to_dynamic();

    assert_eq!(
        value.to_string(),
        "RequiredGroup{a:\"bar\"},OptionalGroup{c:\"foo\",d:-5},RepeatedGroup:[{e:\"\"},{e:\"hello\",f:10}]",
    );
    assert_eq!(
        value.to_string_pretty(),
        "RequiredGroup {\n  a: \"bar\"\n}\nOptionalGroup {\n  c: \"foo\"\n  d: -5\n}\nRepeatedGroup: [{\n  e: \"\"\n}, {\n  e: \"hello\"\n  f: 10\n}]"
    );
}

fn transcode_any(t: &prost_types::Any) -> DynamicMessage {
    // Look up the type in the test pool instead of the global pool used for google types,
    // so we can find the payload.
    let desc = test_file_descriptor()
        .get_message_by_name(t.descriptor().full_name())
        .unwrap();
    let mut message = DynamicMessage::new(desc);
    message.transcode_from(t).unwrap();
    message
}
