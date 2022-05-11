use prost_types::{
    field_descriptor_proto::{Label, Type},
    DescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileDescriptorSet,
    MethodDescriptorProto, ServiceDescriptorProto,
};

use crate::DescriptorPool;

#[test]
fn resolve_service_name() {
    let file_descriptor_set = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            name: Some("myfile.proto".to_owned()),
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

    let descriptor_pool = DescriptorPool::from_file_descriptor_set(file_descriptor_set).unwrap();
    let service = descriptor_pool.services().next().unwrap();
    let method = service.methods().next().unwrap();
    assert_eq!(method.input().full_name(), "my.package.MyMessage");
    assert_eq!(method.output().full_name(), "my.package.MyMessage");
}

#[test]
fn resolve_service_name_other_package() {
    let file_descriptor_set = FileDescriptorSet {
        file: vec![
            FileDescriptorProto {
                name: Some("myfile.proto".to_owned()),
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

    let descriptor_pool = DescriptorPool::from_file_descriptor_set(file_descriptor_set).unwrap();
    let service = descriptor_pool.services().next().unwrap();
    let method = service.methods().next().unwrap();
    assert_eq!(method.input().full_name(), "other.package.MyMessage");
    assert_eq!(method.output().full_name(), "other.package.MyMessage");
}

#[test]
fn resolve_message_name() {
    let file_descriptor_set = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            name: Some("myfile.proto".to_owned()),
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

    let descriptor_pool = DescriptorPool::from_file_descriptor_set(file_descriptor_set).unwrap();
    let message = descriptor_pool
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
            name: Some("myfile.proto".to_owned()),
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

    let descriptor_pool = DescriptorPool::from_file_descriptor_set(file_descriptor_set).unwrap();
    let message = descriptor_pool
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
            name: Some("myfile.proto".to_owned()),
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

    let descriptor_pool = DescriptorPool::from_file_descriptor_set(file_descriptor_set).unwrap();
    let message = descriptor_pool
        .get_message_by_name("my.package.MyMessage")
        .unwrap();
    let field = message.get_field_by_name("my_field").unwrap();
    assert_eq!(
        field.kind().as_message().unwrap().full_name(),
        "my.package.MyFieldMessage"
    );
}

#[test]
fn reference_type_in_previously_added_file() {
    let file_descriptor_set1 = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            name: Some("myfile1.proto".to_owned()),
            package: Some("my.package1".to_owned()),
            syntax: Some("proto3".to_owned()),
            message_type: vec![DescriptorProto {
                name: Some("MyFieldMessage".to_owned()),
                ..Default::default()
            }],
            ..Default::default()
        }],
    };
    let file_descriptor_set2 = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            name: Some("myfile2.proto".to_owned()),
            package: Some("my.package2".to_owned()),
            syntax: Some("proto3".to_owned()),
            dependency: vec!["myfile1.proto".to_owned()],
            public_dependency: vec![0],
            message_type: vec![DescriptorProto {
                name: Some("MyMessage".to_owned()),
                field: vec![FieldDescriptorProto {
                    name: Some("my_field".to_owned()),
                    number: Some(1),
                    label: Some(Label::Optional as i32),
                    r#type: None,
                    type_name: Some(".my.package1.MyFieldMessage".to_owned()),
                    json_name: Some("myfield".to_owned()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }],
    };

    let mut pool = DescriptorPool::new();
    pool.add_file_descriptor_set(file_descriptor_set1).unwrap();
    pool.add_file_descriptor_set(file_descriptor_set2).unwrap();
    assert_eq!(pool.get_file_by_name("notfound"), None);

    let file1 = pool.get_file_by_name("myfile1.proto").unwrap();
    let file2 = pool.get_file_by_name("myfile2.proto").unwrap();
    assert_ne!(file1, file2);
    assert_eq!(file1.dependencies().collect::<Vec<_>>(), vec![]);
    assert_eq!(
        file2.dependencies().collect::<Vec<_>>(),
        vec![file1.clone()]
    );

    assert_eq!(file1.name(), "myfile1.proto");
    assert_eq!(file1.package_name(), "my.package1");
    assert_eq!(file2.name(), "myfile2.proto");
    assert_eq!(file2.package_name(), "my.package2");

    let message = pool.get_message_by_name("my.package2.MyMessage").unwrap();
    assert_eq!(message.parent_file(), file2);

    let field = message.get_field_by_name("my_field").unwrap();
    assert_eq!(
        field.kind().as_message().unwrap().full_name(),
        "my.package1.MyFieldMessage"
    );
    assert_eq!(field.kind().as_message().unwrap().parent_file(), file1);
}

#[test]
fn add_duplicate_file() {
    let file_descriptor_set = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            name: Some("myfile.proto".to_owned()),
            package: Some("my.package".to_owned()),
            syntax: Some("proto3".to_owned()),
            message_type: vec![DescriptorProto {
                name: Some("MyMessage".to_owned()),
                ..Default::default()
            }],
            ..Default::default()
        }],
    };

    let mut pool = DescriptorPool::new();
    pool.add_file_descriptor_set(file_descriptor_set.clone())
        .unwrap();
    pool.add_file_descriptor_set(file_descriptor_set).unwrap();

    assert_eq!(pool.file_descriptor_protos().len(), 1);
}

