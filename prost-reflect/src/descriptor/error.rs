use std::fmt;

use crate::descriptor::{FileDescriptorInner, FileIndex};

/// An error that may occur while creating a [`DescriptorPool`][crate::DescriptorPool].
#[derive(Debug)]
pub struct DescriptorError {
    inner: Box<[DescriptorErrorKind]>,
    #[cfg(feature = "miette")]
    source: Option<String>,
}

#[derive(Debug)]
pub(super) enum DescriptorErrorKind {
    MissingRequiredField {
        label: Label,
    },
    UnknownSyntax {
        syntax: String,
        found: Label,
    },
    DuplicateFileName {
        name: String,
    },
    FileNotFound {
        name: String,
        found: Label,
    },
    InvalidImportIndex,
    InvalidOneofIndex,
    DuplicateName {
        name: String,
        first: Label,
        second: Label,
    },
    DuplicateFieldNumber {
        number: u32,
        #[cfg_attr(not(feature = "miette"), allow(dead_code))]
        first: Label,
        second: Label,
    },
    DuplicateFieldJsonName {
        name: String,
        #[cfg_attr(not(feature = "miette"), allow(dead_code))]
        first: Label,
        second: Label,
    },
    NameNotFound {
        name: String,
        found: Label,
    },
    InvalidType {
        name: String,
        expected: String,
        found: Label,
        #[cfg_attr(not(feature = "miette"), allow(dead_code))]
        defined: Label,
    },
    InvalidFieldDefault {
        value: String,
        kind: String,
        err: Option<Box<dyn std::error::Error>>,
        found: Label,
    },
    EmptyEnum {
        found: Label,
    },
    DuplicateEnumNumber {
        number: i32,
        #[cfg_attr(not(feature = "miette"), allow(dead_code))]
        first: Label,
        second: Label,
    },
    DecodeFileDescriptorSet {
        err: prost::DecodeError,
    },
}

#[derive(Debug)]
pub(super) struct Label {
    file: String,
    path: Box<[i32]>,
    span: Option<[i32; 4]>,
    #[cfg(feature = "miette")]
    message: String,
    #[cfg(feature = "miette")]
    resolved: Option<miette::SourceSpan>,
}

impl DescriptorError {
    pub(super) fn new(errors: Vec<DescriptorErrorKind>) -> DescriptorError {
        debug_assert!(!errors.is_empty());
        DescriptorError {
            inner: errors.into(),
            #[cfg(feature = "miette")]
            source: None,
        }
    }

    /// The primary file in which this error occurred.
    pub fn file(&self) -> Option<&str> {
        self.first().label().map(|l| l.file.as_str())
    }

    /// The 1-based line number at which this error occurred, if available.
    pub fn line(&self) -> Option<usize> {
        self.first()
            .label()
            .and_then(|l| l.span)
            .map(|s| s[0] as usize)
    }

    /// The 1-based column number at which this error occurred, if available.
    pub fn column(&self) -> Option<usize> {
        self.first()
            .label()
            .and_then(|l| l.span)
            .map(|s| s[1] as usize)
    }

    /// Gets the path where this error occurred in the [`FileDescriptorProto`][FileDescriptorProto], if available.
    ///
    /// See [`path`][prost_types::source_code_info::Location::path] for more details on the structure of the path.
    pub fn path(&self) -> Option<&[i32]> {
        self.first().label().map(|l| l.path.as_ref())
    }

    #[cfg(feature = "miette")]
    #[cfg_attr(docsrs, doc(cfg(feature = "miette")))]
    /// Provide source code information for this error.
    ///
    /// The source should correspond to the contents of [`file()`][DescriptorError::source].
    pub fn with_source_code(mut self, source: &str) -> Self {
        if let Some(file) = self.file() {
            let file = file.to_owned();

            self.source = Some(source.into());
            for error in self.inner.as_mut() {
                error.add_source_code(&file, source);
            }
        }
        self
    }

    fn first(&self) -> &DescriptorErrorKind {
        &self.inner[0]
    }
}

