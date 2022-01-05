use prost_types::{
    field_descriptor_proto::{Label, Type},
    DescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileDescriptorSet,
    MethodDescriptorProto, ServiceDescriptorProto,
};

use crate::FileDescriptor;

#[test]
fn resolve_service_name() {
    let file_descriptor_set = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            package: Some("my.package".to_owned()),
            syntax: Some("proto3".to_owned()),
            service: vec![ServiceDescriptorProto {
                name: Some("MyService".to_owned()),
                method: vec![MethodDescriptorProto {
                    name: Some("my_method".to_owned()),
                    input_type: Some("MyMessage".to_owned()),
                    output_type: Some(".my.package.MyMessage".to_owned()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            message_type: vec![DescriptorProto {
                name: Some("MyMessage".to_owned()),
                ..Default::default()
            }],
            ..Default::default()
        }],
    };

    let file_descriptor = FileDescriptor::new(file_descriptor_set).unwrap();
    let service = file_descriptor.services().next().unwrap();
    let method = service.methods().next().unwrap();
    assert_eq!(method.input().full_name(), "my.package.MyMessage");
    assert_eq!(method.output().full_name(), "my.package.MyMessage");
}

#[test]
fn resolve_service_name_other_package() {
    let file_descriptor_set = FileDescriptorSet {
        file: vec![
            FileDescriptorProto {
                package: Some("my.package".to_owned()),
                syntax: Some("proto3".to_owned()),
                service: vec![ServiceDescriptorProto {
                    name: Some("MyService".to_owned()),
                    method: vec![MethodDescriptorProto {
                        name: Some("my_method".to_owned()),
                        input_type: Some("other.package.MyMessage".to_owned()),
                        output_type: Some(".other.package.MyMessage".to_owned()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            },
            FileDescriptorProto {
                package: Some("other.package".to_owned()),
                syntax: Some("proto3".to_owned()),
                message_type: vec![DescriptorProto {
                    name: Some("MyMessage".to_owned()),
                    ..Default::default()
                }],
                ..Default::default()
            },
        ],
    };

    let file_descriptor = FileDescriptor::new(file_descriptor_set).unwrap();
    let service = file_descriptor.services().next().unwrap();
    let method = service.methods().next().unwrap();
    assert_eq!(method.input().full_name(), "other.package.MyMessage");
    assert_eq!(method.output().full_name(), "other.package.MyMessage");
}

#[test]
fn resolve_message_name() {
    let file_descriptor_set = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            package: Some("my.package".to_owned()),
            syntax: Some("proto3".to_owned()),
            message_type: vec![
                DescriptorProto {
                    name: Some("MyFieldMessage".to_owned()),
                    ..Default::default()
                },
                DescriptorProto {
                    name: Some("MyMessage".to_owned()),
                    field: vec![FieldDescriptorProto {
                        name: Some("my_field".to_owned()),
                        number: Some(1),
                        label: Some(Label::Optional as i32),
                        r#type: Some(Type::Message as i32),
                        type_name: Some("MyFieldMessage".to_owned()),
                        json_name: Some("myfield".to_owned()),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
    };

    let file_descriptor = FileDescriptor::new(file_descriptor_set).unwrap();
    let message = file_descriptor
        .get_message_by_name("my.package.MyMessage")
        .unwrap();
    let field = message.get_field_by_name("my_field").unwrap();
    assert_eq!(
        field.kind().as_message().unwrap().full_name(),
        "my.package.MyFieldMessage"
    );
}

#[test]
fn resolve_message_name_nested() {
    let file_descriptor_set = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            package: Some("my.package".to_owned()),
            syntax: Some("proto3".to_owned()),
            message_type: vec![DescriptorProto {
                name: Some("MyMessage".to_owned()),
                field: vec![FieldDescriptorProto {
                    name: Some("my_field".to_owned()),
                    number: Some(1),
                    label: Some(Label::Optional as i32),
                    r#type: Some(Type::Message as i32),
                    type_name: Some("MyFieldMessage".to_owned()),
                    json_name: Some("myfield".to_owned()),
                    ..Default::default()
                }],
                nested_type: vec![DescriptorProto {
                    name: Some("MyFieldMessage".to_owned()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }],
    };

    let file_descriptor = FileDescriptor::new(file_descriptor_set).unwrap();
    let message = file_descriptor
        .get_message_by_name("my.package.MyMessage")
        .unwrap();
    let field = message.get_field_by_name("my_field").unwrap();
    assert_eq!(
        field.kind().as_message().unwrap().full_name(),
        "my.package.MyMessage.MyFieldMessage"
    );
}

#[test]
fn message_field_type_not_set() {
    let file_descriptor_set = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            package: Some("my.package".to_owned()),
            syntax: Some("proto3".to_owned()),
            message_type: vec![
                DescriptorProto {
                    name: Some("MyFieldMessage".to_owned()),
                    ..Default::default()
                },
                DescriptorProto {
                    name: Some("MyMessage".to_owned()),
                    field: vec![FieldDescriptorProto {
                        name: Some("my_field".to_owned()),
                        number: Some(1),
                        label: Some(Label::Optional as i32),
                        r#type: None,
                        type_name: Some(".my.package.MyFieldMessage".to_owned()),
                        json_name: Some("myfield".to_owned()),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
    };

    let file_descriptor = FileDescriptor::new(file_descriptor_set).unwrap();
    let message = file_descriptor
        .get_message_by_name("my.package.MyMessage")
        .unwrap();
    let field = message.get_field_by_name("my_field").unwrap();
    assert_eq!(
        field.kind().as_message().unwrap().full_name(),
        "my.package.MyFieldMessage"
    );
}
