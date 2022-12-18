use prost_types::{
    DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto,
    FileDescriptorProto, MethodDescriptorProto, OneofDescriptorProto, ServiceDescriptorProto,
};

use crate::descriptor::{
    build::DescriptorPoolOffsets, tag, to_index, EnumIndex, EnumValueIndex, ExtensionIndex,
    FieldIndex, FileIndex, MessageIndex, MethodIndex, OneofIndex, ServiceIndex,
};

pub(super) trait Visitor {
    fn visit_file(&mut self, _path: &[i32], _index: FileIndex, _file: &FileDescriptorProto) {}

    fn visit_message(
        &mut self,
        _path: &[i32],
        _full_name: &str,
        _file: FileIndex,
        _parent_message: Option<MessageIndex>,
        _index: MessageIndex,
        _message: &DescriptorProto,
    ) {
    }

    fn visit_field(
        &mut self,
        _path: &[i32],
        _full_name: &str,
        _file: FileIndex,
        _message: MessageIndex,
        _index: FieldIndex,
        _field: &FieldDescriptorProto,
    ) {
    }

    fn visit_oneof(
        &mut self,
        _path: &[i32],
        _full_name: &str,
        _file: FileIndex,
        _message: MessageIndex,
        _index: OneofIndex,
        _oneof: &OneofDescriptorProto,
    ) {
    }

    fn visit_service(
        &mut self,
        _path: &[i32],
        _full_name: &str,
        _file: FileIndex,
        _index: ServiceIndex,
        _service: &ServiceDescriptorProto,
    ) {
    }

    fn visit_method(
        &mut self,
        _path: &[i32],
        _full_name: &str,
        _file: FileIndex,
        _service: ServiceIndex,
        _index: MethodIndex,
        _method: &MethodDescriptorProto,
    ) {
    }

    fn visit_enum(
        &mut self,
        _path: &[i32],
        _full_name: &str,
        _file: FileIndex,
        _parent_message: Option<MessageIndex>,
        _index: EnumIndex,
        _enum: &EnumDescriptorProto,
    ) {
    }

    fn visit_enum_value(
        &mut self,
        _path: &[i32],
        _full_name: &str,
        _file: FileIndex,
        _enum_: EnumIndex,
        _index: EnumValueIndex,
        _value: &EnumValueDescriptorProto,
    ) {
    }

    fn visit_extension(
        &mut self,
        _path: &[i32],
        _full_name: &str,
        _file: FileIndex,
        _parent_message: Option<MessageIndex>,
        _index: ExtensionIndex,
        _extension: &FieldDescriptorProto,
    ) {
    }
}

pub(super) fn visit<'a, V>(
    offsets: DescriptorPoolOffsets,
    files: impl IntoIterator<Item = &'a FileDescriptorProto>,
    visitor: &mut V,
) where
    V: Visitor,
{
    let mut context = Context {
        path: Vec::new(),
        scope: String::new(),
        offsets,
    };

    for file in files {
        context.visit_file(file, visitor);
    }
}

struct Context {
    path: Vec<i32>,
    scope: String,
    offsets: DescriptorPoolOffsets,
}

impl Context {
    fn visit_file(&mut self, file: &FileDescriptorProto, visitor: &mut impl Visitor) {
        if !file.package().is_empty() {
            self.push_scope(file.package());
        }

        let index = post_inc(&mut self.offsets.file);
        visitor.visit_file(&self.path, index, file);

        self.push_path(tag::file::MESSAGE_TYPE);
        for (i, message) in file.message_type.iter().enumerate() {
            self.push_path(i as i32);
            self.visit_message(message, visitor, index, None);
            self.pop_path();
        }
        self.pop_path();

        self.push_path(tag::file::ENUM_TYPE);
        for (i, enum_) in file.enum_type.iter().enumerate() {
            self.push_path(i as i32);
            self.visit_enum(enum_, visitor, index, None);
            self.pop_path();
        }
        self.pop_path();

        self.push_path(tag::file::SERVICE);
        for (i, service) in file.service.iter().enumerate() {
            self.push_path(i as i32);
            self.visit_service(service, visitor, index);
            self.pop_path();
        }
        self.pop_path();

        self.push_path(tag::file::EXTENSION);
        for (i, extension) in file.extension.iter().enumerate() {
            self.push_path(i as i32);
            self.visit_extension(extension, visitor, index, None);
            self.pop_path();
        }
        self.pop_path();

        if !file.package().is_empty() {
            self.pop_scope(file.package());
        }
    }