impl std::error::Error for DescriptorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.first().source()
    }
}

impl fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.first().fmt(f)
    }
}

#[cfg(feature = "miette")]
#[cfg_attr(docsrs, doc(cfg(feature = "miette")))]
impl miette::Diagnostic for DescriptorError {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.first().code()
    }

    fn severity(&self) -> Option<miette::Severity> {
        self.first().severity()
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.first().help()
    }

    fn url<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.first().url()
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        match &self.source {
            Some(source) => Some(source),
            None => None,
        }
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        self.first().labels()
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        if self.inner.len() > 1 {
            Some(Box::new(
                self.inner
                    .iter()
                    .map(|e| e as &dyn miette::Diagnostic)
                    .skip(1),
            ))
        } else {
            None
        }
    }

    fn diagnostic_source(&self) -> Option<&dyn miette::Diagnostic> {
        self.first().diagnostic_source()
    }
}

impl DescriptorErrorKind {
    fn label(&self) -> Option<&Label> {
        match self {
            DescriptorErrorKind::MissingRequiredField { label } => Some(label),
            DescriptorErrorKind::UnknownSyntax { found, .. } => Some(found),
            DescriptorErrorKind::DuplicateFileName { .. } => None,
            DescriptorErrorKind::FileNotFound { found, .. } => Some(found),
            DescriptorErrorKind::InvalidImportIndex => None,
            DescriptorErrorKind::InvalidOneofIndex => None,
            DescriptorErrorKind::DuplicateName { second, .. } => Some(second),
            DescriptorErrorKind::DuplicateFieldNumber { second, .. } => Some(second),
            DescriptorErrorKind::DuplicateFieldJsonName { second, .. } => Some(second),
            DescriptorErrorKind::NameNotFound { found, .. } => Some(found),
            DescriptorErrorKind::InvalidType { found, .. } => Some(found),
            DescriptorErrorKind::InvalidFieldDefault { found, .. } => Some(found),
            DescriptorErrorKind::EmptyEnum { found } => Some(found),
            DescriptorErrorKind::DuplicateEnumNumber { second, .. } => Some(second),
            DescriptorErrorKind::DecodeFileDescriptorSet { .. } => None,
        }
    }

    #[cfg(feature = "miette")]
    fn add_source_code(&mut self, file: &str, source: &str) {
        match self {
            DescriptorErrorKind::MissingRequiredField { label } => {
                label.resolve_span(file, source);
            }
            DescriptorErrorKind::UnknownSyntax { found, .. } => {
                found.resolve_span(file, source);
            }
            DescriptorErrorKind::DuplicateFileName { .. } => {}
            DescriptorErrorKind::FileNotFound { found, .. } => {
                found.resolve_span(file, source);
            }
            DescriptorErrorKind::InvalidImportIndex => {}
            DescriptorErrorKind::InvalidOneofIndex => {}
            DescriptorErrorKind::DuplicateName { first, second, .. } => {
                first.resolve_span(file, source);
                second.resolve_span(file, source);
            }
            DescriptorErrorKind::DuplicateFieldNumber { first, second, .. } => {
                first.resolve_span(file, source);
                second.resolve_span(file, source);
            }
            DescriptorErrorKind::DuplicateFieldJsonName { first, second, .. } => {
                first.resolve_span(file, source);
                second.resolve_span(file, source);
            }
            DescriptorErrorKind::NameNotFound { found, .. } => {
                found.resolve_span(file, source);
            }
            DescriptorErrorKind::InvalidType { found, defined, .. } => {
                found.resolve_span(file, source);
                defined.resolve_span(file, source);
            }
            DescriptorErrorKind::InvalidFieldDefault { found, .. } => {
                found.resolve_span(file, source);
            }
            DescriptorErrorKind::EmptyEnum { found } => {
                found.resolve_span(file, source);
            }
            DescriptorErrorKind::DuplicateEnumNumber { first, second, .. } => {
                first.resolve_span(file, source);
                second.resolve_span(file, source);
            }
            DescriptorErrorKind::DecodeFileDescriptorSet { .. } => {}
        }
    }
}

