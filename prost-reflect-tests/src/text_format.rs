use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    iter::FromIterator,
};

use proptest::prelude::*;
use prost::Message;
use prost_reflect::{text_format::FormatOptions, DynamicMessage, ReflectMessage, Value};

use crate::{
    proto::{
        contains_group, ComplexType, ContainsGroup, IndexOrder, MessageWithAliasedEnum, Point,
        ScalarArrays, Scalars, WellKnownTypes,
    },
    test_file_descriptor,
};

#[test]
fn fmt_scalars() {
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
        value.to_text_format(),
        r#"double:1.1,float:2.2,int32:3,int64:4,uint32:5,uint64:6,sint32:7,sint64:8,fixed32:9,fixed64:10,sfixed32:11,sfixed64:12,bool:true,string:"5",bytes:"i\246\276m\266\377X""#
    );
    assert_eq!(value.to_text_format_with_options(&FormatOptions::new().pretty(true)), "double: 1.1\nfloat: 2.2\nint32: 3\nint64: 4\nuint32: 5\nuint64: 6\nsint32: 7\nsint64: 8\nfixed32: 9\nfixed64: 10\nsfixed32: 11\nsfixed64: 12\nbool: true\nstring: \"5\"\nbytes: \"i\\246\\276m\\266\\377X\"");
}

#[test]
fn fmt_scalars_float_extrema() {
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

    assert_eq!(inf.to_text_format(), "double:inf,float:inf");
    assert_eq!(
        inf.to_text_format_with_options(&FormatOptions::new().pretty(true)),
        "double: inf\nfloat: inf"
    );
    assert_eq!(neg_inf.to_text_format(), "double:-inf,float:-inf");
    assert_eq!(
        neg_inf.to_text_format_with_options(&FormatOptions::new().pretty(true)),
        "double: -inf\nfloat: -inf"
    );
    assert_eq!(nan.to_text_format(), "double:NaN,float:NaN");
    assert_eq!(
        nan.to_text_format_with_options(&FormatOptions::new().pretty(true)),
        "double: NaN\nfloat: NaN"
    );
}

#[test]
fn fmt_scalars_default() {
    let value = Scalars::default().transcode_to_dynamic();

    assert_eq!(value.to_text_format(), "");
    assert_eq!(
        value.to_text_format_with_options(&FormatOptions::new().pretty(true)),
        ""
    );
}

#[test]
fn fmt_scalar_array() {
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

    assert_eq!(value.to_text_format(), "double:[1.1,2.2],float:[3.3,4.4],int32:[5,-6],int64:[7,-8],uint32:[9,10],uint64:[11,12],sint32:[13,-14],sint64:[15,-16],fixed32:[17,18],fixed64:[19,20],sfixed32:[21,-22],sfixed64:[23,24],bool:[true,false],string:[\"25\",\"26\"],bytes:[\"27\",\"28\"]");
    assert_eq!(value.to_text_format_with_options(&FormatOptions::new().pretty(true)), "double: [1.1, 2.2]\nfloat: [3.3, 4.4]\nint32: [5, -6]\nint64: [7, -8]\nuint32: [9, 10]\nuint64: [11, 12]\nsint32: [13, -14]\nsint64: [15, -16]\nfixed32: [17, 18]\nfixed64: [19, 20]\nsfixed32: [21, -22]\nsfixed64: [23, 24]\nbool: [true, false]\nstring: [\"25\", \"26\"]\nbytes: [\"27\", \"28\"]");
}

