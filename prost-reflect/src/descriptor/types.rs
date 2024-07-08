use std::fmt;

use prost::{
    bytes::{Buf, BufMut},
    encoding::{encode_key, skip_field, DecodeContext, WireType},
    DecodeError, Message,
};

pub(crate) use prost_types::{
    enum_descriptor_proto, field_descriptor_proto, uninterpreted_option, EnumOptions,
    EnumValueOptions, ExtensionRangeOptions, FieldOptions, FileOptions, MessageOptions,
    MethodOptions, OneofOptions, ServiceOptions, SourceCodeInfo, UninterpretedOption,
};

#[derive(Clone, PartialEq, Message)]
pub(crate) struct FileDescriptorSet {
    #[prost(message, repeated, tag = "1")]
    pub file: Vec<FileDescriptorProto>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct FileDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(string, optional, tag = "2")]
    pub package: Option<String>,
    #[prost(string, repeated, tag = "3")]
    pub dependency: Vec<String>,
    #[prost(int32, repeated, packed = "false", tag = "10")]
    pub public_dependency: Vec<i32>,
    #[prost(int32, repeated, packed = "false", tag = "11")]
    pub weak_dependency: Vec<i32>,
    #[prost(message, repeated, tag = "4")]
    pub message_type: Vec<DescriptorProto>,
    #[prost(message, repeated, tag = "5")]
    pub(crate) enum_type: Vec<EnumDescriptorProto>,
    #[prost(message, repeated, tag = "6")]
    pub service: Vec<ServiceDescriptorProto>,
    #[prost(message, repeated, tag = "7")]
    pub extension: Vec<FieldDescriptorProto>,
    #[prost(message, optional, tag = "8")]
    pub options: Option<Options<FileOptions>>,
    #[prost(message, optional, tag = "9")]
    pub source_code_info: Option<SourceCodeInfo>,
    #[prost(string, optional, tag = "12")]
    pub syntax: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct DescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(message, repeated, tag = "2")]
    pub field: Vec<FieldDescriptorProto>,
    #[prost(message, repeated, tag = "6")]
    pub extension: Vec<FieldDescriptorProto>,
    #[prost(message, repeated, tag = "3")]
    pub nested_type: Vec<DescriptorProto>,
    #[prost(message, repeated, tag = "4")]
    pub(crate) enum_type: Vec<EnumDescriptorProto>,
    #[prost(message, repeated, tag = "5")]
    pub extension_range: Vec<descriptor_proto::ExtensionRange>,
    #[prost(message, repeated, tag = "8")]
    pub oneof_decl: Vec<OneofDescriptorProto>,
    #[prost(message, optional, tag = "7")]
    pub options: Option<Options<MessageOptions>>,
    #[prost(message, repeated, tag = "9")]
    pub reserved_range: Vec<descriptor_proto::ReservedRange>,
    #[prost(string, repeated, tag = "10")]
    pub reserved_name: Vec<String>,
}

pub(crate) mod descriptor_proto {
    pub(crate) use prost_types::descriptor_proto::ReservedRange;

    use super::*;

