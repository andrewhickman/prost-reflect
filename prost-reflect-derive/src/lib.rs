use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
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
    args_span: Span,
    message_name: Option<syn::Lit>,
    package_name: Option<syn::Lit>,
    file_descriptor: Option<syn::Lit>,
}

fn reflect_message_impl(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        syn::Data::Struct(_) => (),
        syn::Data::Enum(_) => return Ok(Default::default()),
        syn::Data::Union(_) => return Ok(Default::default()),
    };

    let args = Args::parse(input.ident.span(), &input.attrs)?;

    let name = &input.ident;
    let file_descriptor_set = args.file_descriptor()?;
    let message_name = args.message_name(name)?;

    Ok(quote! {
        impl ::prost_reflect::ReflectMessage for #name {
            fn descriptor(&self) -> ::prost_reflect::MessageDescriptor {
                #file_descriptor_set
                    .get_message_by_name(#message_name)
                    .expect("no message found")
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

        let meta = match reflect_attrs[0].parse_meta()? {
            syn::Meta::List(list) => list,
            meta => return Err(syn::Error::new(meta.span(), "expected list of attributes")),
        };

        let mut args = Args {
            args_span: meta.span(),
            file_descriptor: None,
            package_name: None,
            message_name: None,
        };
        for item in meta.nested {
            match item {
                syn::NestedMeta::Meta(syn::Meta::NameValue(value)) => {
                    if value.path.is_ident("file_descriptor") {
                        args.file_descriptor = Some(value.lit);
                    } else if value.path.is_ident("package_name") {
                        args.package_name = Some(value.lit);
                    } else if value.path.is_ident("message_name") {
                        args.message_name = Some(value.lit);
                    } else {
                        return Err(syn::Error::new(value.span(),
                        "unknown argument (expected 'file_descriptor', 'package_name' or 'message_name')"));
                    }
                }
                _ => return Err(syn::Error::new(item.span(), "unexpected attribute")),
            }
        }

        Ok(args)
    }

    fn file_descriptor(&self) -> Result<proc_macro2::TokenStream, syn::Error> {
        if let Some(file_descriptor_set) = &self.file_descriptor {
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
                "missing 'file_descriptor' argument",
            ))
        }
    }

    fn message_name(
        &self,
        struct_name: &syn::Ident,
    ) -> Result<proc_macro2::TokenStream, syn::Error> {
        if let Some(message_name) = &self.message_name {
            Ok(message_name.to_token_stream())
        } else if let Some(package_name) = &self.package_name {
            match package_name {
                syn::Lit::Str(package_name) => Ok(syn::LitStr::new(
                    &format!("{}.{}", package_name.value(), struct_name),
                    self.args_span,
                )
                .to_token_stream()),
                _ => Err(syn::Error::new(
                    self.args_span,
                    "'package_name' must be a string literal",
                )),
            }
        } else {
            Err(syn::Error::new(
                self.args_span,
                "at least one of the 'message_name' or 'package_name' arguments is required",
            ))
        }
    }
}