#[test]
fn fmt_complex_type() {
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

    assert_eq!(value.to_text_format(), "string_map:[{key:\"1\",value{double:1.1,float:2.2,int32:3}}],int_map:[{key:3,value{sint32:7,sint64:8,fixed32:9}}],nested{sfixed32:11,sfixed64:12,bool:true,string:\"5\",bytes:\"6\"},my_enum:[DEFAULT,FOO,2,BAR,NEG],optional_enum:FOO");
    assert_eq!(value.to_text_format_with_options(&FormatOptions::new().pretty(true)), "string_map: [{\n  key: \"1\"\n  value {\n    double: 1.1\n    float: 2.2\n    int32: 3\n  }\n}]\nint_map: [{\n  key: 3\n  value {\n    sint32: 7\n    sint64: 8\n    fixed32: 9\n  }\n}]\nnested {\n  sfixed32: 11\n  sfixed64: 12\n  bool: true\n  string: \"5\"\n  bytes: \"6\"\n}\nmy_enum: [DEFAULT, FOO, 2, BAR, NEG]\noptional_enum: FOO");
}

#[test]
fn fmt_well_known_types() {
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

    assert_eq!(value.to_text_format(), "timestamp{seconds:63108020,nanos:21000000},duration{seconds:1,nanos:340012},struct{fields:[{key:\"number\",value{number_value:42.0}}]},float{value:42.1},double{value:12.4},int32{value:1},int64{value:-2},uint32{value:3},uint64{value:4},bool{},string{value:\"hello\"},bytes{value:\"hello\"},mask{paths:[\"field_one\",\"field_two.b.d\"]},list{values:[{string_value:\"foo\"},{bool_value:false}]},empty{}");
    assert_eq!(value.to_text_format_with_options(&FormatOptions::new().pretty(true)), "timestamp {\n  seconds: 63108020\n  nanos: 21000000\n}\nduration {\n  seconds: 1\n  nanos: 340012\n}\nstruct {\n  fields: [{\n    key: \"number\"\n    value {\n      number_value: 42.0\n    }\n  }]\n}\nfloat {\n  value: 42.1\n}\ndouble {\n  value: 12.4\n}\nint32 {\n  value: 1\n}\nint64 {\n  value: -2\n}\nuint32 {\n  value: 3\n}\nuint64 {\n  value: 4\n}\nbool {}\nstring {\n  value: \"hello\"\n}\nbytes {\n  value: \"hello\"\n}\nmask {\n  paths: [\"field_one\", \"field_two.b.d\"]\n}\nlist {\n  values: [{\n    string_value: \"foo\"\n  }, {\n    bool_value: false\n  }]\n}\nempty {}");
}

#[test]
fn fmt_empty() {
    let value = ().transcode_to_dynamic();

    assert_eq!(value.to_text_format(), "");
    assert_eq!(
        value.to_text_format_with_options(&FormatOptions::new().pretty(true)),
        ""
    );
}

#[test]
fn fmt_any() {
    let value = transcode_any(&prost_types::Any {
        type_url: "type.googleapis.com/test.Point".to_owned(),
        value: Point {
            longitude: 1,
            latitude: 2,
        }
        .encode_to_vec(),
    });

    assert_eq!(
        value.to_text_format(),
        "[type.googleapis.com/test.Point]{latitude:2,longitude:1}"
    );
    assert_eq!(
        value.to_text_format_with_options(&FormatOptions::new().pretty(true)),
        "[type.googleapis.com/test.Point] {\n  latitude: 2\n  longitude: 1\n}"
    );
}

#[test]
fn fmt_any_wkt() {
    let value = transcode_any(&prost_types::Any {
        type_url: "type.googleapis.com/google.protobuf.Int32Value".to_owned(),
        value: 5i32.encode_to_vec(),
    });

    assert_eq!(
        value.to_text_format(),
        r#"[type.googleapis.com/google.protobuf.Int32Value]{value:5}"#
    );
    assert_eq!(
        value.to_text_format_with_options(&FormatOptions::new().pretty(true)),
        "[type.googleapis.com/google.protobuf.Int32Value] {\n  value: 5\n}"
    );
}

#[test]
fn fmt_any_empty() {
    let value = transcode_any(&prost_types::Any {
        type_url: "type.googleapis.com/google.protobuf.Empty".to_owned(),
        value: vec![],
    });

    assert_eq!(
        value.to_text_format(),
        r#"[type.googleapis.com/google.protobuf.Empty]{}"#
    );
    assert_eq!(
        value.to_text_format_with_options(&FormatOptions::new().pretty(true)),
        "[type.googleapis.com/google.protobuf.Empty] {}"
    );
}

