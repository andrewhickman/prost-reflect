use std::{ascii, convert::TryInto, fmt};

use logos::{skip, Lexer, Logos};

use super::error::ParseErrorKind;

#[derive(Debug, Clone, Logos, PartialEq)]
#[logos(extras = TokenExtras)]
#[logos(subpattern exponent = r"[eE][+\-]?[0-9]+")]
pub(crate) enum Token<'a> {
    #[regex("[A-Za-z_][A-Za-z0-9_]*")]
    Ident(&'a str),
    #[regex("0", |lex| int(lex, 10, 0))]
    #[regex("[1-9][0-9]*", |lex| int(lex, 10, 0))]
    #[regex("0[0-7]+", |lex| int(lex, 8, 1))]
    #[regex("0[xX][0-9A-Fa-f]+", |lex| int(lex, 16, 2))]
    IntLiteral(Int<'a>),
    #[regex("0[fF]", float)]
    #[regex("[1-9][0-9]*[fF]", float)]
    #[regex(r#"[0-9]+\.[0-9]*(?&exponent)?[fF]?"#, float)]
    #[regex(r#"[0-9]+(?&exponent)[fF]?"#, float)]
    #[regex(r#"\.[0-9]+(?&exponent)?[fF]?"#, float)]
    FloatLiteral(f64),
    #[regex(r#"'|""#, string)]
    StringLiteral(Vec<u8>),
    #[token(".")]
    Dot,
    #[token("-")]
    Minus,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("<")]
    LeftAngleBracket,
    #[token(">")]
    RightAngleBracket,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token("/")]
    ForwardSlash,
    #[error]
    #[regex(r"[\t\v\f\r\n ]+", skip)]
    #[regex(r"#[^\n]*\n?", skip)]
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Int<'a> {
    pub value: &'a str,
    pub radix: u32,
}

#[derive(Default)]
pub(crate) struct TokenExtras {
    pub error: Option<ParseErrorKind>,
}

impl<'a> fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(value) => write!(f, "{}", value),
            Token::IntLiteral(value) => write!(f, "{}", value.value),
            Token::FloatLiteral(value) => {
                if value.fract() == 0.0 {
                    write!(f, "{:.1}", value)
                } else {
                    write!(f, "{}", value)
                }
            }
            Token::StringLiteral(bytes) => {
                write!(f, "\"")?;
                for &ch in bytes {
                    write!(f, "{}", ascii::escape_default(ch))?;
                }
                write!(f, "\"")?;
                Ok(())
            }
            Token::Dot => write!(f, "."),
            Token::Minus => write!(f, "-"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::LeftBracket => write!(f, "["),
            Token::RightBracket => write!(f, "]"),
            Token::LeftAngleBracket => write!(f, "<"),
            Token::RightAngleBracket => write!(f, ">"),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Semicolon => write!(f, ";"),
            Token::ForwardSlash => write!(f, "/"),
            Token::Error => write!(f, "<ERROR>"),
        }
    }
}

fn int<'a>(lex: &mut Lexer<'a, Token<'a>>, radix: u32, prefix_len: usize) -> Result<Int<'a>, ()> {
    debug_assert!(lex.slice().len() > prefix_len);
    let span = lex.span().start + prefix_len..lex.span().end;

    if matches!(lex.remainder().chars().next(), Some(ch) if ch.is_ascii_alphabetic() || ch == '_') {
        let mut end = span.end + 1;
        while end < lex.source().len() && lex.source().as_bytes()[end].is_ascii_alphabetic() {
            end += 1;
        }
        lex.extras.error = Some(ParseErrorKind::NoSpaceBetweenIntAndIdent {
            span: span.start..end,
        });
        return Err(());
    }

    Ok(Int {
        value: &lex.source()[span],
        radix,
    })
}