    fn visit_message(
        &mut self,
        message: &DescriptorProto,
        visitor: &mut impl Visitor,
        file: FileIndex,
        parent_message: Option<MessageIndex>,
    ) {
        self.push_scope(message.name());

        let index = post_inc(&mut self.offsets.message);
        visitor.visit_message(
            &self.path,
            &self.scope,
            file,
            parent_message,
            index,
            message,
        );

        self.push_path(tag::message::ONEOF_DECL);
        for (i, oneof) in message.oneof_decl.iter().enumerate() {
            self.push_path(i as i32);
            self.visit_oneof(oneof, visitor, file, index, to_index(i));
            self.pop_path();
        }
        self.pop_path();

        self.push_path(tag::message::FIELD);
        for (i, field) in message.field.iter().enumerate() {
            self.push_path(i as i32);
            self.visit_field(field, visitor, file, index, to_index(i));
            self.pop_path();
        }
        self.pop_path();

        self.push_path(tag::message::NESTED_TYPE);
        for (i, nested) in message.nested_type.iter().enumerate() {
            self.push_path(i as i32);
            self.visit_message(nested, visitor, file, Some(index));
            self.pop_path();
        }
        self.pop_path();

        self.push_path(tag::message::ENUM_TYPE);
        for (i, enum_) in message.enum_type.iter().enumerate() {
            self.push_path(i as i32);
            self.visit_enum(enum_, visitor, file, Some(index));
            self.pop_path();
        }
        self.pop_path();

        self.push_path(tag::message::EXTENSION);
        for (i, extension) in message.extension.iter().enumerate() {
            self.push_path(i as i32);
            self.visit_extension(extension, visitor, file, Some(index));
            self.pop_path();
        }
        self.pop_path();

        self.pop_scope(message.name());
    }

    fn visit_field(
        &mut self,
        field: &FieldDescriptorProto,
        visitor: &mut impl Visitor,
        file: FileIndex,
        message: MessageIndex,
        index: FieldIndex,
    ) {
        self.push_scope(field.name());
        visitor.visit_field(&self.path, &self.scope, file, message, index, field);
        self.pop_scope(field.name());
    }

    fn visit_oneof(
        &mut self,
        oneof: &OneofDescriptorProto,
        visitor: &mut impl Visitor,
        file: FileIndex,
        message: MessageIndex,
        index: OneofIndex,
    ) {
        self.push_scope(oneof.name());
        visitor.visit_oneof(&self.path, &self.scope, file, message, index, oneof);
        self.pop_scope(oneof.name());
    }

    fn visit_service(
        &mut self,
        service: &ServiceDescriptorProto,
        visitor: &mut impl Visitor,
        file: FileIndex,
    ) {
        self.push_scope(service.name());

        let index = post_inc(&mut self.offsets.service);
        visitor.visit_service(&self.path, &self.scope, file, index, service);

        self.push_path(tag::service::METHOD);
        for (i, method) in service.method.iter().enumerate() {
            self.push_path(i as i32);
            self.visit_method(method, visitor, file, index, to_index(i));
            self.pop_path();
        }
        self.pop_path();

        self.pop_scope(service.name());
    }

    fn visit_method(
        &mut self,
        method: &MethodDescriptorProto,
        visitor: &mut impl Visitor,
        file: FileIndex,
        service: ServiceIndex,
        index: MethodIndex,
    ) {
        self.push_scope(method.name());
        visitor.visit_method(&self.path, &self.scope, file, service, index, method);
        self.pop_scope(method.name());
    }

    fn visit_enum(
        &mut self,
        enum_: &EnumDescriptorProto,
        visitor: &mut impl Visitor,
        file: FileIndex,
        parent_message: Option<MessageIndex>,
    ) {
        self.push_scope(enum_.name());

        let index = post_inc(&mut self.offsets.enum_);
        visitor.visit_enum(&self.path, &self.scope, file, parent_message, index, enum_);

        self.pop_scope(enum_.name());

        self.push_path(tag::enum_::VALUE);
        for (i, method) in enum_.value.iter().enumerate() {
            self.push_path(i as i32);
            self.visit_enum_value(method, visitor, file, index, to_index(i));
            self.pop_path();
        }
        self.pop_path();
    }

    fn visit_enum_value(
        &mut self,
        value: &EnumValueDescriptorProto,
        visitor: &mut impl Visitor,
        file: FileIndex,
        enum_: EnumIndex,
        index: EnumValueIndex,
    ) {
        self.push_scope(value.name());
        visitor.visit_enum_value(&self.path, &self.scope, file, enum_, index, value);
        self.pop_scope(value.name());
    }

    fn visit_extension(
        &mut self,
        extension: &FieldDescriptorProto,
        visitor: &mut impl Visitor,
        file: FileIndex,
        parent_message: Option<MessageIndex>,
    ) {
        self.push_scope(extension.name());
        let index = post_inc(&mut self.offsets.extension);
        visitor.visit_extension(
            &self.path,
            &self.scope,
            file,
            parent_message,
            index,
            extension,
        );
        self.pop_scope(extension.name());
    }

    fn push_path(&mut self, path: i32) {
        self.path.push(path);
    }

    fn pop_path(&mut self) {
        self.path.pop().unwrap();
    }

    fn push_scope(&mut self, scope: &str) {
        if !self.scope.is_empty() {
            self.scope.push('.');
        }
        self.scope.push_str(scope);
    }

    fn pop_scope(&mut self, scope: &str) {
        debug_assert!(self.scope.ends_with(scope));
        self.scope
            .truncate((self.scope.len() - scope.len()).saturating_sub(1));
    }
}

fn post_inc(index: &mut u32) -> u32 {
    let value = *index;
    *index = value + 1;
    value
}
