//! Resolution of protobuf edition features.
//!
//! Features are inherited lexically: a value set on a file applies to everything in it unless an
//! enclosing message, an enum or a field overrides it. `prost_types` has no representation of
//! features, so they are decoded from the encoded bytes of each options message.

use prost::Message;

use crate::descriptor::{
    tag,
    types::{DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto},
};

const EDITION_2023: i32 = 1000;
const EDITION_2024: i32 = 1001;

const FIELD_PRESENCE_IMPLICIT: i32 = 2;
const FIELD_PRESENCE_LEGACY_REQUIRED: i32 = 3;
const ENUM_TYPE_OPEN: i32 = 1;
const REPEATED_FIELD_ENCODING_EXPANDED: i32 = 2;
const MESSAGE_ENCODING_DELIMITED: i32 = 2;
const JSON_FORMAT_ALLOW: i32 = 1;

/// Whether the feature defaults of this edition are known. An edition may change them, so a file
/// from a later edition cannot be interpreted safely.
pub(super) fn is_supported_edition(edition: Option<i32>) -> bool {
    matches!(edition, Some(EDITION_2023 | EDITION_2024))
}

/// `google.protobuf.FeatureSet`, limited to the features this crate acts on.
#[derive(Clone, PartialEq, Message)]
struct FeatureSet {
    #[prost(int32, optional, tag = "1")]
    field_presence: Option<i32>,
    #[prost(int32, optional, tag = "2")]
    enum_type: Option<i32>,
    #[prost(int32, optional, tag = "3")]
    repeated_field_encoding: Option<i32>,
    #[prost(int32, optional, tag = "5")]
    message_encoding: Option<i32>,
    #[prost(int32, optional, tag = "6")]
    json_format: Option<i32>,
}

macro_rules! features_of {
    ($name:ident, $tag:literal) => {
        #[derive(Clone, PartialEq, Message)]
        struct $name {
            #[prost(message, optional, tag = $tag)]
            features: Option<FeatureSet>,
        }
    };
}

features_of!(FileFeatures, "50");
features_of!(MessageFeatures, "12");
features_of!(FieldFeatures, "21");
features_of!(EnumFeatures, "7");

/// The features in effect at some point in a file.
#[derive(Copy, Clone, Debug)]
pub(super) struct Features {
    field_presence: i32,
    enum_type: i32,
    repeated_field_encoding: i32,
    message_encoding: i32,
    json_format: i32,
}

impl Features {
    /// The defaults shared by editions 2023 and 2024, overlaid with the options of `file`.
    fn of_file(file: &FileDescriptorProto) -> Self {
        let mut features = Features {
            field_presence: 1,
            enum_type: ENUM_TYPE_OPEN,
            repeated_field_encoding: 1,
            message_encoding: 1,
            json_format: JSON_FORMAT_ALLOW,
        };
        if let Some(options) = &file.options {
            features.apply(FileFeatures::decode(options.encoded.as_slice()).map(|o| o.features));
        }
        features
    }

    fn apply(&mut self, set: Result<Option<FeatureSet>, prost::DecodeError>) {
        // A malformed options message is reported when the options themselves are decoded.
        let Ok(Some(set)) = set else { return };
        let FeatureSet {
            field_presence,
            enum_type,
            repeated_field_encoding,
            message_encoding,
            json_format,
        } = set;
        self.field_presence = field_presence.unwrap_or(self.field_presence);
        self.enum_type = enum_type.unwrap_or(self.enum_type);
        self.repeated_field_encoding =
            repeated_field_encoding.unwrap_or(self.repeated_field_encoding);
        self.message_encoding = message_encoding.unwrap_or(self.message_encoding);
        self.json_format = json_format.unwrap_or(self.json_format);
    }

    fn apply_message(&mut self, message: &DescriptorProto) {
        if let Some(options) = &message.options {
            self.apply(MessageFeatures::decode(options.encoded.as_slice()).map(|o| o.features));
        }
    }

    /// The features at `path`, which may be that of a message or of something declared in one.
    /// Stops at the first element that is not a message, so the features of a message at the end
    /// of `path` are included.
    fn scope(file: &FileDescriptorProto, path: &[i32]) -> Self {
        let mut features = Features::of_file(file);
        let mut message: Option<&DescriptorProto> = None;
        for part in path.chunks_exact(2) {
            let index = usize::try_from(part[1]).unwrap_or(usize::MAX);
            let next = match (message, part[0]) {
                (None, tag::file::MESSAGE_TYPE) => file.message_type.get(index),
                (Some(parent), tag::message::NESTED_TYPE) => parent.nested_type.get(index),
                _ => None,
            };
            let Some(next) = next else { break };
            features.apply_message(next);
            message = Some(next);
        }
        features
    }

    /// The features of the message at `path`.
    pub(super) fn of_message(file: &FileDescriptorProto, path: &[i32]) -> Self {
        Features::scope(file, path)
    }

    /// The features of the field or extension at `path`.
    pub(super) fn of_field(
        file: &FileDescriptorProto,
        path: &[i32],
        field: &FieldDescriptorProto,
    ) -> Self {
        let mut features = Features::scope(file, path);
        if let Some(options) = &field.options {
            features.apply(FieldFeatures::decode(options.encoded.as_slice()).map(|o| o.features));
        }
        features
    }

    /// The features of the enum at `path`.
    pub(super) fn of_enum(
        file: &FileDescriptorProto,
        path: &[i32],
        enum_: &EnumDescriptorProto,
    ) -> Self {
        let mut features = Features::scope(file, path);
        if let Some(options) = &enum_.options {
            features.apply(EnumFeatures::decode(options.encoded.as_slice()).map(|o| o.features));
        }
        features
    }

    pub(super) fn is_implicit_presence(&self) -> bool {
        self.field_presence == FIELD_PRESENCE_IMPLICIT
    }

    pub(super) fn is_legacy_required(&self) -> bool {
        self.field_presence == FIELD_PRESENCE_LEGACY_REQUIRED
    }

    pub(super) fn is_expanded(&self) -> bool {
        self.repeated_field_encoding == REPEATED_FIELD_ENCODING_EXPANDED
    }

    pub(super) fn is_delimited(&self) -> bool {
        self.message_encoding == MESSAGE_ENCODING_DELIMITED
    }

    pub(super) fn is_open_enum(&self) -> bool {
        self.enum_type == ENUM_TYPE_OPEN
    }

    pub(super) fn allows_json(&self) -> bool {
        self.json_format == JSON_FORMAT_ALLOW
    }
}