fn float<'a>(lex: &mut Lexer<'a, Token<'a>>) -> f64 {
    let start = lex.span().start;
    let last = lex.span().end - 1;
    let s = match lex.source().as_bytes()[last] {
        b'f' | b'F' => &lex.source()[start..last],
        _ => lex.slice(),
    };

    s.parse().expect("failed to parse float")
}

fn string<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Result<Vec<u8>, ()> {
    #[derive(Logos)]
    #[logos(subpattern hex = r"[0-9A-Fa-f]")]
    enum Component<'a> {
        #[regex(r#"[^\x00\n\\'"]+"#)]
        Unescaped(&'a str),
        #[regex(r#"['"]"#, terminator)]
        Terminator(u8),
        #[regex(r#"\\[xX](?&hex)(?&hex)?"#, hex_escape)]
        #[regex(r#"\\[0-7][0-7]?[0-7]?"#, oct_escape)]
        #[regex(r#"\\[abfnrtv?\\'"]"#, char_escape)]
        Byte(u8),
        #[regex(r#"\\u(?&hex)(?&hex)(?&hex)(?&hex)"#, unicode_escape)]
        #[regex(
            r#"\\U(?&hex)(?&hex)(?&hex)(?&hex)(?&hex)(?&hex)(?&hex)(?&hex)"#,
            unicode_escape
        )]
        Char(char),
        #[error]
        Error,
    }

    fn terminator<'a>(lex: &mut Lexer<'a, Component<'a>>) -> u8 {
        debug_assert_eq!(lex.slice().len(), 1);
        lex.slice().bytes().next().unwrap()
    }

    fn hex_escape<'a>(lex: &mut Lexer<'a, Component<'a>>) -> u8 {
        u32::from_str_radix(&lex.slice()[2..], 16)
            .expect("expected valid hex escape")
            .try_into()
            .expect("two-digit hex escape should be valid byte")
    }

    fn oct_escape<'a>(lex: &mut Lexer<'a, Component<'a>>) -> Result<u8, ()> {
        u32::from_str_radix(&lex.slice()[1..], 8)
            .expect("expected valid oct escape")
            .try_into()
            .map_err(drop)
    }

    fn char_escape<'a>(lex: &mut Lexer<'a, Component<'a>>) -> u8 {
        match lex.slice().as_bytes()[1] {
            b'a' => b'\x07',
            b'b' => b'\x08',
            b'f' => b'\x0c',
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'v' => b'\x0b',
            b'?' => b'?',
            b'\\' => b'\\',
            b'\'' => b'\'',
            b'"' => b'"',
            _ => panic!("failed to parse char escape"),
        }
    }

    fn unicode_escape<'a>(lex: &mut Lexer<'a, Component<'a>>) -> Option<char> {
        let value = u32::from_str_radix(&lex.slice()[2..], 16).expect("expected valid hex escape");
        char::from_u32(value)
    }

    let mut result = Vec::new();

    let mut char_lexer = Component::lexer(lex.remainder());
    let terminator = lex.slice().as_bytes()[0];

    loop {
        match char_lexer.next() {
            Some(Component::Unescaped(s)) => result.extend_from_slice(s.as_bytes()),
            Some(Component::Terminator(t)) if t == terminator => {
                break;
            }
            Some(Component::Terminator(ch) | Component::Byte(ch)) => result.push(ch),
            Some(Component::Char(ch)) => {
                let mut buf = [0; 4];
                result.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
            }
            Some(Component::Error) => {
                let start = lex.span().end + char_lexer.span().start;
                let end = lex.span().end + char_lexer.span().end;

                if char_lexer.slice().starts_with('\\') {
                    lex.extras.error =
                        Some(ParseErrorKind::InvalidStringEscape { span: start..end });
                } else {
                    lex.extras.error =
                        Some(ParseErrorKind::InvalidStringCharacters { span: start..end });
                }
                return Err(());
            }
            None => {
                lex.extras.error = Some(ParseErrorKind::UnexpectedEof {
                    expected: "string terminator".to_owned(),
                });
                return Err(());
            }
        }
    }

    lex.bump(char_lexer.span().end);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_tokens() {
        let source = r#"hell0 052 42 0x2A 5. 0.5 0.42e+2 2e-4 .2e+3 52e3 true
            false "hello \a\b\f\n\r\t\v\?\\\'\" \052 \x2a" #comment
            'hello 😀' _foo"#;
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next().unwrap(), Token::Ident("hell0"));
        assert_eq!(
            lexer.next().unwrap(),
            Token::IntLiteral(Int {
                value: "52",
                radix: 8,
            })
        );
        assert_eq!(
            lexer.next().unwrap(),
            Token::IntLiteral(Int {
                value: "42",
                radix: 10,
            })
        );
        assert_eq!(
            lexer.next().unwrap(),
            Token::IntLiteral(Int {
                value: "2A",
                radix: 16,
            })
        );
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(5.));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(0.5));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(0.42e+2));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(2e-4));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(0.2e+3));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(52e3));
        assert_eq!(lexer.next().unwrap(), Token::Ident("true"));
        assert_eq!(lexer.next().unwrap(), Token::Ident("false"));
        assert_eq!(
            lexer.next().unwrap(),
            Token::StringLiteral(b"hello \x07\x08\x0c\n\r\t\x0b?\\'\" * *".as_ref().into())
        );
        assert_eq!(
            lexer.next().unwrap(),
            Token::StringLiteral(b"hello \xF0\x9F\x98\x80".as_ref().into())
        );
        assert_eq!(lexer.next().unwrap(), Token::Ident("_foo"));
        assert_eq!(lexer.next(), None);

        assert_eq!(lexer.extras.error, None);
    }

    #[test]
    fn integer_overflow() {
        let source = "99999999999999999999999999999999999999";
        let mut lexer = Token::lexer(source);

        assert_eq!(
            lexer.next(),
            Some(Token::IntLiteral(Int {
                value: "99999999999999999999999999999999999999",
                radix: 10,
            }))
        );
        assert_eq!(lexer.next(), None);

        assert_eq!(lexer.extras.error, None);
    }

    #[test]
    fn float_suffix() {
        let source = "10f 5.f 0.5f 0.42e+2f 2e-4f .2e+3f";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(10.));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(5.));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(0.5));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(0.42e+2));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(2e-4));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(0.2e+3));
        assert_eq!(lexer.next(), None);
        assert_eq!(lexer.extras.error, None);
    }

    #[test]
    fn invalid_token() {
        let source = "@ foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Error));

        assert_eq!(lexer.extras.error, None);
    }

    #[test]
    fn invalid_string_char() {
        let source = "\"\x00\" foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Error));

        assert_eq!(
            lexer.extras.error,
            Some(ParseErrorKind::InvalidStringCharacters { span: 1..2 })
        );
    }

    #[test]
    fn unterminated_string() {
        let source = "\"hello \n foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Error));

        assert_eq!(
            lexer.extras.error,
            Some(ParseErrorKind::InvalidStringCharacters { span: 7..8 })
        );
    }

    #[test]
    fn invalid_string_escape() {
        let source = r#""\m""#;
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Error));

        assert_eq!(
            lexer.extras.error,
            Some(ParseErrorKind::InvalidStringEscape { span: 1..2 })
        );
    }

    #[test]
    fn string_escape_invalid_utf8() {
        let source = r#""\xFF""#;
        let mut lexer = Token::lexer(source);

        assert_eq!(
            lexer.next(),
            Some(Token::StringLiteral([0xff].as_ref().into()))
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn string_unicode_escape() {
        let source = r#"'\u0068\u0065\u006c\u006c\u006f\u0020\U0001f600'"#;
        let mut lexer = Token::lexer(source);

        assert_eq!(
            lexer.next(),
            Some(Token::StringLiteral(
                b"hello \xF0\x9F\x98\x80".as_ref().into()
            ))
        );
        assert_eq!(lexer.next(), None);

        assert_eq!(lexer.extras.error, None);
    }

    #[test]
    fn string_invalid_unicode_escape() {
        let mut lexer = Token::lexer(r#"'\Uffffffff'"#);
        assert_eq!(lexer.next(), Some(Token::Error));
        assert_eq!(
            lexer.extras.error,
            Some(ParseErrorKind::InvalidStringEscape { span: 1..11 })
        );
    }

    #[test]
    fn whitespace() {
        assert_eq!(
            Token::lexer("value: -2.0").collect::<Vec<_>>(),
            vec![
                Token::Ident("value"),
                Token::Colon,
                Token::Minus,
                Token::FloatLiteral(2.0),
            ]
        );
        assert_eq!(
            Token::lexer("value: - 2.0").collect::<Vec<_>>(),
            vec![
                Token::Ident("value"),
                Token::Colon,
                Token::Minus,
                Token::FloatLiteral(2.0),
            ]
        );
        assert_eq!(
            Token::lexer("value: -\n  #comment\n  2.0").collect::<Vec<_>>(),
            vec![
                Token::Ident("value"),
                Token::Colon,
                Token::Minus,
                Token::FloatLiteral(2.0),
            ]
        );
        assert_eq!(
            Token::lexer("value: 2 . 0").collect::<Vec<_>>(),
            vec![
                Token::Ident("value"),
                Token::Colon,
                Token::IntLiteral(Int {
                    value: "2",
                    radix: 10,
                }),
                Token::Dot,
                Token::IntLiteral(Int {
                    value: "0",
                    radix: 10,
                }),
            ]
        );

        assert_eq!(
            Token::lexer("foo: 10 bar: 20").collect::<Vec<_>>(),
            vec![
                Token::Ident("foo"),
                Token::Colon,
                Token::IntLiteral(Int {
                    value: "10",
                    radix: 10,
                }),
                Token::Ident("bar"),
                Token::Colon,
                Token::IntLiteral(Int {
                    value: "20",
                    radix: 10,
                }),
            ]
        );
        assert_eq!(
            Token::lexer("foo: 10,bar: 20").collect::<Vec<_>>(),
            vec![
                Token::Ident("foo"),
                Token::Colon,
                Token::IntLiteral(Int {
                    value: "10",
                    radix: 10,
                }),
                Token::Comma,
                Token::Ident("bar"),
                Token::Colon,
                Token::IntLiteral(Int {
                    value: "20",
                    radix: 10,
                }),
            ]
        );
        assert_eq!(
            Token::lexer("foo: 10[com.foo.ext]: 20").collect::<Vec<_>>(),
            vec![
                Token::Ident("foo"),
                Token::Colon,
                Token::IntLiteral(Int {
                    value: "10",
                    radix: 10,
                }),
                Token::LeftBracket,
                Token::Ident("com"),
                Token::Dot,
                Token::Ident("foo"),
                Token::Dot,
                Token::Ident("ext"),
                Token::RightBracket,
                Token::Colon,
                Token::IntLiteral(Int {
                    value: "20",
                    radix: 10,
                }),
            ]
        );

        let mut lexer = Token::lexer("foo: 10bar: 20");
        assert_eq!(lexer.next(), Some(Token::Ident("foo")));
        assert_eq!(lexer.next(), Some(Token::Colon));
        assert_eq!(lexer.next(), Some(Token::Error));
        assert_eq!(
            lexer.extras.error,
            Some(ParseErrorKind::NoSpaceBetweenIntAndIdent { span: 5..10 })
        );

        let mut lexer = Token::lexer("bar: 20_foo");
        assert_eq!(lexer.next(), Some(Token::Ident("bar")));
        assert_eq!(lexer.next(), Some(Token::Colon));
        assert_eq!(lexer.next(), Some(Token::Error));
        assert_eq!(
            lexer.extras.error,
            Some(ParseErrorKind::NoSpaceBetweenIntAndIdent { span: 5..11 })
        );
    }
}
