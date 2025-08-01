use std::collections::HashMap;

use prost::{bytes::Bytes, Message};
use prost_reflect::{DescriptorPool, DynamicMessage, MapKey, ReflectMessage, Syntax, Value};

use crate::{
    proto::{self, ComplexType, Scalars},
    test_file_descriptor, DESCRIPTOR_POOL_BYTES,
};

#[test]
fn test_descriptor_methods() {
    let file_desc = test_file_descriptor()
        .get_file_by_name("desc.proto")
        .unwrap();
    assert_eq!(file_desc.name(), "desc.proto");
    assert_eq!(file_desc.package_name(), "my.package");
    assert_eq!(file_desc.syntax(), Syntax::Proto3);

    let message_desc = test_file_descriptor()
        .get_message_by_name("my.package.MyMessage")
        .unwrap();
    assert_eq!(message_desc.name(), "MyMessage");
    assert_eq!(message_desc.full_name(), "my.package.MyMessage");
    assert_eq!(message_desc.parent_file(), file_desc);
    assert_eq!(message_desc.parent_message(), None);
    assert_eq!(message_desc.package_name(), "my.package");
    assert_eq!(
        message_desc.reserved_ranges().flatten().collect::<Vec<_>>(),
        vec![2, 15, 9, 10, 11]
    );
    assert_eq!(
        message_desc.reserved_names().collect::<Vec<_>>(),
        vec!["foo", "bar"]
    );
    assert_eq!(message_desc.extension_ranges().count(), 0,);

    let field_desc = message_desc.get_field_by_name("my_field").unwrap();
    assert_eq!(field_desc.name(), "my_field");
    assert_eq!(field_desc.full_name(), "my.package.MyMessage.my_field");

    let nested_message_desc = test_file_descriptor()
        .get_message_by_name("my.package.MyMessage.MyNestedMessage")
        .unwrap();
    assert_eq!(nested_message_desc.name(), "MyNestedMessage");
    assert_eq!(
        nested_message_desc.full_name(),
        "my.package.MyMessage.MyNestedMessage"
    );
    assert_eq!(
        nested_message_desc.parent_message(),
        Some(message_desc.clone())
    );
    assert_eq!(nested_message_desc.package_name(), "my.package");

    let enum_desc = test_file_descriptor()
        .get_enum_by_name("my.package.MyEnum")
        .unwrap();
    assert_eq!(enum_desc.name(), "MyEnum");
    assert_eq!(enum_desc.full_name(), "my.package.MyEnum");
    assert_eq!(enum_desc.parent_message(), None);
    assert_eq!(enum_desc.package_name(), "my.package");
    assert_eq!(
        enum_desc.reserved_ranges().flatten().collect::<Vec<_>>(),
        vec![-2, 15, 9, 10, 11]
    );
    assert_eq!(
        enum_desc.reserved_names().collect::<Vec<_>>(),
        vec!["FOO", "BAR"]
    );

    let enum_value_desc = enum_desc.get_value_by_name("MY_VALUE").unwrap();
    assert_eq!(enum_value_desc.name(), "MY_VALUE");
    assert_eq!(enum_value_desc.full_name(), "my.package.MY_VALUE");

    let nested_enum_desc = test_file_descriptor()
        .get_enum_by_name("my.package.MyMessage.MyNestedEnum")
        .unwrap();
    assert_eq!(nested_enum_desc.name(), "MyNestedEnum");
    assert_eq!(
        nested_enum_desc.full_name(),
        "my.package.MyMessage.MyNestedEnum"
    );
    assert_eq!(nested_enum_desc.parent_message(), Some(message_desc));
    assert_eq!(nested_enum_desc.package_name(), "my.package");

    let service_desc = test_file_descriptor()
        .services()
        .find(|s| s.full_name() == "my.package.MyService")
        .unwrap();
    assert_eq!(service_desc.name(), "MyService");
    assert_eq!(service_desc.full_name(), "my.package.MyService");
    assert_eq!(service_desc.package_name(), "my.package");

    let method_desc = service_desc
        .methods()
        .find(|m| m.name() == "MyMethod")
        .unwrap();
    assert_eq!(method_desc.name(), "MyMethod");
    assert_eq!(method_desc.full_name(), "my.package.MyService.MyMethod");
}