#[test]
fn fmt_any_invalid_type_name() {
    let value = transcode_any(&prost_types::Any {
        type_url: "hello".to_owned(),
        value: vec![],
    });

    assert_eq!(value.to_text_format(), "type_url:\"hello\"",);
    assert_eq!(
        value.to_text_format_with_options(&FormatOptions::new().pretty(true)),
        "type_url: \"hello\"",
    );
}

#[test]
fn fmt_any_type_name_not_found() {
    let value = transcode_any(&prost_types::Any {
        type_url: "type.googleapis.com/NotFound".to_owned(),
        value: vec![],
    });

    assert_eq!(
        value.to_text_format(),
        "type_url:\"type.googleapis.com/NotFound\"",
    );
    assert_eq!(
        value.to_text_format_with_options(&FormatOptions::new().pretty(true)),
        "type_url: \"type.googleapis.com/NotFound\"",
    );
}

#[test]
fn fmt_any_invalid_bytes() {
    let value = transcode_any(&prost_types::Any {
        type_url: "type.googleapis.com/google.protobuf.Empty".to_owned(),
        value: b"hello".to_vec(),
    });

    assert_eq!(
        value.to_text_format(),
        "type_url:\"type.googleapis.com/google.protobuf.Empty\",value:\"hello\"",
    );
    assert_eq!(
        value.to_text_format_with_options(&FormatOptions::new().pretty(true)),
        "type_url: \"type.googleapis.com/google.protobuf.Empty\"\nvalue: \"hello\""
    );
}

#[test]
fn fmt_group() {
    let value = ContainsGroup {
        requiredgroup: Some(contains_group::RequiredGroup {
            a: "bar".to_owned(),
            b: None,
        }),
        optionalgroup: Some(contains_group::OptionalGroup {
            c: "foo".to_owned(),
            d: Some(-5),
        }),
        repeatedgroup: vec![
            contains_group::RepeatedGroup {
                ..Default::default()
            },
            contains_group::RepeatedGroup {
                e: "hello".to_owned(),
                f: Some(10),
            },
        ],
    }
    .transcode_to_dynamic();

    assert_eq!(
        value.to_text_format(),
        "RequiredGroup{a:\"bar\"},OptionalGroup{c:\"foo\",d:-5},RepeatedGroup:[{e:\"\"},{e:\"hello\",f:10}]",
    );
    assert_eq!(
        value.to_text_format_with_options(&FormatOptions::new().pretty(true)),
        "RequiredGroup {\n  a: \"bar\"\n}\nOptionalGroup {\n  c: \"foo\"\n  d: -5\n}\nRepeatedGroup: [{\n  e: \"\"\n}, {\n  e: \"hello\"\n  f: 10\n}]"
    );
}

#[test]
fn fmt_index_order() {
    let value = IndexOrder { a: 1, b: 2, c: 3 }.transcode_to_dynamic();
    assert_eq!(
        value.to_text_format_with_options(
            &FormatOptions::new().print_message_fields_in_index_order(true)
        ),
        "a:1,b:2,c:3"
    );
    assert_eq!(
        value.to_text_format_with_options(
            &FormatOptions::new().print_message_fields_in_index_order(false)
        ),
        "c:3,b:2,a:1"
    );
}

