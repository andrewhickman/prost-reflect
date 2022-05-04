use std::fmt;

/// An error that may occur while creating a [`DescriptorPool`][crate::DescriptorPool].
#[derive(Debug)]
pub struct DescriptorError {
    kind: DescriptorErrorKind,
}

#[derive(Debug)]
enum DescriptorErrorKind {
    DecodeFileDescriptorSet {
        err: prost::DecodeError,
    },
    TypeNotFound {
        name: String,
    },
    TypeAlreadyExists {
        name: String,
    },
    UnknownSyntax {
        syntax: String,
    },
    InvalidMapEntry {
        name: String,
    },
    InvalidDefaultValue {
        name: String,
        field: String,
        value: String,
    },
    EmptyEnum,
    InvalidOneofIndex {
        name: String,
        field: String,
    },
    FileAlreadyExists {
        name: String,
    },
}

impl DescriptorError {
    pub(super) fn decode_file_descriptor_set(err: prost::DecodeError) -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::DecodeFileDescriptorSet { err },
        }
    }

    pub(super) fn type_not_found(name: impl ToString) -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::TypeNotFound {
                name: name.to_string(),
            },
        }
    }

    pub(super) fn type_already_exists(name: impl ToString) -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::TypeAlreadyExists {
                name: name.to_string(),
            },
        }
    }

    pub(super) fn unknown_syntax(syntax: impl ToString) -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::UnknownSyntax {
                syntax: syntax.to_string(),
            },
        }
    }

    pub(super) fn invalid_map_entry(name: impl ToString) -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::InvalidMapEntry {
                name: name.to_string(),
            },
        }
    }

    pub(super) fn invalid_default_value(
        name: impl ToString,
        field: impl ToString,
        value: impl ToString,
    ) -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::InvalidDefaultValue {
                name: name.to_string(),
                field: field.to_string(),
                value: value.to_string(),
            },
        }
    }

    pub(super) fn empty_enum() -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::EmptyEnum,
        }
    }

    pub(crate) fn invalid_oneof_index(name: impl ToString, field: impl ToString) -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::InvalidOneofIndex {
                name: name.to_string(),
                field: field.to_string(),
            },
        }
    }

    pub(crate) fn file_already_exists(name: impl ToString) -> Self {
        DescriptorError {
            kind: DescriptorErrorKind::FileAlreadyExists {
                name: name.to_string(),
            },
        }
    }
}

impl std::error::Error for DescriptorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            DescriptorErrorKind::DecodeFileDescriptorSet { err } => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            DescriptorErrorKind::DecodeFileDescriptorSet { .. } => {
                write!(f, "failed to decode file descriptor set")
            }
            DescriptorErrorKind::TypeNotFound { name } => {
                write!(f, "the message or enum type '{}' was not found", name)
            }
            DescriptorErrorKind::TypeAlreadyExists { name } => {
                write!(
                    f,
                    "the message or enum type '{}' is defined multiple times",
                    name
                )
            }
            DescriptorErrorKind::UnknownSyntax { syntax } => {
                write!(f, "the syntax '{}' is not recognized", syntax)
            }
            DescriptorErrorKind::InvalidMapEntry { name } => {
                write!(f, "the map entry message '{}' is invalid", name)
            }
            DescriptorErrorKind::InvalidDefaultValue { name, field, value } => {
                write!(
                    f,
                    "the default value '{}' for field '{}' of message '{}' is invalid",
                    value, field, name
                )
            }
            DescriptorErrorKind::EmptyEnum => write!(f, "enums must have at least one value"),
            DescriptorErrorKind::InvalidOneofIndex { name, field } => {
                write!(
                    f,
                    "the oneof index for field '{}' of message '{}' is invalid",
                    field, name
                )
            }
            DescriptorErrorKind::FileAlreadyExists { name } => write!(f, "a conflicting file named '{}' is already added. Duplicate files must match exactly.", name),
        }
    }
}