#[test]
fn test_descriptor_methods_proto2() {
    let file_desc = test_file_descriptor()
        .get_file_by_name("desc2.proto")
        .unwrap();
    assert_eq!(file_desc.name(), "desc2.proto");
    assert_eq!(file_desc.package_name(), "my.package2");
    assert_eq!(file_desc.syntax(), Syntax::Proto2);

    let message_desc = test_file_descriptor()
        .get_message_by_name("my.package2.MyMessage")
        .unwrap();
    assert_eq!(message_desc.name(), "MyMessage");
    assert_eq!(message_desc.full_name(), "my.package2.MyMessage");
    assert_eq!(message_desc.parent_file(), file_desc);
    assert_eq!(message_desc.parent_message(), None);
    assert_eq!(message_desc.package_name(), "my.package2");
    assert_eq!(
        message_desc
            .extension_ranges()
            .flatten()
            .collect::<Vec<_>>(),
        vec![100, 110, 111, 112, 113, 114, 115],
    );

    let mut extensions: Vec<_> = test_file_descriptor()
        .all_extensions()
        .filter(|ext| ext.parent_file() == file_desc)
        .collect();
    extensions.sort_by_key(|e| e.full_name().to_owned());
    assert_eq!(extensions.len(), 3);

    assert_eq!(
        extensions[0].full_name(),
        "my.package2.MyMessage.in_extendee"
    );
    assert_eq!(
        extensions[0].parent_message().unwrap().full_name(),
        "my.package2.MyMessage"
    );
    assert_eq!(extensions[0].parent_file(), file_desc);
    assert_eq!(
        extensions[0].containing_message().full_name(),
        "my.package2.MyMessage"
    );
    assert_eq!(
        extensions[0].json_name(),
        "[my.package2.MyMessage.in_extendee]"
    );

    assert_eq!(
        extensions[1].full_name(),
        "my.package2.OtherMessage.in_other"
    );
    assert_eq!(
        extensions[1].parent_message().unwrap().full_name(),
        "my.package2.OtherMessage"
    );
    assert_eq!(extensions[1].parent_file(), file_desc);
    assert_eq!(
        extensions[1].containing_message().full_name(),
        "my.package2.MyMessage"
    );
    assert_eq!(
        extensions[1].json_name(),
        "[my.package2.OtherMessage.in_other]"
    );

    assert_eq!(extensions[2].full_name(), "my.package2.in_file");
    assert!(extensions[2].parent_message().is_none());
    assert_eq!(extensions[2].parent_file(), file_desc);
    assert_eq!(
        extensions[2].containing_message().full_name(),
        "my.package2.MyMessage"
    );
    assert_eq!(extensions[2].json_name(), "[my.package2.in_file]");
}

