use prost_types::{
    DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto,
    FileDescriptorProto, MethodDescriptorProto, OneofDescriptorProto, ServiceDescriptorProto,
};

use crate::descriptor::{
    build::{
        visit::{visit, Visitor},
        DescriptorPoolOffsets,
    },
    DescriptorPoolInner, EnumIndex, EnumValueIndex, ExtensionIndex, FieldIndex, FileIndex,
    MessageIndex, MethodIndex, OneofIndex, ServiceIndex,
};

impl DescriptorPoolInner {
    pub(super) fn resolve_options<'a>(
        &mut self,
        offsets: DescriptorPoolOffsets,
        files: impl Iterator<Item = &'a FileDescriptorProto>,
    ) {
        let mut visitor = OptionsVisitor { _pool: self };
        visit(offsets, files, &mut visitor);
    }
}

struct OptionsVisitor<'a> {
    _pool: &'a mut DescriptorPoolInner,
}

impl<'a> Visitor for OptionsVisitor<'a> {
    fn visit_file(&mut self, _: &[i32], _: FileIndex, _file: &FileDescriptorProto) {}

    fn visit_message(
        &mut self,
        _: &[i32],
        _: &str,
        _: FileIndex,
        _: Option<MessageIndex>,
        _: MessageIndex,
        _message: &DescriptorProto,
    ) {
    }

    fn visit_field(
        &mut self,
        _: &[i32],
        _: &str,
        _: FileIndex,
        _: MessageIndex,
        _: FieldIndex,
        _field: &FieldDescriptorProto,
    ) {
    }

    fn visit_oneof(
        &mut self,
        _: &[i32],
        _: &str,
        _: FileIndex,
        _: MessageIndex,
        _: OneofIndex,
        _oneof: &OneofDescriptorProto,
    ) {
    }

    fn visit_service(
        &mut self,
        _: &[i32],
        _: &str,
        _: FileIndex,
        _: ServiceIndex,
        _service: &ServiceDescriptorProto,
    ) {
    }

    fn visit_method(
        &mut self,
        _: &[i32],
        _: &str,
        _: FileIndex,
        _: ServiceIndex,
        _: MethodIndex,
        _method: &MethodDescriptorProto,
    ) {
    }

    fn visit_enum(
        &mut self,
        _: &[i32],
        _: &str,
        _: FileIndex,
        _: Option<MessageIndex>,
        _: EnumIndex,
        _enum_: &EnumDescriptorProto,
    ) {
    }

    fn visit_enum_value(
        &mut self,
        _: &[i32],
        _: &str,
        _: FileIndex,
        _: EnumIndex,
        _: EnumValueIndex,
        _value: &EnumValueDescriptorProto,
    ) {
    }

    fn visit_extension(
        &mut self,
        _: &[i32],
        _: &str,
        _: FileIndex,
        _: Option<MessageIndex>,
        _: ExtensionIndex,
        _extension: &FieldDescriptorProto,
    ) {
    }
}
