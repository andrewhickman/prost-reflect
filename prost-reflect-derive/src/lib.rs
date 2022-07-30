//! This crate provides the [`ReflectMessage`](https://docs.rs/prost-reflect/latest/prost_reflect/derive.ReflectMessage.html) derive macro
//!
//! For documentation, see the example in the [`prost-reflect` crate docs](https://docs.rs/prost-reflect/latest/prost_reflect/index.html#deriving-reflectmessage).
#![doc(html_root_url = "https://docs.rs/prost-reflect-derive/0.9.0/")]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::spanned::Spanned;

/// A derive macro for the [`ReflectMessage`](https://docs.rs/prost-reflect/latest/prost_reflect/trait.ReflectMessage.html) trait.
///
/// For documentation, see the example in the [`prost-reflect` crate docs](https://docs.rs/prost-reflect/latest/prost_reflect/index.html#deriving-reflectmessage).
#[proc_macro_derive(ReflectMessage, attributes(prost_reflect))]
pub fn reflect_message(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match reflect_message_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

struct Args {
    args_span: Span,
    message_name: Option<syn::Lit>,
    descriptor_pool: Option<syn::Lit>,
}

fn reflect_message_impl(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        syn::Data::Struct(_) => (),
        syn::Data::Enum(_) => return Ok(Default::default()),
        syn::Data::Union(_) => return Ok(Default::default()),
    };

    let args = Args::parse(input.ident.span(), &input.attrs)?;

    let name = &input.ident;
    let descriptor_pool = args.descriptor_pool()?;
    let message_name = args.message_name()?;

    Ok(quote! {
        impl ::prost_reflect::ReflectMessage for #name {
            fn descriptor(&self) -> ::prost_reflect::MessageDescriptor {
                #descriptor_pool
                    .get_message_by_name(#message_name)
                    .expect(concat!("descriptor for message type `", #message_name, "` not found"))
            }
        }
    })
}

fn is_prost_reflect_attribute(attr: &syn::Attribute) -> bool {
    attr.path.is_ident("prost_reflect")
}

impl Args {
    fn parse(input_span: proc_macro2::Span, attrs: &[syn::Attribute]) -> Result<Args, syn::Error> {
        let reflect_attrs: Vec<_> = attrs
            .iter()
            .filter(|attr| is_prost_reflect_attribute(attr))
            .collect();

        if reflect_attrs.is_empty() {
            return Err(syn::Error::new(
                input_span,
                "missing #[prost_reflect] attribute",
            ));
        }

        let mut span: Option<Span> = None;
        let mut nested = Vec::new();
        for attr in reflect_attrs {
            span = match span {
                Some(span) => span.join(attr.span()),
                None => Some(attr.span()),
            };
            match attr.parse_meta()? {
                syn::Meta::List(list) => nested.extend(list.nested),
                meta => return Err(syn::Error::new(meta.span(), "expected list of attributes")),
            }
        }

        let mut args = Args {
            args_span: span.unwrap_or_else(Span::call_site),
            descriptor_pool: None,
            message_name: None,
        };
        for item in nested {
            match item {
                syn::NestedMeta::Meta(syn::Meta::NameValue(value)) => {
                    if value.path.is_ident("descriptor_pool") {
                        args.descriptor_pool = Some(value.lit);
                    } else if value.path.is_ident("message_name") {
                        args.message_name = Some(value.lit);
                    } else {
                        return Err(syn::Error::new(
                            value.span(),
                            "unknown argument (expected 'descriptor_pool' or 'message_name')",
                        ));
                    }
                }
                _ => return Err(syn::Error::new(item.span(), "unexpected attribute")),
            }
        }

        Ok(args)
    }

    fn descriptor_pool(&self) -> Result<proc_macro2::TokenStream, syn::Error> {
        if let Some(file_descriptor_set) = &self.descriptor_pool {
            match file_descriptor_set {
                syn::Lit::Str(expr_str) => {
                    let expr: syn::Expr = syn::parse_str(&expr_str.value())?;
                    Ok(expr.to_token_stream())
                }
                _ => Err(syn::Error::new(
                    self.args_span,
                    "'file_descriptor_set' must be a string literal",
                )),
            }
        } else {
            Err(syn::Error::new(
                self.args_span,
                "missing required argument 'descriptor_pool'",
            ))
        }
    }

    fn message_name(&self) -> Result<proc_macro2::TokenStream, syn::Error> {
        if let Some(message_name) = &self.message_name {
            Ok(message_name.to_token_stream())
        } else {
            Err(syn::Error::new(
                self.args_span,
                "missing required argument 'message_name'",
            ))
        }
    }
}