#[test]
fn test_descriptor_names_no_package() {
    let message_desc = test_file_descriptor()
        .get_message_by_name("MyMessage")
        .unwrap();
    assert_eq!(message_desc.name(), "MyMessage");
    assert_eq!(message_desc.full_name(), "MyMessage");
    assert_eq!(message_desc.parent_message(), None);
    assert_eq!(message_desc.package_name(), "");

    let field_desc = message_desc.get_field_by_name("my_field").unwrap();
    assert_eq!(field_desc.name(), "my_field");
    assert_eq!(field_desc.full_name(), "MyMessage.my_field");

    let nested_message_desc = test_file_descriptor()
        .get_message_by_name("MyMessage.MyNestedMessage")
        .unwrap();
    assert_eq!(nested_message_desc.name(), "MyNestedMessage");
    assert_eq!(nested_message_desc.full_name(), "MyMessage.MyNestedMessage");
    assert_eq!(
        nested_message_desc.parent_message(),
        Some(message_desc.clone())
    );
    assert_eq!(nested_message_desc.package_name(), "");

    let enum_desc = test_file_descriptor().get_enum_by_name("MyEnum").unwrap();
    assert_eq!(enum_desc.name(), "MyEnum");
    assert_eq!(enum_desc.full_name(), "MyEnum");
    assert_eq!(enum_desc.parent_message(), None);
    assert_eq!(enum_desc.package_name(), "");

    let enum_value_desc = enum_desc.get_value_by_name("MY_VALUE").unwrap();
    assert_eq!(enum_value_desc.name(), "MY_VALUE");
    assert_eq!(enum_value_desc.full_name(), "MY_VALUE");

    let nested_enum_desc = test_file_descriptor()
        .get_enum_by_name("MyMessage.MyNestedEnum")
        .unwrap();
    assert_eq!(nested_enum_desc.name(), "MyNestedEnum");
    assert_eq!(nested_enum_desc.full_name(), "MyMessage.MyNestedEnum");
    assert_eq!(nested_enum_desc.parent_message(), Some(message_desc));
    assert_eq!(nested_enum_desc.package_name(), "");

    let service_desc = test_file_descriptor()
        .services()
        .find(|s| s.full_name() == "MyService")
        .unwrap();
    assert_eq!(service_desc.name(), "MyService");
    assert_eq!(service_desc.full_name(), "MyService");
    assert_eq!(service_desc.package_name(), "");

    let method_desc = service_desc
        .methods()
        .find(|m| m.name() == "MyMethod")
        .unwrap();
    assert_eq!(method_desc.name(), "MyMethod");
    assert_eq!(method_desc.full_name(), "MyService.MyMethod");
}

#[test]
fn test_debug_impls() {
    // Check none of the debug impls accidentally recurse infinitely
    let _ = format!("{:?}", test_file_descriptor());

    for service in test_file_descriptor().services() {
        let _ = format!("{service:?}");
        for method in service.methods() {
            let _ = format!("{method:?}");
        }
    }

    for file in test_file_descriptor().files() {
        let _ = format!("{file:?}");
    }

    for message in test_file_descriptor().all_messages() {
        let _ = format!("{message:?}");
        for field in message.fields() {
            let _ = format!("{field:?}");
        }
        for oneof in message.oneofs() {
            let _ = format!("{oneof:?}");
        }
    }

    for enum_ in test_file_descriptor().all_enums() {
        let _ = format!("{enum_:?}");
        for value in enum_.values() {
            let _ = format!("{value:?}");
        }
    }

    for extension in test_file_descriptor().all_extensions() {
        let _ = format!("{extension:?}");
    }
}

#[test]
fn test_raw_getters() {
    let _ = format!("{:?}", test_file_descriptor());

    for file in test_file_descriptor().files() {
        assert_eq!(file.file_descriptor_proto().name(), file.name());

        assert!(file.messages().eq(test_file_descriptor()
            .all_messages()
            .filter(|m| m.parent_message().is_none() && m.parent_file() == file)));
        assert!(file.enums().eq(test_file_descriptor()
            .all_enums()
            .filter(|m| m.parent_message().is_none() && m.parent_file() == file)));
        assert!(file.extensions().eq(test_file_descriptor()
            .all_extensions()
            .filter(|m| m.parent_message().is_none() && m.parent_file() == file)));
        assert!(file.services().eq(test_file_descriptor()
            .services()
            .filter(|m| m.parent_file() == file)));
    }

    for service in test_file_descriptor().services() {
        assert_eq!(service.service_descriptor_proto().name(), service.name());
        for method in service.methods() {
            assert_eq!(method.method_descriptor_proto().name(), method.name());
        }
    }

    for message in test_file_descriptor().all_messages() {
        assert_eq!(message.descriptor_proto().name(), message.name());
        for field in message.fields() {
            assert_eq!(field.field_descriptor_proto().name(), field.name());
        }
        for oneof in message.oneofs() {
            assert_eq!(oneof.oneof_descriptor_proto().name(), oneof.name());
        }
        assert!(message.extensions().eq(test_file_descriptor()
            .all_extensions()
            .filter(|m| m.containing_message() == message)));

        assert!(message.child_messages().eq(test_file_descriptor()
            .all_messages()
            .filter(|m| m.parent_message() == Some(message.clone()))));
        assert!(message.child_enums().eq(test_file_descriptor()
            .all_enums()
            .filter(|m| m.parent_message() == Some(message.clone()))));
        assert!(message.child_extensions().eq(test_file_descriptor()
            .all_extensions()
            .filter(|m| m.parent_message() == Some(message.clone()))));
    }

    for enum_ in test_file_descriptor().all_enums() {
        assert_eq!(enum_.enum_descriptor_proto().name(), enum_.name());
        for value in enum_.values() {
            assert_eq!(value.enum_value_descriptor_proto().name(), value.name());
        }
    }

    for extension in test_file_descriptor().all_extensions() {
        assert_eq!(extension.field_descriptor_proto().name(), extension.name());
    }
}