    #[derive(Clone, PartialEq, Message)]
    pub(crate) struct ExtensionRange {
        #[prost(int32, optional, tag = "1")]
        pub start: Option<i32>,
        #[prost(int32, optional, tag = "2")]
        pub end: Option<i32>,
        #[prost(message, optional, tag = "3")]
        pub options: Option<Options<ExtensionRangeOptions>>,
    }
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct FieldDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(int32, optional, tag = "3")]
    pub number: Option<i32>,
    #[prost(enumeration = "field_descriptor_proto::Label", optional, tag = "4")]
    pub label: Option<i32>,
    #[prost(enumeration = "field_descriptor_proto::Type", optional, tag = "5")]
    pub r#type: Option<i32>,
    #[prost(string, optional, tag = "6")]
    pub type_name: Option<String>,
    #[prost(string, optional, tag = "2")]
    pub extendee: Option<String>,
    #[prost(string, optional, tag = "7")]
    pub default_value: Option<String>,
    #[prost(int32, optional, tag = "9")]
    pub oneof_index: Option<i32>,
    #[prost(string, optional, tag = "10")]
    pub json_name: Option<String>,
    #[prost(message, optional, tag = "8")]
    pub options: Option<Options<FieldOptions>>,
    #[prost(bool, optional, tag = "17")]
    pub proto3_optional: Option<bool>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct OneofDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(message, optional, tag = "2")]
    pub options: Option<Options<OneofOptions>>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct EnumDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(message, repeated, tag = "2")]
    pub value: Vec<EnumValueDescriptorProto>,
    #[prost(message, optional, tag = "3")]
    pub options: Option<Options<EnumOptions>>,
    #[prost(message, repeated, tag = "4")]
    pub reserved_range: Vec<enum_descriptor_proto::EnumReservedRange>,
    #[prost(string, repeated, tag = "5")]
    pub reserved_name: Vec<String>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct EnumValueDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(int32, optional, tag = "2")]
    pub number: Option<i32>,
    #[prost(message, optional, tag = "3")]
    pub options: Option<Options<EnumValueOptions>>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct ServiceDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(message, repeated, tag = "2")]
    pub method: Vec<MethodDescriptorProto>,
    #[prost(message, optional, tag = "3")]
    pub options: Option<Options<ServiceOptions>>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct MethodDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(string, optional, tag = "2")]
    pub input_type: Option<String>,
    #[prost(string, optional, tag = "3")]
    pub output_type: Option<String>,
    #[prost(message, optional, tag = "4")]
    pub options: Option<Options<MethodOptions>>,
    #[prost(bool, optional, tag = "5", default = "false")]
    pub client_streaming: Option<bool>,
    #[prost(bool, optional, tag = "6", default = "false")]
    pub server_streaming: Option<bool>,
}

#[derive(Clone, Default, PartialEq)]
pub(crate) struct Options<T> {
    pub(crate) encoded: Vec<u8>,
    pub(crate) value: T,
}

impl FileDescriptorProto {
    pub(crate) fn from_prost(file: prost_types::FileDescriptorProto) -> FileDescriptorProto {
        FileDescriptorProto {
            name: file.name,
            package: file.package,
            dependency: file.dependency,
            public_dependency: file.public_dependency,
            weak_dependency: file.weak_dependency,
            message_type: file
                .message_type
                .into_iter()
                .map(DescriptorProto::from_prost)
                .collect(),
            enum_type: file
                .enum_type
                .into_iter()
                .map(EnumDescriptorProto::from_prost)
                .collect(),
            service: file
                .service
                .into_iter()
                .map(ServiceDescriptorProto::from_prost)
                .collect(),
            extension: file
                .extension
                .into_iter()
                .map(FieldDescriptorProto::from_prost)
                .collect(),
            options: file.options.map(Options::from_prost),
            source_code_info: file.source_code_info,
            syntax: file.syntax,
        }
    }

    pub(crate) fn to_prost(&self) -> prost_types::FileDescriptorProto {
        prost_types::FileDescriptorProto {
            name: self.name.clone(),
            package: self.package.clone(),
            dependency: self.dependency.clone(),
            public_dependency: self.public_dependency.clone(),
            weak_dependency: self.weak_dependency.clone(),
            message_type: self
                .message_type
                .iter()
                .map(DescriptorProto::to_prost)
                .collect(),
            enum_type: self
                .enum_type
                .iter()
                .map(EnumDescriptorProto::to_prost)
                .collect(),
            service: self
                .service
                .iter()
                .map(ServiceDescriptorProto::to_prost)
                .collect(),
            extension: self
                .extension
                .iter()
                .map(FieldDescriptorProto::to_prost)
                .collect(),
            options: self.options.as_ref().map(Options::to_prost),
            source_code_info: self.source_code_info.clone(),
            syntax: self.syntax.clone(),
        }
    }
}

impl DescriptorProto {
    pub(crate) fn from_prost(file: prost_types::DescriptorProto) -> DescriptorProto {
        DescriptorProto {
            name: file.name,
            field: file
                .field
                .into_iter()
                .map(FieldDescriptorProto::from_prost)
                .collect(),
            extension: file
                .extension
                .into_iter()
                .map(FieldDescriptorProto::from_prost)
                .collect(),
            nested_type: file
                .nested_type
                .into_iter()
                .map(DescriptorProto::from_prost)
                .collect(),
            enum_type: file
                .enum_type
                .into_iter()
                .map(EnumDescriptorProto::from_prost)
                .collect(),
            extension_range: file
                .extension_range
                .into_iter()
                .map(descriptor_proto::ExtensionRange::from_prost)
                .collect(),
            oneof_decl: file
                .oneof_decl
                .into_iter()
                .map(OneofDescriptorProto::from_prost)
                .collect(),
            options: file.options.map(Options::from_prost),
            reserved_range: file.reserved_range,
            reserved_name: file.reserved_name,
        }
    }

