use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

use syn::{Attribute, Token, punctuated::Punctuated, parse::Parser};

// Helper struct to parse rsz(align = N)
struct RszMeta {
    align: Option<syn::Expr>,
}

impl RszMeta {
    fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut align = None;

        for attr in attrs {
            if !attr.path().is_ident("rsz") {
                continue;
            }

            let parser = Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated;
            let args = parser.parse2(attr.meta.require_list()?.tokens.clone())?;

            for meta in args {
                if meta.path.is_ident("align") {
                    align = Some(meta.value);
                }
            }
        }

        Ok(RszMeta { align })
    }
}


#[proc_macro_derive(DeRszType, attributes(rsz))]
pub fn derive_from_bytes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let meta = match RszMeta::from_attrs(&input.attrs) {
        Ok(meta) => meta,
        Err(err) => return err.to_compile_error().into(),
    };

    let align_code = if let Some(expr) = meta.align {
        quote! { ctx.data.seek_align_up(#expr)?; }
    } else {
        quote! {}
    };

    let result = match &input.data {
        syn::Data::Struct(data) => {
            match &data.fields {
                syn::Fields::Named(fields_named) => {
                    // Handle named fields (e.g., struct S { a: T, b: U })
                    let field_reads = fields_named.named.iter().map(|f| {
                        let ident = f.ident.as_ref().unwrap();
                        let ty = &f.ty;
                        quote! { #ident: <#ty>::from_bytes(ctx)? }
                    });

                    quote! {
                        impl DeRszInstance for #name {
                            fn as_any(&self) -> &dyn Any {
                                self
                            }
                            fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
                                serde_json::json!(self)
                            }
                        }
                        impl<'a> DeRszType<'a> for #name {
                            fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<#name> {
                                let res = Ok(Self {
                                    #(#field_reads),*
                                });
                                #align_code;
                                res
                            }
                        }
                    }
                }
                syn::Fields::Unnamed(fields_unnamed) => {
                    // Handle tuple structs (e.g., struct S(T, U))
                    let field_reads = fields_unnamed.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote! { <#ty>::from_bytes(ctx)? }
                    });

                    quote! {
                        impl DeRszInstance for #name {
                            fn as_any(&self) -> &dyn Any {
                                self
                            }
                            fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
                                serde_json::json!(self)
                            }
                        }
                        #[allow(unused)]
                        impl<'a> DeRszType<'a> for #name {
                            fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<#name> {
                                let res = Ok(Self(
                                        #(#field_reads),*
                                ));
                                #align_code;
                                res
                            }
                        }
                    }
                }
                syn::Fields::Unit => {
                    // Handle unit structs (e.g., struct S;)
                    quote! {
                        impl DeRszInstance for #name {
                            fn as_any(&self) -> &dyn Any {
                                self
                            }
                        }
                        impl<'a> DeRszType<'a> for #name {
                            fn from_bytes(_ctx: &'a mut RszDeserializerCtx) -> Result<#name> {
                                Ok(Self)
                            }
                        }
                    }
                }
            }
        }
        _ => {
            syn::Error::new_spanned(&input, "DeRszType can only be derived for structs")
                .to_compile_error()
        }
    };

    result.into()
}