#[test]
fn descriptor_pool_add_individual_files() {
    let original = test_file_descriptor();

    let mut roundtripped = DescriptorPool::new();
    // These should be sorted into topological order by the protobuf compiler.
    for file in original.file_descriptor_protos() {
        roundtripped
            .add_file_descriptor_proto(file.clone())
            .unwrap();
    }

    assert_ne!(original, roundtripped);
    assert!(original
        .all_messages()
        .map(|m| m.full_name().to_owned())
        .eq(roundtripped
            .all_messages()
            .map(|m| m.full_name().to_owned())));
    let message_desc = roundtripped
        .get_message_by_name("my.package.MyMessage")
        .unwrap();
    assert_eq!(message_desc.name(), "MyMessage");
    assert_eq!(message_desc.full_name(), "my.package.MyMessage");
    assert_eq!(message_desc.parent_pool(), &roundtripped);
    assert_eq!(message_desc.parent_message(), None);
    assert_eq!(message_desc.package_name(), "my.package");
}

#[test]
fn test_enum_alias() {
    let enum_desc = test_file_descriptor()
        .get_enum_by_name("test.EnumWithAlias")
        .unwrap();
    assert_eq!(enum_desc.name(), "EnumWithAlias");
    assert_eq!(enum_desc.full_name(), "test.EnumWithAlias");
    assert_eq!(enum_desc.parent_message(), None);
    assert_eq!(enum_desc.package_name(), "test");

    assert_eq!(enum_desc.get_value_by_name("FOO").unwrap().number(), 0);
    assert_eq!(enum_desc.get_value_by_name("BAR").unwrap().number(), 0);
    assert_eq!(enum_desc.get_value_by_name("A").unwrap().number(), 1);
    assert_eq!(enum_desc.get_value_by_name("B").unwrap().number(), 1);
    assert_eq!(enum_desc.get_value_by_name("C").unwrap().number(), 1);
    assert_eq!(enum_desc.get_value_by_name("TWO").unwrap().number(), 2);

    assert_eq!(enum_desc.get_value(0).unwrap().number(), 0);
    assert!(matches!(
        enum_desc.get_value(0).unwrap().name(),
        "FOO" | "BAR"
    ));
    assert_eq!(enum_desc.get_value(1).unwrap().number(), 1);
    assert!(matches!(
        enum_desc.get_value(1).unwrap().name(),
        "A" | "B" | "C"
    ));
    assert_eq!(enum_desc.get_value(2).unwrap().number(), 2);
    assert_eq!(enum_desc.get_value(2).unwrap().name(), "TWO");
    assert_eq!(enum_desc.get_value(3), None);
}