    pub(crate) fn to_prost(&self) -> prost_types::DescriptorProto {
        prost_types::DescriptorProto {
            name: self.name.clone(),
            field: self
                .field
                .iter()
                .map(FieldDescriptorProto::to_prost)
                .collect(),
            extension: self
                .extension
                .iter()
                .map(FieldDescriptorProto::to_prost)
                .collect(),
            nested_type: self
                .nested_type
                .iter()
                .map(DescriptorProto::to_prost)
                .collect(),
            enum_type: self
                .enum_type
                .iter()
                .map(EnumDescriptorProto::to_prost)
                .collect(),
            extension_range: self
                .extension_range
                .iter()
                .map(descriptor_proto::ExtensionRange::to_prost)
                .collect(),
            oneof_decl: self
                .oneof_decl
                .iter()
                .map(OneofDescriptorProto::to_prost)
                .collect(),
            options: self.options.as_ref().map(Options::to_prost),
            reserved_range: self.reserved_range.clone(),
            reserved_name: self.reserved_name.clone(),
        }
    }
}

impl FieldDescriptorProto {
    pub(crate) fn from_prost(file: prost_types::FieldDescriptorProto) -> FieldDescriptorProto {
        FieldDescriptorProto {
            name: file.name,
            number: file.number,
            label: file.label,
            r#type: file.r#type,
            type_name: file.type_name,
            extendee: file.extendee,
            default_value: file.default_value,
            oneof_index: file.oneof_index,
            json_name: file.json_name,
            options: file.options.map(Options::from_prost),
            proto3_optional: file.proto3_optional,
        }
    }

    pub(crate) fn to_prost(&self) -> prost_types::FieldDescriptorProto {
        prost_types::FieldDescriptorProto {
            name: self.name.clone(),
            number: self.number,
            label: self.label,
            r#type: self.r#type,
            type_name: self.type_name.clone(),
            extendee: self.extendee.clone(),
            default_value: self.default_value.clone(),
            oneof_index: self.oneof_index,
            json_name: self.json_name.clone(),
            options: self.options.as_ref().map(Options::to_prost),
            proto3_optional: self.proto3_optional,
        }
    }
}

impl OneofDescriptorProto {
    pub(crate) fn from_prost(file: prost_types::OneofDescriptorProto) -> OneofDescriptorProto {
        OneofDescriptorProto {
            name: file.name,
            options: file.options.map(Options::from_prost),
        }
    }

    pub(crate) fn to_prost(&self) -> prost_types::OneofDescriptorProto {
        prost_types::OneofDescriptorProto {
            name: self.name.clone(),
            options: self.options.as_ref().map(Options::to_prost),
        }
    }
}

impl descriptor_proto::ExtensionRange {
    pub(crate) fn from_prost(
        file: prost_types::descriptor_proto::ExtensionRange,
    ) -> descriptor_proto::ExtensionRange {
        descriptor_proto::ExtensionRange {
            start: file.start,
            end: file.end,
            options: file.options.map(Options::from_prost),
        }
    }

    pub(crate) fn to_prost(&self) -> prost_types::descriptor_proto::ExtensionRange {
        prost_types::descriptor_proto::ExtensionRange {
            start: self.start,
            end: self.end,
            options: self.options.as_ref().map(Options::to_prost),
        }
    }
}

impl EnumDescriptorProto {
    pub(crate) fn from_prost(file: prost_types::EnumDescriptorProto) -> EnumDescriptorProto {
        EnumDescriptorProto {
            name: file.name,
            value: file
                .value
                .into_iter()
                .map(EnumValueDescriptorProto::from_prost)
                .collect(),
            options: file.options.map(Options::from_prost),
            reserved_range: file.reserved_range,
            reserved_name: file.reserved_name,
        }
    }

