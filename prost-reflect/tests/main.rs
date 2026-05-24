use std::{
    env, fs,
    path::{Path, PathBuf},
};

use insta::assert_yaml_snapshot;
use miette::JSONReportHandler;
use prost::Message;
use prost_reflect::{DescriptorError, DescriptorPool, DynamicMessage, ReflectMessage};
use prost_types::FileDescriptorSet;

fn test_data_dir() -> PathBuf {
    PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("tests/data")
}

fn read_file_descriptor_set(path: impl AsRef<Path>) -> DynamicMessage {
    let yaml_bytes = fs::read(test_data_dir().join(path)).unwrap();

    let deserializer = yaml_serde::Deserializer::from_slice(&yaml_bytes);
    DynamicMessage::deserialize(FileDescriptorSet::default().descriptor(), deserializer).unwrap()
}

fn check(name: &str, add_wkt: bool) -> Result<DescriptorPool, DescriptorError> {
    let input = read_file_descriptor_set(format!("{}.yml", name));
    let proto_bytes = input.encode_to_vec();

    let mut pool = if add_wkt {
        FileDescriptorSet::default()
            .descriptor()
            .parent_pool()
            .clone()
    } else {
        DescriptorPool::new()
    };
    pool.decode_file_descriptor_set(proto_bytes.as_slice())?;

    Ok(pool)
}

fn check_ok(name: &str, add_wkt: bool) {
    let pool = check(name, add_wkt).unwrap();
    let set_desc = pool
        .get_message_by_name("google.protobuf.FileDescriptorSet")
        .unwrap_or_else(|| FileDescriptorSet::default().descriptor());

    let mut actual = DynamicMessage::decode(set_desc, pool.encode_to_vec().as_slice()).unwrap();

    if add_wkt {
        actual
            .get_field_by_name_mut("file")
            .unwrap()
            .as_list_mut()
            .unwrap()
            .retain(|f| {
                !f.as_message()
                    .unwrap()
                    .get_field_by_name("package")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .starts_with("google.protobuf")
            });
    }

    insta::with_settings!({ sort_maps => true }, {
        assert_yaml_snapshot!(name, actual);
    });
}

fn check_err(name: &str, add_wkt: bool) {
    let actual_err = check(name, add_wkt).unwrap_err();
    let mut actual_json = String::new();
    JSONReportHandler::new()
        .render_report(&mut actual_json, &actual_err)
        .unwrap();
    let actual = serde_json::from_str::<serde_json::Value>(&actual_json).unwrap();

    insta::with_settings!({ sort_maps => true }, {
        assert_yaml_snapshot!(name, actual);
    });
}

macro_rules! check_ok {
    ($name:ident) => {
        #[test]
        fn $name() {
            check_ok(stringify!($name), false);
        }
    };
    ($name:ident, add_wkt: true) => {
        #[test]
        fn $name() {
            check_ok(stringify!($name), true);
        }
    };
}

macro_rules! check_err {
    ($name:ident) => {
        #[test]
        fn $name() {
            check_err(stringify!($name), false);
        }
    };
    ($name:ident, add_wkt: true) => {
        #[test]
        fn $name() {
            check_err(stringify!($name), true);
        }
    };
}