#[test]
fn test_get_extension() {
    let file_descriptor_set = test_file_descriptor()
        .get_message_by_name("google.protobuf.FileDescriptorSet")
        .unwrap();

    let mut dynamic_message = prost_reflect::DynamicMessage::new(file_descriptor_set);
    dynamic_message.merge(DESCRIPTOR_POOL_BYTES).unwrap();

    let extension = test_file_descriptor()
        .get_message_by_name("google.protobuf.EnumValueOptions")
        .unwrap()
        .get_extension_by_full_name("demo.len")
        .unwrap();

    assert_eq!(
        dynamic_message
            .get_field_by_name("file")
            .unwrap()
            .as_list()
            .unwrap()
            .iter()
            .map(|f| f.as_message().unwrap())
            .find(|f| f.get_field_by_name("name").unwrap().as_str() == Some("ext.proto"))
            .unwrap()
            .get_field_by_name("enum_type")
            .unwrap()
            .as_list()
            .unwrap()[0]
            .as_message()
            .unwrap()
            .get_field_by_name("value")
            .unwrap()
            .as_list()
            .unwrap()[1]
            .as_message()
            .unwrap()
            .get_field_by_name("options")
            .unwrap()
            .as_message()
            .unwrap()
            .get_extension(&extension)
            .as_ref(),
        &Value::U32(1)
    );

    let e = test_file_descriptor().get_enum_by_name("demo.Foo").unwrap();
    assert!(e.get_value(0).unwrap().options().has_extension(&extension));
    assert_eq!(
        e.get_value(0)
            .unwrap()
            .options()
            .get_extension(&extension)
            .as_ref(),
        &Value::U32(0)
    );
    assert!(e.get_value(1).unwrap().options().has_extension(&extension));
    assert_eq!(
        e.get_value(1)
            .unwrap()
            .options()
            .get_extension(&extension)
            .as_ref(),
        &Value::U32(1)
    );
    assert!(e.get_value(2).unwrap().options().has_extension(&extension));
    assert_eq!(
        e.get_value(2)
            .unwrap()
            .options()
            .get_extension(&extension)
            .as_ref(),
        &Value::U32(2)
    );
}

#[test]
fn test_file_extension_options() {
    let pool = test_file_descriptor();

    let file = pool.get_file_by_name("options.proto").unwrap();
    let file_ext = pool.get_extension_by_name("custom.options.file").unwrap();
    assert_eq!(
        file.options().get_extension(&file_ext).as_ref(),
        &Value::I32(-1)
    );
}

#[test]
fn test_message_extension_options() {
    let pool = test_file_descriptor();

    let message = pool
        .get_message_by_name("custom.options.Aggregate")
        .unwrap();
    let message_ext = pool
        .get_extension_by_name("custom.options.message")
        .unwrap();
    assert_eq!(
        message.options().get_extension(&message_ext).as_ref(),
        &Value::String("abc".into())
    );

    let field = message.get_field_by_name("a").unwrap();
    let field_ext = pool.get_extension_by_name("custom.options.field").unwrap();
    assert_eq!(
        field.options().get_extension(&field_ext).as_ref(),
        &Value::Bytes(b"\x08".as_ref().into())
    );

    let oneof = message.oneofs().find(|o| o.name() == "O").unwrap();
    let oneof_ext = pool.get_extension_by_name("custom.options.oneof").unwrap();
    assert_eq!(
        oneof.options().get_extension(&oneof_ext).as_ref(),
        &Value::List(vec![Value::F32(5.5), Value::F32(-5.0), Value::F32(5.0)]),
    );
}

#[test]
fn test_extension_extension_options() {
    let pool = test_file_descriptor();

    let ext = pool.get_extension_by_name("custom.options.field").unwrap();
    assert_eq!(
        ext.options().get_extension(&ext).as_ref(),
        &Value::Bytes("extension".into())
    );
}

#[test]
fn test_service_extension_options() {
    let pool = test_file_descriptor();

    let service = pool.get_service_by_name("custom.options.Service").unwrap();
    let service_ext = pool
        .get_extension_by_name("custom.options.service")
        .unwrap();
    assert_eq!(
        service.options().get_extension(&service_ext).as_ref(),
        &Value::Bool(true)
    );

    let method = service.methods().next().unwrap();
    let method_ext = pool.get_extension_by_name("custom.options.method").unwrap();
    assert_eq!(
        method.options().get_extension(&method_ext).as_ref(),
        &Value::U64(6)
    );
}

