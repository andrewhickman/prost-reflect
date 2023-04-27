use crate::{DescriptorPool, MessageDescriptor, ReflectMessage};

pub(crate) const WELL_KNOWN_TYPES_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/well_known_types.bin"));

macro_rules! impl_reflect_message {
    ($($ty:ty => $name:literal;)*) => {
        $(
            impl ReflectMessage for $ty {
                #[doc = concat!("Returns a descriptor for the `", $name, "` message type.")]
                fn descriptor(&self) -> MessageDescriptor {
                    match DescriptorPool::global().get_message_by_name($name) {
                        Some(desc) => desc,
                        None => panic!("descriptor for well-known type `{}` not found", $name),
                    }
                }
            }
        )*

        #[test]
        fn test_reflect_message_impls() {
            $(
                assert_eq!(<$ty>::default().descriptor().full_name(), $name);
            )*
        }
    };
}

impl_reflect_message! {
    () => "google.protobuf.Empty";
    bool => "google.protobuf.BoolValue";
    f32 => "google.protobuf.FloatValue";
    f64 => "google.protobuf.DoubleValue";
    i32 => "google.protobuf.Int32Value";
    i64 => "google.protobuf.Int64Value";
    String => "google.protobuf.StringValue";
    u32 => "google.protobuf.UInt32Value";
    u64 => "google.protobuf.UInt64Value";
    Vec<u8> => "google.protobuf.BytesValue";
    prost_types::Any => "google.protobuf.Any";
    prost_types::Api => "google.protobuf.Api";
    prost_types::compiler::CodeGeneratorRequest => "google.protobuf.compiler.CodeGeneratorRequest";
    prost_types::compiler::CodeGeneratorResponse => "google.protobuf.compiler.CodeGeneratorResponse";
    prost_types::compiler::code_generator_response::File => "google.protobuf.compiler.CodeGeneratorResponse.File";
    prost_types::compiler::Version => "google.protobuf.compiler.Version";
    prost_types::DescriptorProto => "google.protobuf.DescriptorProto";
    prost_types::descriptor_proto::ExtensionRange => "google.protobuf.DescriptorProto.ExtensionRange";
    prost_types::descriptor_proto::ReservedRange => "google.protobuf.DescriptorProto.ReservedRange";
    prost_types::Duration => "google.protobuf.Duration";
    prost_types::Enum => "google.protobuf.Enum";
    prost_types::EnumDescriptorProto => "google.protobuf.EnumDescriptorProto";
    prost_types::enum_descriptor_proto::EnumReservedRange => "google.protobuf.EnumDescriptorProto.EnumReservedRange";
    prost_types::EnumOptions => "google.protobuf.EnumOptions";
    prost_types::EnumValue => "google.protobuf.EnumValue";
    prost_types::EnumValueDescriptorProto => "google.protobuf.EnumValueDescriptorProto";
    prost_types::EnumValueOptions => "google.protobuf.EnumValueOptions";
    prost_types::ExtensionRangeOptions => "google.protobuf.ExtensionRangeOptions";
    prost_types::Field => "google.protobuf.Field";
    prost_types::FieldDescriptorProto => "google.protobuf.FieldDescriptorProto";
    prost_types::FieldMask => "google.protobuf.FieldMask";
    prost_types::FieldOptions => "google.protobuf.FieldOptions";
    prost_types::FileDescriptorProto => "google.protobuf.FileDescriptorProto";
    prost_types::FileDescriptorSet => "google.protobuf.FileDescriptorSet";
    prost_types::FileOptions => "google.protobuf.FileOptions";
    prost_types::GeneratedCodeInfo => "google.protobuf.GeneratedCodeInfo";
    prost_types::generated_code_info::Annotation => "google.protobuf.GeneratedCodeInfo.Annotation";
    prost_types::ListValue => "google.protobuf.ListValue";
    prost_types::MessageOptions => "google.protobuf.MessageOptions";
    prost_types::Method => "google.protobuf.Method";
    prost_types::MethodDescriptorProto => "google.protobuf.MethodDescriptorProto";
    prost_types::MethodOptions => "google.protobuf.MethodOptions";
    prost_types::Mixin => "google.protobuf.Mixin";
    prost_types::OneofDescriptorProto => "google.protobuf.OneofDescriptorProto";
    prost_types::OneofOptions => "google.protobuf.OneofOptions";
    prost_types::Option => "google.protobuf.Option";
    prost_types::ServiceDescriptorProto => "google.protobuf.ServiceDescriptorProto";
    prost_types::ServiceOptions => "google.protobuf.ServiceOptions";
    prost_types::SourceCodeInfo => "google.protobuf.SourceCodeInfo";
    prost_types::source_code_info::Location => "google.protobuf.SourceCodeInfo.Location";
    prost_types::SourceContext => "google.protobuf.SourceContext";
    prost_types::Struct => "google.protobuf.Struct";
    prost_types::Timestamp => "google.protobuf.Timestamp";
    prost_types::Type => "google.protobuf.Type";
    prost_types::UninterpretedOption => "google.protobuf.UninterpretedOption";
    prost_types::uninterpreted_option::NamePart => "google.protobuf.UninterpretedOption.NamePart";
    prost_types::Value => "google.protobuf.Value";
    prost::bytes::Bytes => "google.protobuf.BytesValue";
}
