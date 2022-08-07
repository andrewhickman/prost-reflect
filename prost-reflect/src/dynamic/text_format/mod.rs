mod format;
#[cfg(feature = "text-format")]
mod parse;

use std::fmt;

#[cfg(feature = "text-format")]
pub use self::parse::ParseError;
#[cfg(feature = "text-format")]
use crate::MessageDescriptor;

use crate::{DynamicMessage, Value};

/// Options to control printing of the protobuf text format.
///
/// Used by [`DynamicMessage::to_text_format_with_options()`].
#[derive(Debug, Clone)]
#[cfg_attr(docsrs, doc(cfg(feature = "text-format")))]
pub struct FormatOptions {
    pretty: bool,
    skip_unknown_fields: bool,
    expand_any: bool,
}

#[cfg(feature = "text-format")]
impl DynamicMessage {
    /// Parse a [`DynamicMessage`] from the given message encoded using the [text format](https://developers.google.com/protocol-buffers/docs/text-format-spec).
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value};
    /// # let pool = DescriptorPool::decode(include_bytes!("../../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("package.MyMessage").unwrap();
    /// let dynamic_message = DynamicMessage::parse_text_format(message_descriptor, "foo: 150").unwrap();
    /// assert_eq!(dynamic_message.get_field_by_name("foo").unwrap().as_ref(), &Value::I32(150));
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "text-format")))]
    pub fn parse_text_format(desc: MessageDescriptor, input: &str) -> Result<Self, ParseError> {
        let mut message = DynamicMessage::new(desc);
        message.merge_text_format(input)?;
        Ok(message)
    }

    /// Merges the given message encoded using the [text format](https://developers.google.com/protocol-buffers/docs/text-format-spec) into this message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value};
    /// # let pool = DescriptorPool::decode(include_bytes!("../../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("package.MyMessage").unwrap();
    /// let mut dynamic_message = DynamicMessage::new(message_descriptor);
    /// dynamic_message.merge_text_format("foo: 150").unwrap();
    /// assert_eq!(dynamic_message.get_field_by_name("foo").unwrap().as_ref(), &Value::I32(150));
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "text-format")))]
    pub fn merge_text_format(&mut self, input: &str) -> Result<(), ParseError> {
        parse::Parser::new(input)
            .parse_message(self)
            .map_err(ParseError::new)
    }

    /// Formats this dynamic message using the protobuf text format, with default options.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_types::FileDescriptorSet;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value, text_format::FormatOptions};
    /// # let pool = DescriptorPool::decode(include_bytes!("../../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("package.MyMessage").unwrap();
    /// let dynamic_message = DynamicMessage::decode(message_descriptor, b"\x08\x96\x01\x1a\x02\x10\x42".as_ref()).unwrap();
    /// assert_eq!(dynamic_message.to_text_format(), "foo:150,nested{bar:66}");
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "text-format")))]
    pub fn to_text_format(&self) -> String {
        self.to_text_format_with_options(&FormatOptions::new())
    }

    /// Formats this dynamic message using the protobuf text format, with custom options.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_types::FileDescriptorSet;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value, text_format::FormatOptions};
    /// # let pool = DescriptorPool::decode(include_bytes!("../../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("package.MyMessage").unwrap();
    /// let dynamic_message = DynamicMessage::decode(message_descriptor, b"\x08\x96\x01\x1a\x02\x10\x42".as_ref()).unwrap();
    /// let options = FormatOptions::new().pretty(true);
    /// assert_eq!(dynamic_message.to_text_format_with_options(&options), "foo: 150\nnested {\n  bar: 66\n}");
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "text-format")))]
    pub fn to_text_format_with_options(&self, options: &FormatOptions) -> String {
        let mut result = String::new();
        format::Writer::new(options.clone(), &mut result)
            .fmt_message(self)
            .expect("writing to string cannot fail");
        result
    }
}

impl FormatOptions {
    /// Creates new instance of [`FormatOptions`] with default options.
    pub fn new() -> Self {
        FormatOptions::default()
    }

    /// Whether to prettify the format output.
    ///
    /// If set to `true`, each field will be printed on a new line, and nested messages will be indented.
    ///
    /// The default value is `false`.
    pub fn pretty(mut self, yes: bool) -> Self {
        self.pretty = yes;
        self
    }