impl std::error::Error for DescriptorErrorKind {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DescriptorErrorKind::DecodeFileDescriptorSet { err } => Some(err),
            DescriptorErrorKind::InvalidFieldDefault { err, .. } => err.as_deref(),
            _ => None,
        }
    }
}

impl fmt::Display for DescriptorErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DescriptorErrorKind::MissingRequiredField { label } => {
                write!(f, "missing required field at {:?}", label.path)
            }
            DescriptorErrorKind::UnknownSyntax { syntax, .. } => {
                write!(f, "unknown syntax '{}'", syntax)
            }
            DescriptorErrorKind::DuplicateFileName { name, .. } => {
                write!(
                    f,
                    "a different file named '{}' has already been added",
                    name
                )
            }
            DescriptorErrorKind::FileNotFound { name, .. } => {
                write!(f, "imported file '{}' has not been added", name)
            }
            DescriptorErrorKind::InvalidImportIndex => {
                write!(f, "invalid import index")
            }
            DescriptorErrorKind::InvalidOneofIndex => {
                write!(f, "invalid oneof index")
            }
            DescriptorErrorKind::DuplicateName {
                name,
                first,
                second,
            } => {
                if first.file == second.file {
                    write!(f, "name '{}' is defined twice", name)
                } else {
                    write!(
                        f,
                        "name '{}' is already defined file '{}'",
                        name, first.file
                    )
                }
            }
            DescriptorErrorKind::DuplicateFieldNumber { number, .. } => {
                write!(f, "field number '{}' is already used", number)
            }
            DescriptorErrorKind::DuplicateFieldJsonName { name, .. } => {
                write!(f, "a field with JSON name '{}' is already defined", name)
            }
            DescriptorErrorKind::NameNotFound { name, .. } => {
                write!(f, "name '{}' is not defined", name)
            }
            DescriptorErrorKind::InvalidType { name, expected, .. } => {
                write!(f, "'{}' is not {}", name, expected)
            }
            DescriptorErrorKind::InvalidFieldDefault { value, kind, .. } => {
                write!(f, "invalid default value '{}' for type '{}'", value, kind)
            }
            DescriptorErrorKind::EmptyEnum { .. } => {
                write!(f, "enums must have at least one value")
            }
            DescriptorErrorKind::DuplicateEnumNumber { number, .. } => {
                write!(f, "enum number '{}' has already been used", number)
            }
            DescriptorErrorKind::DecodeFileDescriptorSet { .. } => {
                write!(f, "failed to decode file descriptor set")
            }
        }
    }
}