#[test]
fn add_conflicting_duplicate_file() {
    let file_descriptor_set1 = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            name: Some("myfile.proto".to_owned()),
            package: Some("my.package".to_owned()),
            syntax: Some("proto3".to_owned()),
            message_type: vec![DescriptorProto {
                name: Some("MyMessage1".to_owned()),
                ..Default::default()
            }],
            ..Default::default()
        }],
    };
    let file_descriptor_set2 = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            name: Some("myfile.proto".to_owned()),
            package: Some("my.package".to_owned()),
            syntax: Some("proto3".to_owned()),
            message_type: vec![DescriptorProto {
                name: Some("MyMessage2".to_owned()),
                ..Default::default()
            }],
            ..Default::default()
        }],
    };

    let mut pool = DescriptorPool::new();
    pool.add_file_descriptor_set(file_descriptor_set1).unwrap();
    let err = pool
        .add_file_descriptor_set(file_descriptor_set2)
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("file named 'myfile.proto' is already added"));
}

#[test]
fn add_file_rollback_on_error() {
    let bad_file_descriptor_set = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            name: Some("myfile.proto".to_owned()),
            package: Some("my.package".to_owned()),
            syntax: Some("proto3".to_owned()),
            service: vec![ServiceDescriptorProto {
                name: Some("MyService".to_owned()),
                method: vec![MethodDescriptorProto {
                    name: Some("my_method".to_owned()),
                    input_type: Some(".my.package.NopeMessage".to_owned()),
                    output_type: Some(".my.package.NopeMessage".to_owned()),
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

    let mut pool = DescriptorPool::new();
    let err = pool
        .add_file_descriptor_set(bad_file_descriptor_set)
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("the message or enum type 'my.package.NopeMessage' was not found"));
    assert_eq!(pool.file_descriptor_protos().count(), 0);
    assert_eq!(pool.get_message_by_name(".my.package.MyMessage"), None);
}

#[test]
fn add_file_missing_dependency() {
    let bad_file_descriptor_set = FileDescriptorSet {
        file: vec![FileDescriptorProto {
            name: Some("myfile.proto".to_owned()),
            package: Some("my.package".to_owned()),
            dependency: vec!["notfound.proto".to_owned()],
            public_dependency: vec![0],
            syntax: Some("proto3".to_owned()),
            ..Default::default()
        }],
    };

    let mut pool = DescriptorPool::new();
    let err = pool
        .add_file_descriptor_set(bad_file_descriptor_set)
        .unwrap_err();
    assert!(err.to_string().contains(
        "the file 'notfound.proto' was not found while resolving dependencies for 'myfile.proto'"
    ));
}