#[test]
fn parse_group() {
    let value = ContainsGroup {
        requiredgroup: Some(contains_group::RequiredGroup {
            a: "bar".to_owned(),
            b: None,
        }),
        optionalgroup: Some(contains_group::OptionalGroup {
            c: "foo".to_owned(),
            d: Some(-5),
        }),
        repeatedgroup: vec![
            contains_group::RepeatedGroup {
                ..Default::default()
            },
            contains_group::RepeatedGroup {
                e: "hello".to_owned(),
                f: Some(10),
            },
        ],
    }
    .transcode_to_dynamic();

    assert_eq!(
        DynamicMessage::parse_text_format(value.descriptor(), "RequiredGroup{a:\"bar\"},OptionalGroup{c:\"foo\",d:-5},RepeatedGroup:[{e:\"\"},{e:\"hello\",f:10}]").unwrap(),
        value,
    );
    assert_eq!(
        DynamicMessage::parse_text_format(value.descriptor(), "requiredgroup{a:\"bar\"}")
            .unwrap_err()
            .to_string(),
        "field 'requiredgroup' not found for message 'test2.ContainsGroup'"
    );
}

#[test]
fn parse_scalars() {
    let value: Scalars = from_text(
        "
        double: 1.1,
        float: 2.2f,
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
        bool: true,
        string: \"5\",
        bytes: \"abc\\366\\xFE\\a\\b\\f\\n\\r\\t\\v\\\\\\'\\\"\\x00\",
    ",
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
            bytes: b"abc\xf6\xfe\x07\x08\x0c\n\r\t\x0b\\'\"\x00".to_vec(),
        },
    );
}

#[test]
fn parse_bool() {
    fn parse(s: &str) -> bool {
        from_text::<Scalars>(s).bool
    }

    assert!(!parse("bool: false"));
    assert!(!parse("bool: False"));
    assert!(!parse("bool: f"));
    assert!(!parse("bool: 0"));
    assert!(!parse("bool: 00"));
    assert!(!parse("bool: 0x0"));
    assert!(parse("bool: true"));
    assert!(parse("bool: True"));
    assert!(parse("bool: t"));
    assert!(parse("bool: t"));
    assert!(parse("bool: 1"));
    assert!(parse("bool: 01"));
    assert!(parse("bool: 0x1"));
}

#[test]
fn parse_scalars_float_extrema() {
    assert_eq!(
        from_text::<Scalars>("double: infinity, float: -INF"),
        Scalars {
            double: f64::INFINITY,
            float: f32::NEG_INFINITY,
            ..Default::default()
        }
    );
    let nan: Scalars = from_text("double: NaN, float: -nan");
    assert!(nan.float.is_nan());
    assert!(nan.double.is_nan());
}

#[test]
fn parse_scalars_empty() {
    let value: Scalars = from_text("");
    assert_eq!(value, Scalars::default());
}

#[test]
fn parse_aliased_enum() {
    let value1: MessageWithAliasedEnum = from_text("aliased: A");
    let value2: MessageWithAliasedEnum = from_text("aliased: B");

    assert_eq!(value1, MessageWithAliasedEnum { aliased: 1 },);
    assert_eq!(value2, MessageWithAliasedEnum { aliased: 1 },);
}

#[test]
fn parse_array() {
    let value: ScalarArrays = from_text("double: [1.1, 2f] , float: 3 ; float: inf");
    assert_eq!(
        value,
        ScalarArrays {
            double: vec![1.1, 2.0],
            float: vec![3.0, f32::INFINITY],
            ..Default::default()
        },
    );
}

