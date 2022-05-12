use prost_reflect::Syntax;

use crate::test_file_descriptor;

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

    let mut extensions: Vec<_> = test_file_descriptor().all_extensions().collect();
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
        let _ = format!("{:?}", service);
        for method in service.methods() {
            let _ = format!("{:?}", method);
        }
    }

    for file in test_file_descriptor().files() {
        let _ = format!("{:?}", file);
    }

    for message in test_file_descriptor().all_messages() {
        let _ = format!("{:?}", message);
        for field in message.fields() {
            let _ = format!("{:?}", field);
        }
        for oneof in message.oneofs() {
            let _ = format!("{:?}", oneof);
        }
    }

    for enum_ in test_file_descriptor().all_enums() {
        let _ = format!("{:?}", enum_);
        for value in enum_.values() {
            let _ = format!("{:?}", value);
        }
    }

    for extension in test_file_descriptor().all_extensions() {
        let _ = format!("{:?}", extension);
    }
}

#[test]
fn test_raw_getters() {
    // Check none of the debug impls accidentally recurse infinitely
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

        assert!(message.messages().eq(test_file_descriptor()
            .all_messages()
            .filter(|m| m.parent_message() == Some(message.clone()))));
        assert!(message.enums().eq(test_file_descriptor()
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
