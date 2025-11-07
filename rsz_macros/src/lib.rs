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

#[proc_macro_derive(DeRszInstance)]
pub fn derive_instance(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let _meta = match RszMeta::from_attrs(&input.attrs) {
        Ok(meta) => meta,
        Err(err) => return err.to_compile_error().into(),
    };

    let result = match &input.data {
        syn::Data::Struct(data) => {
            let fields_to: Vec<proc_macro2::TokenStream> = match &data.fields {
                syn::Fields::Named(fields_named) => {
                    // Handle named fields (e.g., struct S { a: T, b: U })
                    let field_to: Vec<proc_macro2::TokenStream> = fields_named.named.iter().enumerate().map(|(_i, f)| {
                        let ident = f.ident.as_ref().unwrap();
                        let _ty = &f.ty;
                        quote! { self.#ident.to_bytes(ctx)?; }
                    }).collect();
                    field_to
                },
                syn::Fields::Unnamed(fields_unnamed) => {
                    // Handle tuple structs (e.g., struct S(T, U))
                    let field_to: Vec<proc_macro2::TokenStream> = fields_unnamed.unnamed.iter().enumerate().map(|(i, f)| {
                        let _ty = &f.ty;
                        let i: syn::Index = i.into();
                        quote! { self.#i.to_bytes(ctx)?; }
                    }).collect::<Vec<_>>();
                    field_to
                },
                syn::Fields::Unit => {
                    let field_to: Vec<proc_macro2::TokenStream> = (0..1).map(|_i| {
                        quote!{ }.into()
                    }).collect();
                    field_to
                }
            };
            let res = quote! {
                impl DeRszInstance for #name {
                    fn as_any(&self) -> &dyn Any {
                        self
                    }
                    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
                        serde_json::json!(self)
                    }
                    fn to_bytes(&self, ctx: &mut RszSerializerCtx) -> Result<()> {
                        #(#fields_to)*
                        Ok(())
                    }
                }
            };
            res
        }
        _ => {
            syn::Error::new_spanned(&input, "DeRszType can only be derived for structs")
                .to_compile_error()
        }
    };
    result.into()
}


#[proc_macro_derive(Edit)]
pub fn derive_edit(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let _meta = match RszMeta::from_attrs(&input.attrs) {
        Ok(meta) => meta,
        Err(err) => return err.to_compile_error().into(),
    };

    let result = match &input.data {
        syn::Data::Struct(data) => {
            let fields_edit: Vec<proc_macro2::TokenStream> = match &data.fields {
                syn::Fields::Named(fields_named) => {
                    // Handle named fields (e.g., struct S { a: T, b: U })
                    let field_to: Vec<proc_macro2::TokenStream> = fields_named.named.iter().enumerate().map(|(_i, f)| {
                        let ident = f.ident.as_ref().unwrap();
                        let _ty = &f.ty;
                        quote! { self.#ident.edit(ui, ctx); }
                    }).collect();
                    field_to
                },
                syn::Fields::Unnamed(fields_unnamed) => {
                    // Handle tuple structs (e.g., struct S(T, U))
                    let field_to: Vec<proc_macro2::TokenStream> = fields_unnamed.unnamed.iter().enumerate().map(|(i, f)| {
                        let _ty = &f.ty;
                        let i: syn::Index = i.into();
                        quote! { self.#i.edit(ui, ctx); }
                    }).collect::<Vec<_>>();
                    field_to
                },
                syn::Fields::Unit => {
                    let field_to: Vec<proc_macro2::TokenStream> = (0..1).map(|_i| {
                        quote!{ }.into()
                    }).collect();
                    field_to
                }
            };
            let res = quote! {
                impl<'a> Edit for #name {
                    fn edit(&mut self, ui: &mut Ui, ctx: &mut RszEditCtx) {
                        #(#fields_edit)*
                    }
                }
            };
            res
        }
        _ => {
            syn::Error::new_spanned(&input, "DeRszType can only be derived for structs")
                .to_compile_error()
        }
    };
    result.into()
}


#[proc_macro_derive(DeRszFrom, attributes(rsz))]
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
                    let field_from = fields_named.named.iter().map(|f| {
                        let ident = f.ident.as_ref().unwrap();
                        let ty = &f.ty;
                        quote! { #ident: <#ty>::from_bytes(ctx)? }
                    });
                    let field_from_json = fields_named.named.iter().map(|f| {
                        let ident = f.ident.as_ref().unwrap();
                        let ty = &f.ty;
                        quote! { #ident: <#ty>::from_json(data, ctx)? }
                    });

                    quote! {
                        impl<'a> DeRszType<'a> for #name {
                            fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<#name> {
                                let res = Ok(Self {
                                    #(#field_from),*
                                });
                                #align_code;
                                res
                            }
                        }
                        impl RszFromJson for #name {
                            fn from_json(data: &serde_json::Value, ctx: &mut RszJsonDeserializerCtx) -> Result<#name> {
                                let res = Ok(Self {
                                    #(#field_from_json),*
                                });
                                res
                            }
                        }
                    }
                }
                syn::Fields::Unnamed(fields_unnamed) => {
                    // Handle tuple structs (e.g., struct S(T, U))
                    let field_from = fields_unnamed.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote! { <#ty>::from_bytes(ctx)? }
                    });
                    let field_from_json = fields_unnamed.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote! { <#ty>::from_json(data, ctx)? }
                    });

                    quote! {
                        impl<'a> DeRszType<'a> for #name {
                            fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<#name> {
                                let res = Ok(Self(
                                        #(#field_from),*
                                ));
                                #align_code;
                                res
                            }
                        }
                        impl RszFromJson for #name {
                            fn from_json(data: &serde_json::Value, ctx: &mut RszJsonDeserializerCtx) -> Result<#name> {
                                let res = Ok(Self( #(#field_from_json),*));
                                res
                            }
                        }
                    }
                }
                syn::Fields::Unit => {
                    // Handle unit structs (e.g., struct S;)
                    quote! {
                        impl<'a> DeRszType<'a> for #name {
                            fn from_bytes(_ctx: &'a mut RszDeserializerCtx) -> Result<#name> {
                                Ok(Self)
                            }
                        }
                        impl RszFromJson for #name {
                            fn from_json(data: &serde_json::Value, ctx: &mut RszJsonDeserializerCtx) -> Result<#name> {
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