#[test]
fn test_enum_extension_options() {
    let pool = test_file_descriptor();

    let enum_ = pool.get_enum_by_name("custom.options.Enum").unwrap();
    let enum_ext = pool.get_extension_by_name("custom.options.enum").unwrap();
    assert_eq!(
        enum_.options().get_extension(&enum_ext).as_ref(),
        &Value::Message(
            proto::options::Aggregate {
                a: 32,
                o: Some(proto::options::aggregate::O::B("abc".into()))
            }
            .transcode_to_dynamic()
        ),
    );

    let value = enum_.get_value_by_name("VALUE").unwrap();
    let value_ext = pool.get_extension_by_name("custom.options.value").unwrap();
    assert_eq!(
        value.options().get_extension(&value_ext).as_ref(),
        &Value::EnumNumber(1)
    );
}

#[test]
fn message_default_value_presence() {
    let mut message = Scalars::default().transcode_to_dynamic();
    let field = message.descriptor().get_field_by_name("int32").unwrap();

    assert_eq!(message.fields().count(), 0);
    assert!(!message.has_field(&field));
    assert_eq!(message.get_field(&field).as_ref(), &Value::I32(0));

    message.set_field(&field, Value::I32(0));

    assert_eq!(message.fields().count(), 0);
    assert!(!message.has_field(&field));
    assert_eq!(message.get_field(&field).as_ref(), &Value::I32(0));

    message.get_field_mut(&field);

    assert_eq!(message.fields().count(), 0);
    assert!(!message.has_field(&field));
    assert_eq!(message.get_field(&field).as_ref(), &Value::I32(0));
}

#[test]
fn message_list_fields_scalars() {
    let message = Scalars {
        double: 0.0,
        float: 2.2,
        int32: 3,
        int64: 0,
        uint32: 5,
        uint64: 6,
        sint32: 7,
        sint64: 0,
        fixed32: 9,
        fixed64: 0,
        sfixed32: 11,
        sfixed64: 12,
        r#bool: false,
        string: "5".to_owned(),
        bytes: b"6".to_vec(),
    }
    .transcode_to_dynamic();

    assert_eq!(
        message.fields().collect::<Vec<_>>(),
        vec![
            (
                message.descriptor().get_field_by_name("float").unwrap(),
                &Value::F32(2.2)
            ),
            (
                message.descriptor().get_field_by_name("int32").unwrap(),
                &Value::I32(3)
            ),
            (
                message.descriptor().get_field_by_name("uint32").unwrap(),
                &Value::U32(5)
            ),
            (
                message.descriptor().get_field_by_name("uint64").unwrap(),
                &Value::U64(6)
            ),
            (
                message.descriptor().get_field_by_name("sint32").unwrap(),
                &Value::I32(7)
            ),
            (
                message.descriptor().get_field_by_name("fixed32").unwrap(),
                &Value::U32(9)
            ),
            (
                message.descriptor().get_field_by_name("sfixed32").unwrap(),
                &Value::I32(11)
            ),
            (
                message.descriptor().get_field_by_name("sfixed64").unwrap(),
                &Value::I64(12)
            ),
            (
                message.descriptor().get_field_by_name("string").unwrap(),
                &Value::String("5".to_owned())
            ),
            (
                message.descriptor().get_field_by_name("bytes").unwrap(),
                &Value::Bytes(Bytes::from_static(b"6"))
            ),
        ]
    );
}

#[test]
fn message_list_extensions() {
    let message_desc = test_file_descriptor()
        .get_message_by_name("my.package2.MyMessage")
        .unwrap();

    let field_desc = message_desc.get_field_by_name("int").unwrap();
    let extension_desc = message_desc.get_extension(113).unwrap();

    let mut dynamic_message = DynamicMessage::new(message_desc.clone());

    assert_eq!(dynamic_message.fields().count(), 0);
    assert_eq!(dynamic_message.extensions().count(), 0);
    assert_eq!(dynamic_message.fields_mut().count(), 0);
    assert_eq!(dynamic_message.extensions_mut().count(), 0);

    dynamic_message.set_field(&field_desc, Value::I32(0));
    dynamic_message.set_extension(&extension_desc, Value::F64(42.0));

    assert!(dynamic_message
        .fields()
        .eq([(field_desc.clone(), &Value::I32(0))]));
    assert!(dynamic_message
        .extensions()
        .eq([(extension_desc.clone(), &Value::F64(42.0))]));

    assert!(dynamic_message
        .fields_mut()
        .eq([(field_desc, &mut Value::I32(0))]));
    assert!(dynamic_message
        .extensions_mut()
        .eq([(extension_desc, &mut Value::F64(42.0))]));
}

