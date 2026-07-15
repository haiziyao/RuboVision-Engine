use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, ExprLit, Ident, Item, Lit, Meta, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

struct AttributeArgs {
    args: Punctuated<Meta, Token![,]>,
}

impl Parse for AttributeArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            args: Punctuated::parse_terminated(input)?,
        })
    }
}

fn parse_args(attr: TokenStream) -> Result<AttributeArgs, TokenStream> {
    syn::parse::<AttributeArgs>(attr).map_err(|error| error.to_compile_error().into())
}

fn string_arg(args: &AttributeArgs, name: &str) -> Result<String, TokenStream> {
    let mut result = None;
    for meta in &args.args {
        let Meta::NameValue(name_value) = meta else {
            return Err(
                syn::Error::new_spanned(meta, format!("expected `{name} = \"...\"`"))
                    .to_compile_error()
                    .into(),
            );
        };
        if !name_value.path.is_ident(name) {
            return Err(syn::Error::new_spanned(
                &name_value.path,
                format!("unknown argument; expected `{name}`"),
            )
            .to_compile_error()
            .into());
        }
        if result.is_some() {
            return Err(syn::Error::new_spanned(
                name_value,
                format!("duplicate `{name}` argument"),
            )
            .to_compile_error()
            .into());
        }
        let Expr::Lit(ExprLit {
            lit: Lit::Str(value),
            ..
        }) = &name_value.value
        else {
            return Err(syn::Error::new_spanned(
                &name_value.value,
                format!("`{name}` must be a string literal"),
            )
            .to_compile_error()
            .into());
        };
        result = Some(value.value());
    }

    result.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("missing `{name}` argument"),
        )
        .to_compile_error()
        .into()
    })
}

fn item_type_ident(item: &Item) -> Option<&Ident> {
    match item {
        Item::Struct(item) => Some(&item.ident),
        Item::Enum(item) => Some(&item.ident),
        Item::Union(item) => Some(&item.ident),
        _ => None,
    }
}

fn register_source(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match parse_args(attr) {
        Ok(args) => args,
        Err(error) => return error,
    };
    let kind = match string_arg(&args, "kind") {
        Ok(kind) => kind,
        Err(error) => return error,
    };
    let item = match syn::parse::<Item>(item) {
        Ok(item) => item,
        Err(error) => return error.to_compile_error().into(),
    };
    let Some(ident) = item_type_ident(&item) else {
        return syn::Error::new_spanned(
            item,
            "`source` can only be used on a struct, enum, or union",
        )
        .to_compile_error()
        .into();
    };

    quote! {
        #item

        ::rubo_engine::inventory::submit! {
            ::rubo_engine::SourceInventoryRegistration {
                kind: #kind,
                register: ::rubo_engine::register_source_factory::<#ident>,
            }
        }
    }
    .into()
}

fn register_device(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match parse_args(attr) {
        Ok(args) => args,
        Err(error) => return error,
    };
    let kind = match string_arg(&args, "kind") {
        Ok(kind) => kind,
        Err(error) => return error,
    };
    let item = match syn::parse::<Item>(item) {
        Ok(item) => item,
        Err(error) => return error.to_compile_error().into(),
    };
    let Some(ident) = item_type_ident(&item) else {
        return syn::Error::new_spanned(
            item,
            "`device` can only be used on a struct, enum, or union",
        )
        .to_compile_error()
        .into();
    };

    quote! {
        #item

        ::rubo_engine::inventory::submit! {
            ::rubo_engine::DeviceInventoryRegistration {
                kind: #kind,
                register: ::rubo_engine::register_device_type::<#ident>,
            }
        }
    }
    .into()
}

fn register_function(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match parse_args(attr) {
        Ok(args) => args,
        Err(error) => return error,
    };
    let id = match string_arg(&args, "id") {
        Ok(id) => id,
        Err(error) => return error,
    };
    let item = match syn::parse::<Item>(item) {
        Ok(item) => item,
        Err(error) => return error.to_compile_error().into(),
    };
    let Some(ident) = item_type_ident(&item) else {
        return syn::Error::new_spanned(
            item,
            "`function` can only be used on a struct, enum, or union",
        )
        .to_compile_error()
        .into();
    };

    quote! {
        #item

        ::rubo_engine::inventory::submit! {
            ::rubo_engine::FunctionInventoryRegistration {
                id: #id,
                register: ::rubo_engine::register_function_type::<#ident>,
            }
        }
    }
    .into()
}

fn register_sink(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match parse_args(attr) {
        Ok(args) => args,
        Err(error) => return error,
    };
    let id = match string_arg(&args, "id") {
        Ok(id) => id,
        Err(error) => return error,
    };
    let item = match syn::parse::<Item>(item) {
        Ok(item) => item,
        Err(error) => return error.to_compile_error().into(),
    };
    let Some(ident) = item_type_ident(&item) else {
        return syn::Error::new_spanned(
            item,
            "`sink` can only be used on a struct, enum, or union",
        )
        .to_compile_error()
        .into();
    };

    quote! {
        #item

        ::rubo_engine::inventory::submit! {
            ::rubo_engine::SinkInventoryRegistration {
                id: #id,
                register: ::rubo_engine::register_sink_type::<#ident>,
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn source(attr: TokenStream, item: TokenStream) -> TokenStream {
    register_source(attr, item)
}

#[proc_macro_attribute]
pub fn device(attr: TokenStream, item: TokenStream) -> TokenStream {
    register_device(attr, item)
}

#[proc_macro_attribute]
pub fn function(attr: TokenStream, item: TokenStream) -> TokenStream {
    register_function(attr, item)
}

#[proc_macro_attribute]
pub fn sink(attr: TokenStream, item: TokenStream) -> TokenStream {
    register_sink(attr, item)
}