check_err!(name_conflict_in_imported_files);
check_err!(name_conflict_with_import);
check_err!(name_conflict_package1);
check_err!(name_conflict_package2);
check_ok!(name_conflict_package3);
check_err!(name_conflict_field_camel_case1);
check_err!(name_conflict_field_camel_case2);
check_ok!(name_conflict_field_camel_case3);
check_err!(name_conflict1);
check_err!(name_conflict2);
check_err!(name_conflict3);
check_err!(invalid_message_number1);
check_err!(invalid_message_number2);
check_err!(generate_map_entry_message_name_conflict);
check_err!(generate_group_message_name_conflict);
check_err!(generate_synthetic_oneof_name_conflict);
check_err!(invalid_service_type1);
check_err!(invalid_service_type2);
check_err!(invalid_service_type3);
check_err!(name_resolution1);
check_err!(name_resolution2);
check_ok!(name_resolution3);
check_err!(name_resolution4);
check_ok!(name_resolution5);
check_ok!(name_resolution6);
check_err!(name_collision1);
check_err!(name_collision2);
check_err!(name_collision3);
check_err!(name_collision4);
check_err!(name_collision5);
check_err!(field_default_value1);
check_ok!(field_default_value2);
check_ok!(field_set_json_name);
check_err!(enum_field_invalid_default1);
check_err!(enum_field_invalid_default2);
check_err!(enum_field_invalid_default3);
check_err!(enum_field_invalid_default4);
check_ok!(enum_field_invalid_default5);
check_err!(enum_field_invalid_default6);
check_ok!(enum_field_invalid_default7);
check_err!(enum_field_invalid_default8);
check_ok!(enum_field_invalid_default9);
check_err!(field_default_invalid_type1);
check_err!(field_default_invalid_type2);
check_err!(field_default_invalid_type3);
check_err!(message_field_duplicate_number1);
check_err!(message_field_duplicate_number2);
check_err!(message_reserved_range_overlap_with_field1);
check_err!(message_reserved_range_overlap_with_field2);
check_ok!(message_reserved_range_message_set1);
check_ok!(message_reserved_range_message_set2);
check_ok!(extend_group_field);
check_err!(extend_field_number_not_in_extensions1);
check_err!(extend_field_number_not_in_extensions2);
check_ok!(oneof_group_field);
check_err!(enum_reserved_range_overlap_with_value1);
check_err!(enum_reserved_range_overlap_with_value2);
check_err!(enum_reserved_range_overlap_with_value3);
check_err!(enum_duplicate_number1);
check_err!(enum_duplicate_number2);
check_ok!(enum_duplicate_number3);
check_ok!(enum_default1);
check_err!(enum_default2);
check_ok!(enum_default3);
check_err!(option_unknown_field);
check_err!(option_unknown_extension);
check_err!(option_extension_dependency_not_imported, add_wkt: true);
check_ok!(option_extension_dependency_transitive, add_wkt: true);
check_err!(option_extension_wrong_extendee, add_wkt: true);
check_err!(option_extension_invalid_type);
check_err!(option_already_set);
check_ok!(option_map_entry_set_explicitly);
check_ok!(option_resolution1, add_wkt: true);
check_ok!(option_resolution2, add_wkt: true);
check_ok!(option_resolution3, add_wkt: true);
check_ok!(option_resolution4, add_wkt: true);
check_ok!(option_resolution5, add_wkt: true);
check_ok!(option_resolution6, add_wkt: true);
check_ok!(option_resolution7, add_wkt: true);
check_ok!(option_resolution8, add_wkt: true);
check_ok!(option_resolution9, add_wkt: true);
check_ok!(option_resolution10, add_wkt: true);
check_ok!(option_resolution11, add_wkt: true);
check_ok!(option_resolution12, add_wkt: true);
check_ok!(option_resolution13, add_wkt: true);
check_ok!(option_resolution14, add_wkt: true);
check_ok!(option_resolution15, add_wkt: true);
check_ok!(option_resolution16, add_wkt: true);
check_ok!(option_resolution17, add_wkt: true);
check_err!(option_resolution18, add_wkt: true);
check_ok!(option_resolution19);
check_ok!(option_resolution20, add_wkt: true);
check_ok!(option_resolution21, add_wkt: true);
check_ok!(option_resolution22, add_wkt: true);
check_ok!(option_resolution23, add_wkt: true);
check_ok!(option_resolution24, add_wkt: true);
check_err!(dependency_not_imported);
check_ok!(dependency_resolution_transitive);
check_ok!(dependency_resolution_transitive2);
check_err!(dependency_resolution_transitive3);

// Regression test for https://github.com/andrewhickman/prost-reflect/issues/195
//
// When FieldOptions is present but carries no explicit `packed` entry,
// is_packed() must follow the syntax default: true for proto3, false for proto2.
// This applies to both regular fields (resolve_field) and extension fields
// (resolve_extension).
#[test]
fn is_packed_with_present_but_empty_field_options() {
    use prost::Message as _;
    use prost_types::{
        descriptor_proto::ExtensionRange,
        field_descriptor_proto::{Label, Type},
        DescriptorProto, FieldDescriptorProto, FieldOptions, FileDescriptorProto,
        FileDescriptorSet,
    };

    // Build a pool with one repeated int32 regular field and one repeated int32
    // extension field, both carrying FieldOptions::default() (present but empty,
    // as emitted by protoc when a custom option annotation is present but packed
    // is unset).
    let make_pool = |syntax: &str| {
        let fds = FileDescriptorSet {
            file: vec![FileDescriptorProto {
                name: Some("test.proto".into()),
                syntax: Some(syntax.into()),
                message_type: vec![DescriptorProto {
                    name: Some("Foo".into()),
                    field: vec![FieldDescriptorProto {
                        name: Some("values".into()),
                        number: Some(1),
                        label: Some(Label::Repeated as i32),
                        r#type: Some(Type::Int32 as i32),
                        options: Some(FieldOptions::default()),
                        ..Default::default()
                    }],
                    extension_range: vec![ExtensionRange {
                        start: Some(100),
                        end: Some(200),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                extension: vec![FieldDescriptorProto {
                    name: Some("ext_values".into()),
                    number: Some(100),
                    label: Some(Label::Repeated as i32),
                    r#type: Some(Type::Int32 as i32),
                    extendee: Some(".Foo".into()),
                    options: Some(FieldOptions::default()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
        };
        DescriptorPool::decode(fds.encode_to_vec().as_slice()).unwrap()
    };

    // proto3: packed defaults to true when options is present but packed is absent
    let pool = make_pool("proto3");
    let field = pool
        .get_message_by_name("Foo")
        .unwrap()
        .get_field_by_name("values")
        .unwrap();
    assert!(
        field.is_packed(),
        "proto3 regular field: present-but-empty FieldOptions must be packed"
    );
    let ext = pool.get_extension_by_name("ext_values").unwrap();
    assert!(
        ext.is_packed(),
        "proto3 extension field: present-but-empty FieldOptions must be packed"
    );

    // proto2: packed defaults to false when options is present but packed is absent
    let pool = make_pool("proto2");
    let field = pool
        .get_message_by_name("Foo")
        .unwrap()
        .get_field_by_name("values")
        .unwrap();
    assert!(
        !field.is_packed(),
        "proto2 regular field: present-but-empty FieldOptions must not be packed"
    );
    let ext = pool.get_extension_by_name("ext_values").unwrap();
    assert!(
        !ext.is_packed(),
        "proto2 extension field: present-but-empty FieldOptions must not be packed"
    );
}