#[test]
fn message_take_field() {
    let mut message = ComplexType {
        string_map: HashMap::from_iter([("foo".to_owned(), Scalars::default())]),
        nested: Some(Scalars {
            int32: 3,
            ..Default::default()
        }),
        optional_enum: 3,
        ..Default::default()
    }
    .transcode_to_dynamic();

    let map = Value::Map(HashMap::from_iter([(
        MapKey::String("foo".to_owned()),
        Value::Message(Scalars::default().transcode_to_dynamic()),
    )]));
    let nested = Value::Message(
        Scalars {
            int32: 3,
            ..Default::default()
        }
        .transcode_to_dynamic(),
    );
    let num = Value::EnumNumber(3);

    assert!(message.fields().eq([
        (
            message
                .descriptor()
                .get_field_by_name("string_map")
                .unwrap(),
            &map
        ),
        (
            message.descriptor().get_field_by_name("nested").unwrap(),
            &nested
        ),
        (
            message
                .descriptor()
                .get_field_by_name("optional_enum")
                .unwrap(),
            &num
        )
    ]));

    assert_eq!(message.take_field_by_name("int_map"), None);
    assert_eq!(message.take_field_by_name("notfound"), None);
    assert_eq!(message.take_field_by_name("string_map"), Some(map));
    assert_eq!(message.take_field_by_name("nested"), Some(nested));
    assert_eq!(message.take_field_by_name("optional_enum"), Some(num));

    assert!(message.fields().eq([]));
}

#[test]
fn message_take_fields() {
    let message_desc = test_file_descriptor()
        .get_message_by_name("my.package2.MyMessage")
        .unwrap();

    let field_desc = message_desc.get_field_by_name("int").unwrap();
    let extension_desc = message_desc.get_extension(113).unwrap();

    let mut dynamic_message = DynamicMessage::new(message_desc.clone());

    assert_eq!(dynamic_message.fields().count(), 0);
    assert_eq!(dynamic_message.extensions().count(), 0);
    assert_eq!(dynamic_message.fields_mut().count(), 0);
    assert_eq!(dynamic_message.extensions_mut().count(), 0);

    dynamic_message.set_field(&field_desc, Value::I32(0));
    dynamic_message.set_extension(&extension_desc, Value::F64(42.0));

    assert!(dynamic_message
        .take_fields()
        .eq([(field_desc, Value::I32(0))]));
    assert!(dynamic_message
        .take_extensions()
        .eq([(extension_desc, Value::F64(42.0))]));

    assert_eq!(dynamic_message.fields().count(), 0);
    assert_eq!(dynamic_message.extensions().count(), 0);
    assert_eq!(dynamic_message.fields_mut().count(), 0);
    assert_eq!(dynamic_message.extensions_mut().count(), 0);
}

#[test]
fn oneof_not_synthetic() {
    let message_desc = test_file_descriptor()
        .get_message_by_name("test.MessageWithOneof")
        .unwrap();

    assert_eq!(message_desc.oneofs().len(), 1);

    let oneof_desc = message_desc.oneofs().next().unwrap();
    assert_eq!(oneof_desc.name(), "test_oneof");
    assert!(!oneof_desc.is_synthetic());
}

#[test]
fn proto3_optional_field() {
    let message_desc = test_file_descriptor()
        .get_message_by_name("test.MessageWithOptionalEnum")
        .unwrap();
    let field_desc = message_desc.get_field_by_name("optional_enum").unwrap();
    let oneof_desc = field_desc.containing_oneof().unwrap();

    assert!(field_desc.supports_presence());
    assert_eq!(oneof_desc.name(), "_optional_enum");
    assert!(oneof_desc.is_synthetic());
    assert!(oneof_desc.fields().eq([field_desc.clone()]));

    assert_eq!(message_desc.oneofs().len(), 1);
    assert!(message_desc.oneofs().eq([oneof_desc.clone()]));
}
