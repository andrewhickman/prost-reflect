use logos::Span;
use std::{
    error::Error,
    fmt::{self, Display},
};

/// An error that may occur while parsing the protobuf text format.
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "text-format")))]
pub struct ParseError {
    kind: Box<ParseErrorKind>,
    #[cfg(feature = "miette")]
    source: String,
}

impl ParseError {
    #[cfg(feature = "miette")]
    pub(crate) fn new(kind: ParseErrorKind, source: &str) -> Self {
        ParseError {
            kind: Box::new(kind),
            source: source.to_owned(),
        }
    }

    #[cfg(not(feature = "miette"))]
    pub(crate) fn new(kind: ParseErrorKind, _: &str) -> Self {
        ParseError {
            kind: Box::new(kind),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub(crate) enum ParseErrorKind {
    InvalidToken {
        #[cfg_attr(feature = "miette", label("found here"))]
        span: Span,
    },
    InvalidStringCharacters {
        #[cfg_attr(feature = "miette", label("invalid characters"))]
        span: Span,
    },
    InvalidStringEscape {
        #[cfg_attr(feature = "miette", label("defined here"))]
        span: Span,
    },
    InvalidUtf8String {
        #[cfg_attr(feature = "miette", label("defined here"))]
        span: Span,
    },
    NoSpaceBetweenIntAndIdent {
        #[cfg_attr(feature = "miette", label("found here"))]
        span: Span,
    },
    UnexpectedToken {
        expected: String,
        found: String,
        #[cfg_attr(feature = "miette", label("found here"))]
        span: Span,
    },
    UnexpectedEof {
        expected: String,
    },
    #[cfg_attr(
        feature = "miette",
        diagnostic(help("the value must be between {min} and {max} inclusive"))
    )]
    IntegerValueOutOfRange {
        expected: String,
        actual: String,
        min: String,
        max: String,
        #[cfg_attr(feature = "miette", label("defined here"))]
        span: Span,
    },
    FieldNotFound {
        field_name: String,
        message_name: String,
        #[cfg_attr(feature = "miette", label("set here"))]
        span: Span,
    },
    FieldAlreadySet {
        field_name: String,
        #[cfg_attr(feature = "miette", label("set here"))]
        span: Span,
    },
    OneofAlreadySet {
        oneof_name: String,
        #[cfg_attr(feature = "miette", label("set here"))]
        span: Span,
    },
    ExtensionNotFound {
        extension_name: String,
        message_name: String,
        #[cfg_attr(feature = "miette", label("set here"))]
        span: Span,
    },
    MessageNotFound {
        message_name: String,
        #[cfg_attr(feature = "miette", label("used here"))]
        span: Span,
    },
    EnumValueNotFound {
        value_name: String,
        enum_name: String,
        #[cfg_attr(feature = "miette", label("used here"))]
        span: Span,
    },
    InvalidTypeForAny {
        #[cfg_attr(feature = "miette", label("used here"))]
        span: Span,
    },
    InvalidMapKey,
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseErrorKind::InvalidToken { .. } => write!(f, "invalid token"),
            ParseErrorKind::InvalidStringCharacters { .. } => write!(f, "invalid string character"),
            ParseErrorKind::InvalidStringEscape { .. } => write!(f, "invalid string escape"),
            ParseErrorKind::InvalidUtf8String { .. } => write!(f, "string is not valid utf-8"),
            ParseErrorKind::NoSpaceBetweenIntAndIdent { .. } => write!(
                f,
                "whitespace is required between an integer literal and an identifier"
            ),
            ParseErrorKind::UnexpectedToken {
                expected, found, ..
            } => write!(f, "expected {}, but found '{}'", expected, found),
            ParseErrorKind::UnexpectedEof { expected, .. } => {
                write!(f, "expected {}, but reached end of input", expected)
            }
            ParseErrorKind::IntegerValueOutOfRange {
                expected, actual, ..
            } => write!(
                f,
                "expected value to be {}, but the value {} is out of range",
                expected, actual
            ),
            ParseErrorKind::FieldNotFound {
                field_name,
                message_name,
                ..
            } => write!(
                f,
                "field '{}' not found for message '{}'",
                field_name, message_name
            ),
            ParseErrorKind::FieldAlreadySet { field_name, .. } => {
                write!(f, "'{}' is already set", field_name,)
            }
            ParseErrorKind::OneofAlreadySet { oneof_name, .. } => {
                write!(f, "a value is already set for oneof '{}'", oneof_name)
            }
            ParseErrorKind::ExtensionNotFound {
                extension_name,
                message_name,
                ..
            } => {
                write!(
                    f,
                    "extension '{}' not found for message '{}'",
                    extension_name, message_name
                )
            }
            ParseErrorKind::MessageNotFound { message_name, .. } => {
                write!(f, "message type '{}' not found", message_name)
            }
            ParseErrorKind::EnumValueNotFound {
                value_name,
                enum_name,
                ..
            } => {
                write!(
                    f,
                    "value '{}' was not found for enum '{}'",
                    value_name, enum_name
                )
            }
            ParseErrorKind::InvalidTypeForAny { .. } => write!(
                f,
                "the field type must be 'google.protobuf.Any' to use Any expansion syntax"
            ),
            ParseErrorKind::InvalidMapKey { .. } => write!(f, "invalid value type for map key"),
        }
    }
}

impl Error for ParseErrorKind {}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl Error for ParseError {}

#[cfg(feature = "miette")]
#[cfg_attr(docsrs, doc(cfg(feature = "miette")))]
impl miette::Diagnostic for ParseError {
    fn code<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.kind.code()
    }

    fn severity(&self) -> Option<miette::Severity> {
        self.kind.severity()
    }

    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.kind.help()
    }

    fn url<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.kind.url()
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.source)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        self.kind.labels()
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        self.kind.related()
    }

    fn diagnostic_source(&self) -> Option<&dyn miette::Diagnostic> {
        self.kind.diagnostic_source()
    }
}