#[test]
fn parse_complex_type() {
    let value: ComplexType = from_text(
        "
      string_map: [{
        key: '1';
        value: {
          double: 1.1;
          float: 2.2f;
          int32: 3;
        };
      }, {
        key: \"2\" \"3\";
        value: {
          int64: 4,
          uint32: 5,
          uint64: 6,
        };
      }],
      int_map: [{
        key: 3,
        value: <
            sint32: 7
            sint64: 8
            fixed32: 9
        >
      }, {
        key: 4,
        value: <
            sint64: 8
            fixed32: 9
            fixed64: 10
        >
      }],
      nested {
        sfixed32: 11
        sfixed64: 12
        bool: true
        string: '5'
        bytes: \"6\"
      },
      my_enum: [DEFAULT, FOO]
      my_enum: [2, BAR]
      my_enum: NEG
      optional_enum: FOO
    ",
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
                    "23".to_owned(),
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
fn deserialize_any() {
    let desc = test_file_descriptor()
        .get_message_by_name("google.protobuf.Any")
        .unwrap();
    let value: prost_types::Any = DynamicMessage::parse_text_format(
        desc,
        "[type.googleapis.com/test.Point]: {
            longitude: 1,
            latitude: 2,
        }",
    )
    .unwrap()
    .transcode_to()
    .unwrap();

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
fn parse_error() {
    fn error(s: &str) -> String {
        let desc = test_file_descriptor()
            .get_message_by_name("test.Scalars")
            .unwrap();
        DynamicMessage::parse_text_format(desc, s)
            .unwrap_err()
            .to_string()
    }

    fn any_error(s: &str) -> String {
        let desc = test_file_descriptor()
            .get_message_by_name("google.protobuf.Any")
            .unwrap();
        DynamicMessage::parse_text_format(desc, s)
            .unwrap_err()
            .to_string()
    }

    fn ct_error(s: &str) -> String {
        let desc = test_file_descriptor()
            .get_message_by_name("test.ComplexType")
            .unwrap();
        DynamicMessage::parse_text_format(desc, s)
            .unwrap_err()
            .to_string()
    }

    assert_eq!(
        error(r#"string: -'string'"#),
        "expected a string, but found '-'"
    );
    assert_eq!(
        error(r#"float: -"#),
        "expected a number, but reached end of input"
    );
    assert_eq!(
        error(r#"double"#),
        "expected ':' or a message value, but reached end of input"
    );
    assert_eq!(error(r#"int32: {"#), "expected an integer, but found '{'");
    assert_eq!(
        error(r#"float: {} foo: 10f"#),
        "expected a number, but found '{'"
    );
    assert_eq!(error(r#"uint32: <"#), "expected an integer, but found '<'");
    assert_eq!(
        error(r#"sfixed64 'foo'"#),
        "expected ':' or a message value, but found '\"foo\"'"
    );
    assert_eq!(error(r#"sfixed64 =="#), "invalid token");

    assert_eq!(error(r#"string: "\xFF""#), "string is not valid utf-8");
    assert_eq!(error(r#"int32: 3074457345618258432"#), "expected value to be a signed 32-bit integer, but the value 3074457345618258432 is out of range");
    assert_eq!(
        error(r#"uint32: -7483648"#),
        "expected value to be an unsigned 32-bit integer, but the value -7483648 is out of range"
    );
    assert_eq!(error(r#"int64: -18446744073709551615"#), "expected value to be a signed 64-bit integer, but the value -18446744073709551615 is out of range");
    assert_eq!(
        error(r#"uint64: -1"#),
        "expected value to be an unsigned 64-bit integer, but the value -1 is out of range"
    );

    assert_eq!(error("double: '1'"), "expected a number, but found '\"1\"'");
    assert_eq!(error("double: FOO"), "expected a number, but found 'FOO'");
    assert_eq!(
        error("int32: '1'"),
        "expected an integer, but found '\"1\"'"
    );
    assert_eq!(
        error("fixed64: BAR"),
        "expected an integer, but found 'BAR'"
    );
    assert_eq!(
        error("bool: TRUE"),
        "expected 'true' or 'false', but found 'TRUE'"
    );
    assert_eq!(
        error("bool: tRuE"),
        "expected 'true' or 'false', but found 'tRuE'"
    );
    assert_eq!(error("bool: 3"), "expected 0 or 1, but found '3'");
    assert_eq!(
        error("bool: 99999999999999"),
        "expected 0 or 1, but found '99999999999999'"
    );
    assert_eq!(error("bytes: 1.2"), "expected a string, but found '1.2'");
    assert_eq!(error("bytes: TRUE"), "expected a string, but found 'TRUE'");
    assert_eq!(error("bytes: '\\x'"), "invalid string escape");
    assert_eq!(error("bytes {}"), "expected a string, but found '{'");

    assert_eq!(
        error("notfound: 5"),
        "field 'notfound' not found for message 'test.Scalars'"
    );
    assert_eq!(
        error("string: '5' ; string: '6'"),
        "'string' is already set"
    );
    assert_eq!(
        error("[my . ext]: '5'"),
        "extension 'my.ext' not found for message 'test.Scalars'"
    );

    assert_eq!(
        any_error("[example.com/my.message] <>"),
        "unknown domain 'example.com' for type url"
    );
    assert_eq!(
        any_error("[type.googleapis.com/namespace.NotFound] {}"),
        "message type 'namespace.NotFound' not found"
    );
    assert_eq!(
        any_error("[type.googleapis.com/test.Scalars]: 5"),
        "expected '{' or '<', but found '5'"
    );

    assert_eq!(
        ct_error("my_enum: NOTFOUND"),
        "value 'NOTFOUND' was not found for enum 'test.ComplexType.MyEnum'"
    );
    assert_eq!(
        ct_error("my_enum: 4.2"),
        "expected an enum value, but found '4.2'"
    );
    assert_eq!(
        ct_error("my_enum: 'nope'"),
        "expected an enum value, but found '\"nope\"'"
    );
}

#[test]
fn duplicate_oneof_field() {
    let desc = test_file_descriptor()
        .get_message_by_name("test.MessageWithOneof")
        .unwrap();

    assert_eq!(
        DynamicMessage::parse_text_format(desc.clone(), "oneof_field_1: 'hello', oneof_field_2: 5")
            .unwrap_err()
            .to_string(),
        "a value is already set for oneof 'test_oneof'"
    );

    let d = DynamicMessage::parse_text_format(desc, "oneof_field_1: 'hello'").unwrap();
    assert_eq!(
        d.get_field_by_name("oneof_field_1").unwrap().as_ref(),
        &Value::String("hello".to_owned())
    );
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        .. ProptestConfig::default()
    })]

    #[test]
    fn roundtrip_arb_scalars(message: Scalars) {
        roundtrip_text(&message);
    }

    #[test]
    fn roundtrip_arb_scalar_arrays(message: ScalarArrays) {
        roundtrip_text(&message);
    }

    #[test]
    fn roundtrip_arb_complex_type(message: ComplexType) {
        roundtrip_text(&message);
    }

    #[test]
    fn roundtrip_arb_well_known_types(message: WellKnownTypes) {
        roundtrip_text(&message);
    }

    // TODO Disabled for now due to logos bug: https://github.com/maciejhirsz/logos/issues/255
    // #[test]
    // fn deserialize_error_scalars(s in ".{2,256}") {
    //     let _: Scalars = from_text(&s);
    // }

    // #[test]
    // fn deserialize_error_scalar_arrays(s in ".{2,256}") {
    //     let _: Scalars = from_text(&s);
    // }

    // #[test]
    // fn deserialize_error_complex_type(s in ".{2,256}") {
    //     let _: Scalars = from_text(&s);
    // }

    // #[test]
    // fn deserialize_error_well_known_types(s in ".{2,256}") {
    //     let _: Scalars = from_text(&s);
    // }
}

#[track_caller]
fn from_text<T>(text: &str) -> T
where
    T: PartialEq + Debug + ReflectMessage + Default,
{
    DynamicMessage::parse_text_format(T::default().descriptor(), text)
        .unwrap()
        .transcode_to()
        .unwrap()
}

#[track_caller]
fn roundtrip_text<T>(value: &T)
where
    T: PartialEq + Debug + ReflectMessage + Default,
{
    let dynamic = value.transcode_to_dynamic();
    let text = dynamic.to_text_format();
    let parsed_text: T = from_text(&text);
    assert_eq!(value, &parsed_text);

    let pretty = dynamic.to_text_format_with_options(&FormatOptions::new().pretty(true));
    let parsed_pretty: T = from_text(&pretty);
    assert_eq!(value, &parsed_pretty);
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
