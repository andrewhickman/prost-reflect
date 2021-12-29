use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;

#[proc_macro_derive(ReflectMessage, attributes(prost_reflect))]
pub fn reflect_message(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match reflect_message_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

struct Args {
    message_name: syn::LitStr,
    file_descriptor_path: syn::LitStr,
}

fn reflect_message_impl(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        syn::Data::Struct(_) => (),
        syn::Data::Enum(enum_data) => {
            return Err(syn::Error::new(
                enum_data.enum_token.span,
                "cannot derive ReflectMessage for enum",
            ))
        }
        syn::Data::Union(union_data) => {
            return Err(syn::Error::new(
                union_data.union_token.span,
                "cannot derive ReflectMessage for union",
            ))
        }
    };

    let name = &input.ident;
    let Args {
        file_descriptor_path,
        message_name,
    } = parse_attrs(input.ident.span(), &input.attrs)?;

    Ok(quote! {
        impl ::prost_reflect::ReflectMessage for #name {
            fn descriptor(&self) -> ::prost_reflect::MessageDescriptor {
                ::prost_reflect::FileDescriptor::new_cached(include_bytes!(#file_descriptor_path))
                    .expect("invalid file descriptor set")
                    .get_message_by_name(#message_name)
                    .expect("no message found")
            }
        }
    })
}

fn parse_attrs(
    input_span: proc_macro2::Span,
    attrs: &[syn::Attribute],
) -> Result<Args, syn::Error> {
    let reflect_attrs: Vec<_> = attrs
        .iter()
        .filter(|attr| is_prost_reflect_attribute(attr))
        .collect();

    match reflect_attrs.len() {
        0 => {
            return Err(syn::Error::new(
                input_span,
                "missing #[prost_reflect] attribute",
            ))
        }
        1 => (),
        _ => {
            return Err(syn::Error::new(
                reflect_attrs[1].span(),
                "multiple #[prost_reflect] attributes",
            ))
        }
    };

    reflect_attrs[0].parse_args::<Args>()
}

fn is_prost_reflect_attribute(attr: &syn::Attribute) -> bool {
    attr.path.is_ident("prost_reflect")
}

impl Parse for Args {
    fn parse(parser: ParseStream) -> syn::Result<Self> {
        let mut file_descriptor_path: Option<syn::LitStr> = None;
        let mut message_name: Option<syn::LitStr> = None;
        loop {
            let ident = parser.parse::<syn::Path>()?;
            if ident.is_ident("file_descriptor_path") {
                let _ = parser.parse::<syn::Token!(=)>()?;
                file_descriptor_path = Some(parser.parse()?);
            } else if ident.is_ident("message_name") {
                let _ = parser.parse::<syn::Token!(=)>()?;
                message_name = Some(parser.parse()?);
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    format_args!(
                        "unknown argument (expected 'file_descriptor_path' or 'message_name')"
                    ),
                ));
            }

            if let (Some(file_descriptor_path), Some(message_name)) =
                (&file_descriptor_path, &message_name)
            {
                return Ok(Args {
                    file_descriptor_path: file_descriptor_path.clone(),
                    message_name: message_name.clone(),
                });
            } else {
                let _ = parser.parse::<syn::Token!(,)>()?;
            }
        }
    }
}
