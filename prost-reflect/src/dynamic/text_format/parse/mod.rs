mod error;
mod lex;

use std::{borrow::Cow, convert::TryFrom, iter::once};

use logos::{Lexer, Logos, Span};
use prost::Message;

pub use self::error::ParseError;

use self::{
    error::ParseErrorKind,
    lex::{Int, Token},
};
use crate::{
    descriptor::{MAP_ENTRY_KEY_NUMBER, MAP_ENTRY_VALUE_NUMBER},
    dynamic::fields::FieldDescriptorLike,
    DynamicMessage, EnumDescriptor, FieldDescriptor, Kind, MapKey, MessageDescriptor, Value,
};

pub(in crate::dynamic::text_format) struct Parser<'a> {
    lexer: Lexer<'a, Token<'a>>,
    peek: Option<Result<(Token<'a>, Span), ParseErrorKind>>,
}

enum FieldName {
    Ident(String),
    Extension(String),
    Any(String, String),
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Parser {
            lexer: Token::lexer(input),
            peek: None,
        }
    }

    pub fn parse_message(&mut self, message: &mut DynamicMessage) -> Result<(), ParseErrorKind> {
        while self.peek()?.is_some() {
            self.parse_field(message)?;
        }
        Ok(())
    }

    fn parse_message_value(
        &mut self,
        message: &mut DynamicMessage,
    ) -> Result<Span, ParseErrorKind> {
        let (terminator, start) = match self.peek()? {
            Some((Token::LeftBrace, _)) => (Token::RightBrace, self.bump()),
            Some((Token::LeftAngleBracket, _)) => (Token::RightAngleBracket, self.bump()),
            _ => self.unexpected_token("'{' or '<'")?,
        };

        loop {
            match self.peek()? {
                Some((Token::Ident(_) | Token::LeftBracket, _)) => self.parse_field(message)?,
                Some((tok, _)) if tok == terminator => {
                    let end = self.bump();
                    return Ok(join_span(start, end));
                }
                _ => self.unexpected_token(format!("'{}' or a field name", terminator))?,
            }
        }
    }

    fn parse_field(&mut self, message: &mut DynamicMessage) -> Result<(), ParseErrorKind> {
        let (name, span) = self.parse_field_name()?;

        match self.peek()? {
            Some((Token::Colon, _)) => {
                self.bump();
            }
            Some((Token::LeftBrace | Token::LeftAngleBracket, _)) => (),
            _ => self.unexpected_token("':' or a message value")?,
        };

        match name {
            FieldName::Ident(field_name) => {
                let field = find_field(&message.desc, &field_name).ok_or_else(|| {
                    ParseErrorKind::FieldNotFound {
                        field_name,
                        message_name: message.desc.full_name().to_owned(),
                        span,
                    }
                })?;

                self.parse_field_value(message, &field)?;
            }
            FieldName::Extension(extension_name) => {
                let extension = message
                    .desc
                    .get_extension_by_full_name(&extension_name)
                    .ok_or_else(|| ParseErrorKind::ExtensionNotFound {
                        extension_name,
                        message_name: message.desc.full_name().to_owned(),
                        span,
                    })?;

                self.parse_field_value(message, &extension)?;
            }
            FieldName::Any(domain, message_name) => {
                let value_message = match message
                    .desc
                    .parent_pool()
                    .get_message_by_name(&message_name)
                {
                    Some(msg) => msg,
                    None => return Err(ParseErrorKind::MessageNotFound { message_name, span }),
                };

                let mut value = DynamicMessage::new(value_message);
                self.parse_message_value(&mut value)?;

                let type_url = format!("{}/{}", domain, message_name);
                let value = value.encode_to_vec();

                if !(message.desc.full_name() == "google.protobuf.Any"
                    && message
                        .try_set_field_by_number(1, Value::String(type_url))
                        .is_ok()
                    && message
                        .try_set_field_by_number(2, Value::Bytes(value.into()))
                        .is_ok())
                {
                    return Err(ParseErrorKind::InvalidTypeForAny { span });
                }
            }
        }

        if matches!(self.peek()?, Some((Token::Comma | Token::Semicolon, _))) {
            self.bump();
        }

        Ok(())
    }

    fn parse_field_name(&mut self) -> Result<(FieldName, Span), ParseErrorKind> {
        match self.peek()? {
            Some((Token::Ident(ident), _)) => Ok((FieldName::Ident(ident.to_owned()), self.bump())),
            Some((Token::LeftBracket, _)) => {
                let start = self.bump();

                let name_or_domain = self
                    .parse_full_ident(&[Token::RightBracket, Token::ForwardSlash])?
                    .into_owned();
                match self.peek()? {
                    Some((Token::RightBracket, _)) => {
                        let end = self.bump();
                        Ok((FieldName::Extension(name_or_domain), join_span(start, end)))
                    }
                    Some((Token::ForwardSlash, _)) => {
                        self.bump();
                        let type_name = self.parse_full_ident(&[Token::RightBracket])?;
                        let end = self.expect(Token::RightBracket)?;
                        Ok((
                            FieldName::Any(name_or_domain, type_name.into_owned()),
                            join_span(start, end),
                        ))
                    }
                    _ => self.unexpected_token("']' or '/'")?,
                }
            }
            _ => self.unexpected_token("a field name")?,
        }
    }

    fn parse_field_value(
        &mut self,
        message: &mut DynamicMessage,
        field: &impl FieldDescriptorLike,
    ) -> Result<(), ParseErrorKind> {
        if field.is_list() {
            let (value, _) = self.parse_repeated_value(&field.kind())?;
            let result = message.fields.get_mut(field).as_list_mut().unwrap();
            if let Value::List(values) = value {
                result.extend(values);
            } else {
                result.push(value);
            }
            Ok(())
        } else if field.is_map() {
            fn unpack(value: Value) -> Result<(MapKey, Value), ParseErrorKind> {
                match value {
                    Value::Message(msg) => {
                        let key = msg
                            .get_field_by_number(MAP_ENTRY_KEY_NUMBER)
                            .unwrap()
                            .into_owned()
                            .into_map_key()
                            .ok_or(ParseErrorKind::InvalidMapKey)?;
                        let value = msg
                            .get_field_by_number(MAP_ENTRY_VALUE_NUMBER)
                            .unwrap()
                            .into_owned();
                        Ok((key, value))
                    }
                    _ => panic!("map entry must be message"),
                }
            }

            let (value, _) = self.parse_repeated_value(&field.kind())?;
            let result = message.fields.get_mut(field).as_map_mut().unwrap();
            if let Value::List(values) = value {
                for value in values {
                    let (key, value) = unpack(value)?;
                    result.insert(key, value);
                }
            } else {
                let (key, value) = unpack(value)?;
                result.insert(key, value);
            }
            Ok(())
        } else {
            let kind = field.kind();
            let (value, span) = self.parse_value(&kind)?;

            if message.fields.has(field) {
                return Err(ParseErrorKind::FieldAlreadySet {
                    field_name: field.text_name().to_owned(),
                    span,
                });
            } else if let Some(oneof) = field.containing_oneof() {
                for oneof_field in oneof.fields() {
                    if message.has_field(&oneof_field) {
                        return Err(ParseErrorKind::OneofAlreadySet {
                            oneof_name: oneof.name().to_owned(),
                            span,
                        });
                    }
                }
            }
            message.fields.set(field, value);
            Ok(())
        }
    }

    fn parse_repeated_value(&mut self, kind: &Kind) -> Result<(Value, Span), ParseErrorKind> {
        match self.peek()? {
            Some((Token::LeftBracket, _)) => {
                let start = self.bump();

                let mut result = Vec::new();

                // Check for empty list first
                if let Some((Token::RightBracket, _)) = self.peek()? {
                    let end = self.bump();
                    return Ok((Value::List(result), join_span(start, end)));
                }

                result.push(self.parse_value(kind)?.0);

                loop {
                    match self.peek()? {
                        Some((Token::Comma, _)) => {
                            self.bump();
                            result.push(self.parse_value(kind)?.0);
                        }
                        Some((Token::RightBracket, _)) => {
                            let end = self.bump();
                            return Ok((Value::List(result), join_span(start, end)));
                        }
                        _ => self.unexpected_token("',' or ']'")?,
                    }
                }
            }
            _ => self.parse_value(kind),
        }
    }

    fn parse_value(&mut self, kind: &Kind) -> Result<(Value, Span), ParseErrorKind> {
        match kind {
            Kind::Float => {
                let (value, span) = self.parse_float()?;
                Ok((Value::F32(value as f32), span))
            }
            Kind::Double => {
                let (value, span) = self.parse_float()?;
                Ok((Value::F64(value), span))
            }
            Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => {
                let (value, span) = self.parse_i32()?;
                Ok((Value::I32(value), span))
            }
            Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => {
                let (value, span) = self.parse_i64()?;
                Ok((Value::I64(value), span))
            }
            Kind::Uint32 | Kind::Fixed32 => {
                let (value, span) = self.parse_u32()?;
                Ok((Value::U32(value), span))
            }
            Kind::Uint64 | Kind::Fixed64 => {
                let (value, span) = self.parse_u64()?;
                Ok((Value::U64(value), span))
            }
            Kind::Bool => {
                let (value, span) = self.parse_bool()?;
                Ok((Value::Bool(value), span))
            }
            Kind::String => {
                let (value, span) = self.parse_bytes()?;
                match String::from_utf8(value) {
                    Ok(value) => Ok((Value::String(value), span)),
                    Err(_) => Err(ParseErrorKind::InvalidUtf8String { span }),
                }
            }
            Kind::Bytes => {
                let (value, span) = self.parse_bytes()?;
                Ok((Value::Bytes(value.into()), span))
            }
            Kind::Message(desc) => {
                let mut message = DynamicMessage::new(desc.clone());
                let span = self.parse_message_value(&mut message)?;
                Ok((Value::Message(message), span))
            }
            Kind::Enum(desc) => {
                let (value, span) = self.parse_enum(desc)?;
                Ok((Value::EnumNumber(value), span))
            }
        }
    }

    fn parse_float(&mut self) -> Result<(f64, Span), ParseErrorKind> {
        let (negative, start) = match self.peek()? {
            Some((Token::Minus, _)) => (true, self.bump()),
            Some((_, span)) => (false, span),
            None => self.unexpected_token("a number")?,
        };

        let (value, end) = match self.peek()? {
            Some((Token::FloatLiteral(value), _)) => (value, self.bump()),
            Some((Token::IntLiteral(Int { value, radix: 10 }), _)) => {
                (value.parse().unwrap(), self.bump())
            }
            Some((Token::Ident(value), _))
                if value.eq_ignore_ascii_case("inf") || value.eq_ignore_ascii_case("infinity") =>
            {
                (f64::INFINITY, self.bump())
            }
            Some((Token::Ident(value), _)) if value.eq_ignore_ascii_case("nan") => {
                (f64::NAN, self.bump())
            }
            _ => self.unexpected_token("a number")?,
        };

        if negative {
            Ok((-value, join_span(start, end)))
        } else {
            Ok((value, join_span(start, end)))
        }
    }

    fn parse_i32(&mut self) -> Result<(i32, Span), ParseErrorKind> {
        let (negative, int, span) = self.parse_int()?;
        let converted_value = if negative {
            u32::from_str_radix(int.value, int.radix)
                .ok()
                .and_then(|value| {
                    if value == (i32::MAX as u32 + 1) {
                        Some(i32::MIN)
                    } else {
                        i32::try_from(value).map(|value| -value).ok()
                    }
                })
        } else {
            i32::from_str_radix(int.value, int.radix).ok()
        };

        match converted_value {
            Some(value) => Ok((value, span)),
            None => Err(ParseErrorKind::IntegerValueOutOfRange {
                expected: "a signed 32-bit integer".to_owned(),
                actual: if negative {
                    format!("-{}", int.value)
                } else {
                    int.value.to_owned()
                },
                min: i32::MIN.to_string(),
                max: i32::MAX.to_string(),
                span,
            }),
        }
    }

    fn parse_i64(&mut self) -> Result<(i64, Span), ParseErrorKind> {
        let (negative, int, span) = self.parse_int()?;
        let converted_value = if negative {
            u64::from_str_radix(int.value, int.radix)
                .ok()
                .and_then(|value| {
                    if value == (i64::MAX as u64 + 1) {
                        Some(i64::MIN)
                    } else {
                        i64::try_from(value).map(|value| -value).ok()
                    }
                })
        } else {
            i64::from_str_radix(int.value, int.radix).ok()
        };

        match converted_value {
            Some(value) => Ok((value, span)),
            None => Err(ParseErrorKind::IntegerValueOutOfRange {
                expected: "a signed 64-bit integer".to_owned(),
                actual: if negative {
                    format!("-{}", int.value)
                } else {
                    int.value.to_owned()
                },
                min: i64::MIN.to_string(),
                max: i64::MAX.to_string(),
                span,
            }),
        }
    }

    fn parse_u32(&mut self) -> Result<(u32, Span), ParseErrorKind> {
        let (negative, int, span) = self.parse_int()?;
        let converted_value = if negative {
            None
        } else {
            u32::from_str_radix(int.value, int.radix).ok()
        };

        match converted_value {
            Some(value) => Ok((value, span)),
            None => Err(ParseErrorKind::IntegerValueOutOfRange {
                expected: "an unsigned 32-bit integer".to_owned(),
                actual: if negative {
                    format!("-{}", int.value)
                } else {
                    int.value.to_string()
                },
                min: u32::MIN.to_string(),
                max: u32::MAX.to_string(),
                span,
            }),
        }
    }

    fn parse_u64(&mut self) -> Result<(u64, Span), ParseErrorKind> {
        let (negative, int, span) = self.parse_int()?;
        let converted_value = if negative {
            None
        } else {
            u64::from_str_radix(int.value, int.radix).ok()
        };

        match converted_value {
            Some(value) => Ok((value, span)),
            None => Err(ParseErrorKind::IntegerValueOutOfRange {
                expected: "an unsigned 64-bit integer".to_owned(),
                actual: if negative {
                    format!("-{}", int.value)
                } else {
                    int.value.to_string()
                },
                min: u64::MIN.to_string(),
                max: u64::MAX.to_string(),
                span,
            }),
        }
    }

    fn parse_int(&mut self) -> Result<(bool, Int<'a>, Span), ParseErrorKind> {
        let (negative, start) = match self.peek()? {
            Some((Token::Minus, _)) => (true, self.bump()),
            Some((_, span)) => (false, span),
            None => self.unexpected_token("an integer")?,
        };

        let (value, end) = match self.peek()? {
            Some((Token::IntLiteral(value), _)) => (value, self.bump()),
            _ => self.unexpected_token("an integer")?,
        };

        Ok((negative, value, join_span(start, end)))
    }

    fn parse_bool(&mut self) -> Result<(bool, Span), ParseErrorKind> {
        match self.peek()? {
            Some((Token::Ident("false"), _))
            | Some((Token::Ident("False"), _))
            | Some((Token::Ident("f"), _)) => Ok((false, self.bump())),
            Some((Token::Ident("true"), _))
            | Some((Token::Ident("True"), _))
            | Some((Token::Ident("t"), _)) => Ok((true, self.bump())),
            Some((Token::IntLiteral(v), _)) => {
                let value = match u8::from_str_radix(v.value, v.radix) {
                    Ok(v) => v,
                    Err(_e) => return self.unexpected_token("0 or 1"),
                };
                if value == 1 {
                    Ok((true, self.bump()))
                } else if value == 0 {
                    Ok((false, self.bump()))
                } else {
                    self.unexpected_token("0 or 1")
                }
            }
            _ => self.unexpected_token("'true' or 'false'"),
        }
    }

    fn parse_bytes(&mut self) -> Result<(Vec<u8>, Span), ParseErrorKind> {
        let (mut result, mut span) = match self.peek()? {
            Some((Token::StringLiteral(value), _)) => (value, self.bump()),
            _ => self.unexpected_token("a string")?,
        };

        while let Some((Token::StringLiteral(value), _)) = self.peek()? {
            result.extend_from_slice(&value);
            span = join_span(span, self.bump());
        }

        Ok((result, span))
    }

    fn parse_enum(&mut self, desc: &EnumDescriptor) -> Result<(i32, Span), ParseErrorKind> {
        match self.peek()? {
            Some((Token::Ident(name), _)) => {
                let span = self.bump();
                if let Some(value) = desc.get_value_by_name(name) {
                    Ok((value.number(), span))
                } else {
                    Err(ParseErrorKind::EnumValueNotFound {
                        value_name: name.to_owned(),
                        enum_name: desc.full_name().to_owned(),
                        span,
                    })
                }
            }
            Some((Token::Minus | Token::IntLiteral(_), _)) => self.parse_i32(),
            _ => self.unexpected_token("an enum value")?,
        }
    }

    fn parse_full_ident(&mut self, terminators: &[Token]) -> Result<Cow<'a, str>, ParseErrorKind> {
        let mut result = match self.peek()? {
            Some((Token::Ident(ident), _)) => Cow::Borrowed(ident),
            _ => self.unexpected_token("an identifier")?,
        };
        self.bump();

        loop {
            match self.peek()? {
                Some((Token::Dot, _)) => {
                    self.bump();
                }
                Some((tok, _)) if terminators.contains(&tok) => return Ok(result),
                _ => self.unexpected_token(fmt_expected(
                    once(Token::Dot).chain(terminators.iter().cloned()),
                ))?,
            }

            match self.peek()? {
                Some((Token::Ident(ident), _)) => {
                    let result = result.to_mut();
                    result.push('.');
                    result.push_str(ident);
                    self.bump();
                }
                _ => self.unexpected_token("an identifier")?,
            };
        }
    }

    fn expect(&mut self, expected: Token) -> Result<Span, ParseErrorKind> {
        if let Some((tok, _)) = self.peek()? {
            if tok == expected {
                return Ok(self.bump());
            }
        };

        self.unexpected_token(expected)?
    }

    fn bump(&mut self) -> Span {
        let (_, span) = self
            .peek
            .take()
            .expect("called bump without peek returning Some()")
            .expect("called bump on invalid token");
        span
    }

    fn peek(&mut self) -> Result<Option<(Token<'a>, Span)>, ParseErrorKind> {
        if self.peek.is_none() {
            self.peek = self.next();
        }
        self.peek.clone().transpose()
    }

    fn next(&mut self) -> Option<Result<(Token<'a>, Span), ParseErrorKind>> {
        debug_assert!(self.peek.is_none());
        match self.lexer.next() {
            Some(Err(())) => Some(Err(self.lexer.extras.error.take().unwrap_or_else(|| {
                ParseErrorKind::InvalidToken {
                    span: self.lexer.span(),
                }
            }))),
            Some(Ok(tok)) => Some(Ok((tok, self.lexer.span()))),
            None => None,
        }
    }

    fn unexpected_token<T>(&mut self, expected: impl ToString) -> Result<T, ParseErrorKind> {
        match self.peek()? {
            Some((found, span)) => Err(ParseErrorKind::UnexpectedToken {
                expected: expected.to_string(),
                found: found.to_string(),
                span,
            }),
            None => Err(ParseErrorKind::UnexpectedEof {
                expected: expected.to_string(),
            }),
        }
    }
}

fn find_field(desc: &MessageDescriptor, name: &str) -> Option<FieldDescriptor> {
    if let Some(field) = desc.get_field_by_name(name) {
        if !field.is_group() {
            return Some(field);
        }
    }

    if let Some(field) = desc.get_field_by_name(&name.to_ascii_lowercase()) {
        if field.is_group() && name == field.kind().as_message().unwrap().name() {
            return Some(field);
        }
    }

    None
}

fn fmt_expected<'a>(ts: impl Iterator<Item = Token<'a>>) -> String {
    use std::fmt::Write;

    let ts: Vec<_> = ts.collect();

    let mut s = String::with_capacity(32);
    write!(s, "'{}'", ts[0]).unwrap();
    if ts.len() > 1 {
        for t in &ts[1..][..ts.len() - 2] {
            s.push_str(", ");
            write!(s, "'{}'", t).unwrap();
        }
        s.push_str(" or ");
        write!(s, "'{}'", ts[ts.len() - 1]).unwrap();
    }
    s
}

fn join_span(start: Span, end: Span) -> Span {
    start.start..end.end
}