#[cfg(feature = "miette")]
#[cfg_attr(docsrs, doc(cfg(feature = "miette")))]
impl miette::Diagnostic for DescriptorErrorKind {
    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        match self {
            DescriptorErrorKind::MissingRequiredField { .. } => None,
            DescriptorErrorKind::UnknownSyntax { .. } => {
                Some(Box::new("valid values are 'proto2' and 'proto3'"))
            }
            DescriptorErrorKind::DuplicateFileName { .. } => None,
            DescriptorErrorKind::FileNotFound { .. } => None,
            DescriptorErrorKind::InvalidImportIndex => None,
            DescriptorErrorKind::InvalidOneofIndex => None,
            DescriptorErrorKind::DuplicateName { .. } => None,
            DescriptorErrorKind::DuplicateFieldNumber { .. } => None,
            DescriptorErrorKind::DuplicateFieldJsonName { .. } => None,
            DescriptorErrorKind::NameNotFound { .. } => None,
            DescriptorErrorKind::InvalidType { .. } => None,
            DescriptorErrorKind::InvalidFieldDefault { .. } => None,
            DescriptorErrorKind::EmptyEnum { .. } => None,
            DescriptorErrorKind::DuplicateEnumNumber { .. } => Some(Box::new(
                "set the 'allow_alias' option allow re-using enum numbers",
            )),
            DescriptorErrorKind::DecodeFileDescriptorSet { .. } => None,
        }
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let mut spans = Vec::new();
        match self {
            DescriptorErrorKind::MissingRequiredField { label } => spans.extend(label.to_span()),
            DescriptorErrorKind::UnknownSyntax { found: defined, .. } => {
                spans.extend(defined.to_span());
            }
            DescriptorErrorKind::DuplicateFileName { .. } => {}
            DescriptorErrorKind::FileNotFound { found, .. } => {
                spans.extend(found.to_span());
            }
            DescriptorErrorKind::InvalidImportIndex => {}
            DescriptorErrorKind::InvalidOneofIndex => {}
            DescriptorErrorKind::DuplicateName { first, second, .. } => {
                spans.extend(first.to_span());
                spans.extend(second.to_span());
            }
            DescriptorErrorKind::DuplicateFieldNumber { first, second, .. } => {
                spans.extend(first.to_span());
                spans.extend(second.to_span());
            }
            DescriptorErrorKind::DuplicateFieldJsonName { first, second, .. } => {
                spans.extend(first.to_span());
                spans.extend(second.to_span());
            }
            DescriptorErrorKind::NameNotFound { found, .. } => {
                spans.extend(found.to_span());
            }
            DescriptorErrorKind::InvalidType { found, defined, .. } => {
                spans.extend(found.to_span());
                spans.extend(defined.to_span());
            }
            DescriptorErrorKind::InvalidFieldDefault { found, .. } => {
                spans.extend(found.to_span());
            }
            DescriptorErrorKind::EmptyEnum { found } => {
                spans.extend(found.to_span());
            }
            DescriptorErrorKind::DuplicateEnumNumber { first, second, .. } => {
                spans.extend(first.to_span());
                spans.extend(second.to_span());
            }
            DescriptorErrorKind::DecodeFileDescriptorSet { .. } => {}
        }
        if spans.is_empty() {
            None
        } else {
            Some(Box::new(spans.into_iter()))
        }
    }
}

impl Label {
    pub fn new(
        files: &[FileDescriptorInner],
        #[cfg_attr(not(feature = "miette"), allow(unused_variables))] message: impl ToString,
        file: FileIndex,
        path: Box<[i32]>,
    ) -> Self {
        let file = &files[file as usize].raw;

        let span = file
            .source_code_info
            .as_ref()
            .and_then(|s| s.location.iter().find(|l| *l.path == *path))
            .and_then(|l| match *l.span {
                [start_line, start_col, end_col] => {
                    Some([start_line, start_col, start_line, end_col])
                }
                [start_line, start_col, end_line, end_col] => {
                    Some([start_line, start_col, end_line, end_col])
                }
                _ => None,
            });

        Label {
            file: file.name().to_owned(),
            span,
            path,
            #[cfg(feature = "miette")]
            message: message.to_string(),
            #[cfg(feature = "miette")]
            resolved: None,
        }
    }

    #[cfg(feature = "miette")]
    pub fn resolve_span(&mut self, file: &str, source: &str) {
        if file == self.file {
            if let Some([start_line, start_col, end_line, end_col]) = self.span {
                let start = miette::SourceOffset::from_location(
                    source,
                    start_line.saturating_add(1) as _,
                    start_col.saturating_add(1) as _,
                )
                .offset();
                let end = miette::SourceOffset::from_location(
                    source,
                    end_line.saturating_add(1) as _,
                    end_col.saturating_add(1) as _,
                )
                .offset();
                self.resolved = Some(miette::SourceSpan::from(start..end));
            }
        }
    }

    #[cfg(feature = "miette")]
    fn to_span(&self) -> Option<miette::LabeledSpan> {
        match self.resolved {
            Some(span) if !span.is_empty() => Some(miette::LabeledSpan::new_with_span(
                Some(self.message.clone()),
                span,
            )),
            _ => None,
        }
    }
}