    pub(crate) fn to_prost(&self) -> prost_types::EnumDescriptorProto {
        prost_types::EnumDescriptorProto {
            name: self.name.clone(),
            value: self
                .value
                .iter()
                .map(EnumValueDescriptorProto::to_prost)
                .collect(),
            options: self.options.as_ref().map(Options::to_prost),
            reserved_range: self.reserved_range.clone(),
            reserved_name: self.reserved_name.clone(),
        }
    }
}

impl EnumValueDescriptorProto {
    pub(crate) fn from_prost(
        file: prost_types::EnumValueDescriptorProto,
    ) -> EnumValueDescriptorProto {
        EnumValueDescriptorProto {
            name: file.name,
            number: file.number,
            options: file.options.map(Options::from_prost),
        }
    }

    pub(crate) fn to_prost(&self) -> prost_types::EnumValueDescriptorProto {
        prost_types::EnumValueDescriptorProto {
            name: self.name.clone(),
            number: self.number,
            options: self.options.as_ref().map(Options::to_prost),
        }
    }
}

impl ServiceDescriptorProto {
    pub(crate) fn from_prost(file: prost_types::ServiceDescriptorProto) -> ServiceDescriptorProto {
        ServiceDescriptorProto {
            name: file.name,
            method: file
                .method
                .into_iter()
                .map(MethodDescriptorProto::from_prost)
                .collect(),
            options: file.options.map(Options::from_prost),
        }
    }

    pub(crate) fn to_prost(&self) -> prost_types::ServiceDescriptorProto {
        prost_types::ServiceDescriptorProto {
            name: self.name.clone(),
            method: self
                .method
                .iter()
                .map(MethodDescriptorProto::to_prost)
                .collect(),
            options: self.options.as_ref().map(Options::to_prost),
        }
    }
}

impl MethodDescriptorProto {
    pub(crate) fn from_prost(file: prost_types::MethodDescriptorProto) -> MethodDescriptorProto {
        MethodDescriptorProto {
            name: file.name,
            input_type: file.input_type,
            output_type: file.output_type,
            options: file.options.map(Options::from_prost),
            client_streaming: file.client_streaming,
            server_streaming: file.server_streaming,
        }
    }

    pub(crate) fn to_prost(&self) -> prost_types::MethodDescriptorProto {
        prost_types::MethodDescriptorProto {
            name: self.name.clone(),
            input_type: self.input_type.clone(),
            output_type: self.output_type.clone(),
            options: self.options.as_ref().map(Options::to_prost),
            client_streaming: self.client_streaming,
            server_streaming: self.server_streaming,
        }
    }
}

impl<T> Options<T>
where
    T: Message + Clone,
{
    fn from_prost(options: T) -> Self {
        Options {
            encoded: options.encode_to_vec(),
            value: options,
        }
    }

    fn to_prost(&self) -> T {
        self.value.clone()
    }
}

impl<T> fmt::Debug for Options<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<T> Message for Options<T>
where
    T: Message + Default,
{
    fn encode_raw(&self, buf: &mut impl BufMut)
    where
        Self: Sized,
    {
        buf.put(self.encoded.as_slice());
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        Self: Sized,
    {
        struct CopyBufAdapter<'a, B> {
            dest: &'a mut Vec<u8>,
            src: &'a mut B,
        }

        impl<'a, B> Buf for CopyBufAdapter<'a, B>
        where
            B: Buf,
        {
            fn advance(&mut self, cnt: usize) {
                self.dest.put((&mut self.src).take(cnt));
            }

            fn chunk(&self) -> &[u8] {
                self.src.chunk()
            }

            fn remaining(&self) -> usize {
                self.src.remaining()
            }
        }

        encode_key(tag, wire_type, &mut self.encoded);
        let start = self.encoded.len();
        skip_field(
            wire_type,
            tag,
            &mut CopyBufAdapter {
                dest: &mut self.encoded,
                src: buf,
            },
            ctx.clone(),
        )?;
        self.value
            .merge_field(tag, wire_type, &mut &self.encoded[start..], ctx)?;

        Ok(())
    }

    fn encoded_len(&self) -> usize {
        self.encoded.len()
    }

    fn clear(&mut self) {
        self.encoded.clear();
        self.value.clear();
    }
}