    /// Whether to include unknown fields in the output.
    ///
    /// If set to `false`, unknown fields will be printed. The protobuf format does not include type information,
    /// so the formatter will attempt to infer types.
    ///
    /// The default value is `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_types::FileDescriptorSet;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value, text_format::FormatOptions};
    /// # let pool = DescriptorPool::decode(include_bytes!("../../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("google.protobuf.Empty").unwrap();
    /// let dynamic_message = DynamicMessage::decode(message_descriptor, b"\x08\x96\x01\x1a\x02\x10\x42".as_ref()).unwrap();
    /// assert_eq!(dynamic_message.to_text_format(), "");
    /// let options = FormatOptions::new().skip_unknown_fields(false);
    /// assert_eq!(dynamic_message.to_text_format_with_options(&options), "1:150,3{2:66}");
    /// ```
    #[cfg(feature = "text-format")]
    pub fn skip_unknown_fields(mut self, yes: bool) -> Self {
        self.skip_unknown_fields = yes;
        self
    }

    /// Whether to use the expanded form of the `google.protobuf.Any` type.
    ///
    /// If set to `true`, `Any` fields will use an expanded form:
    ///
    /// ```textproto
    /// [type.googleapis.com/package.MyMessage] {
    ///   foo: 150
    /// }
    /// ```
    ///
    /// If set to `false`, the normal text format representation will be used:
    ///
    /// ```textproto
    /// type_url: "type.googleapis.com/package.MyMessage"
    /// value: "\x08\x96\x01"
    /// ```
    ///
    /// The default value is `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_types::FileDescriptorSet;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value, text_format::FormatOptions, bytes::Bytes};
    /// # let pool = DescriptorPool::decode(include_bytes!("../../file_descriptor_set.bin").as_ref()).unwrap();
    /// let message_descriptor = pool.get_message_by_name("google.protobuf.Any").unwrap();
    /// let mut dynamic_message = DynamicMessage::new(message_descriptor);
    /// dynamic_message.set_field_by_name("type_url", Value::String("type.googleapis.com/package.MyMessage".to_owned()));
    /// dynamic_message.set_field_by_name("value", Value::Bytes(Bytes::from_static(b"\x08\x96\x01\x1a\x02\x10\x42".as_ref())));
    ///
    /// assert_eq!(dynamic_message.to_text_format(), "[type.googleapis.com/package.MyMessage]{foo:150,nested{bar:66}}");
    /// let options = FormatOptions::new().expand_any(false);
    /// assert_eq!(dynamic_message.to_text_format_with_options(&options), r#"type_url:"type.googleapis.com/package.MyMessage",value:"\010\226\001\032\002\020B""#);
    /// ```
    #[cfg(feature = "text-format")]
    pub fn expand_any(mut self, yes: bool) -> Self {
        self.expand_any = yes;
        self
    }
}

impl Default for FormatOptions {
    fn default() -> Self {
        FormatOptions {
            pretty: false,
            skip_unknown_fields: true,
            expand_any: true,
        }
    }
}

impl fmt::Display for DynamicMessage {
    /// Formats this message using the protobuf text format.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prost::Message;
    /// # use prost_types::FileDescriptorSet;
    /// # use prost_reflect::{DynamicMessage, DescriptorPool, Value};
    /// # let pool = DescriptorPool::decode(include_bytes!("../../file_descriptor_set.bin").as_ref()).unwrap();
    /// # let message_descriptor = pool.get_message_by_name("package.MyMessage").unwrap();
    /// let dynamic_message = DynamicMessage::decode(message_descriptor, b"\x08\x96\x01\x1a\x02\x10\x42".as_ref()).unwrap();
    /// assert_eq!(format!("{}", dynamic_message), "foo:150,nested{bar:66}");
    /// // The alternate format specifier may be used to pretty-print the output
    /// assert_eq!(format!("{:#}", dynamic_message), "foo: 150\nnested {\n  bar: 66\n}");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format::Writer::new(FormatOptions::new().pretty(f.alternate()), f).fmt_message(self)
    }
}

impl fmt::Display for Value {
    /// Formats this value using the protobuf text format.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::{collections::HashMap, iter::FromIterator};
    /// # use prost_reflect::{MapKey, Value};
    /// assert_eq!(format!("{}", Value::String("hello".to_owned())), "\"hello\"");
    /// assert_eq!(format!("{}", Value::List(vec![Value::I32(1), Value::I32(2)])), "[1,2]");
    /// assert_eq!(format!("{}", Value::Map(HashMap::from_iter([(MapKey::I32(1), Value::U32(2))]))), "[{key:1,value:2}]");
    /// // The alternate format specifier may be used to indent the output
    /// assert_eq!(format!("{:#}", Value::Map(HashMap::from_iter([(MapKey::I32(1), Value::U32(2))]))), "[{\n  key: 1\n  value: 2\n}]");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format::Writer::new(FormatOptions::new().pretty(f.alternate()), f).fmt_value(self, None)
    }
}
