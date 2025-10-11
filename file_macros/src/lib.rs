use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, DeriveInput, Expr, Ident, Meta, Type};

use syn::{Token, punctuated::Punctuated};

/*
 * File Reader Proc Macro Idea
 * #[derive(FileRW)]
 * #[versions]
 * struct Foo {
 *     #[magic("TDB\0")]
 *     magic: u32,
 *     #[version]
 *     version: u32,
 *     a: u32,
 *     b: u64,
 *     #[predicates()]
 *     c: Option<u32>,
 *     d_offset: u64
 *     d_len: u32,
 *     #[vecvar(start=d_offset, len=d_len, method)] // method here is how its gonna read the vec, string offsets, inline structs, etc
 *     d: Vec<u32>
 * }
 */

// Read Depends On
struct DependsOnEntry {
    name: Ident,
    _colon_token: Token![:],
    ty: Type,
}

impl Parse for DependsOnEntry {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        Ok(DependsOnEntry {
            name: input.parse()?,
            _colon_token: input.parse()?,
            ty: input.parse()?,
        })
    }
}

struct DependsOnAttr {
    entries: Punctuated<DependsOnEntry, Token![,]>,
}

impl Parse for DependsOnAttr {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        Ok(DependsOnAttr {
            entries: Punctuated::parse_terminated(input)?,
        })
    }
}

// Read Context Args
struct ContextEntry {
    name: Ident,
    _colon_token: Token![:],
    value: Expr,
}

impl Parse for ContextEntry {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            _colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

struct ContextAttr {
    ty: Ident,
    args: Punctuated<ContextEntry, Token![,]>,
}

impl Parse for ContextAttr {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        // Parse the first argument as a type
        let ty: Ident = input.parse()?;

        let mut args = Punctuated::new();

        // If there's a comma, parse additional args
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            args = Punctuated::parse_terminated(input)?;
        }

        Ok(ContextAttr { ty, args })
    }
}


#[proc_macro_derive(StructRW, attributes(magic, varlist, var, depends_on, context, if_predicate))]
pub fn derive_struct_rw(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let fields = match input.data {
        syn::Data::Struct(ref data) => &data.fields,
        _ => unimplemented!("StructRW only works on structs")
    };

    let mut context = Vec::new();
    for attr in &input.attrs {
        if attr.path().is_ident("depends_on") {
            let depends_on_attr: DependsOnAttr = attr.parse_args().unwrap();
            for entry in depends_on_attr.entries {
                let name = entry.name;
                let ty = entry.ty;
                context.push(quote! { #name: #ty });
            }
        }
    }

    let context_name = format_ident!("{}Context", name);
    let context = if context.is_empty() {
        None
    } else {
        Some(quote!{
            pub struct #context_name<'a> {
                #(#context),*
            }
        })
    };





    let result = match fields {
        syn::Fields::Named(fields) => {
            let header_fields_set = fields.named.iter().map(|f| {
                let ident = f.ident.as_ref().unwrap();
                let ty = &f.ty;
                let mut context_args = vec![];
                let mut context_args_name = None;
                let mut val_res = quote!{ {<#ty>::read(reader, ctx)? } };
                let mut condition = None;
                for attr in &f.attrs {
                    if attr.path().is_ident("condition") {
                        let expr: Expr = attr.parse_args().unwrap();
                        condition = Some(expr);
                    }
                    if attr.path().is_ident("var") {
                        let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated).unwrap();
                        let mut info = vec![];
                        for meta in nested {
                            match meta {
                                Meta::Path(path) => {
                                    info.push(path);
                                }
                                _ => unimplemented!("ahhh")
                            }
                        }
                        if let Some(ty) = info[0].get_ident() {
                            if let Some(offset) = info[1].get_ident() {
                                val_res = quote! {
                                    {
                                        reader.seek(std::io::SeekFrom::Start(#offset.into()))?;
                                        //reader.seek_assert_align_up(#offset, 16)?;
                                        let v = <#ty>::read(reader, ctx)?;
                                        v
                                    }
                                }

                            }
                        }
                    }
                    if attr.path().is_ident("varlist") {
                        let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated).unwrap();

                        let mut ty: Option<Expr> = None;
                        let mut offset: Option<Expr> = None;
                        let mut offsets: Option<Expr> = None; // when you have an offset var
                        let mut count: Option<Expr> = None;
                        for meta in nested {
                            match meta {
                                Meta::NameValue(nv) => {
                                    if nv.path.is_ident("offset") {
                                        offset = Some(nv.value);
                                    }
                                    else if nv.path.is_ident("ty") {
                                        ty = Some(nv.value);
                                    }
                                    else if nv.path.is_ident("offsets") {
                                        offsets = Some(nv.value);
                                    }
                                    else if nv.path.is_ident("count") {
                                        count = Some(nv.value);
                                    }
                                }
                                _ => unimplemented!("ahhh")
                            }
                        }
                        let ty = ty.expect("varlist requires a type");
                        if let Some(count) = count {
                            if let Some(offsets) = offsets {
                                val_res = quote! {
                                    {
                                        let pos = reader.tell()?;
                                        let v = #offsets.iter().map(|x| {
                                            reader.seek(std::io::SeekFrom::Start((*x).into()))?;
                                            <#ty>::read(reader, ctx)
                                        }).collect::<std::result::Result<Vec<_>, Box<dyn std::error::Error>>>()?;
                                        reader.seek(std::io::SeekFrom::Start(pos.into()))?;
                                        v
                                    }
                                }
                            } else {
                                match offset {
                                    Some(offset) =>  {
                                        val_res = quote! {
                                            {
                                                let pos = reader.tell()?;
                                                reader.seek(std::io::SeekFrom::Start(#offset.into()))?;
                                                let v = (0..#count).map(|x| {
                                                    <#ty>::read(reader, ctx)
                                                }).collect::<std::result::Result<Vec<_>, Box<dyn std::error::Error>>>()?;
                                                reader.seek(std::io::SeekFrom::Start(pos.into()))?;
                                                v
                                            }
                                        }
                                    }
                                    None => {
                                        val_res = quote! {
                                            {
                                                let v = (0..#count).map(|x| {
                                                    <#ty>::read(reader, ctx)
                                                }).collect::<std::result::Result<Vec<_>, Box<dyn std::error::Error>>>()?;
                                                v
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            if let Some(offset) = offset {
                                val_res = quote! {
                                    {
                                        let pos = reader.tell()?;
                                        let end = reader.seek(std::io::SeekFrom::End(0))?;
                                        reader.seek(std::io::SeekFrom::Start(#offset.into()))?;
                                        let remaining = end - #offset;
                                        let v = (0..remaining).map(|x| {
                                            <#ty>::read(reader, ctx)
                                        }).collect::<std::result::Result<Vec<_>, Box<dyn std::error::Error>>>()?;
                                        reader.seek(std::io::SeekFrom::Start(pos.into()))?;
                                        v
                                    }
                                }

                            }
                        }
                    }

                    if attr.path().is_ident("magic") {
                        match &attr.meta {
                            Meta::NameValue(nv) => {
                                let val = &nv.value;
                                val_res = quote! {
                                    {
                                        let magic = <#ty>::read(reader, ctx)?;
                                        if &magic != #val {
                                            return Err("magic not equal".into())
                                        }
                                        magic
                                    }
                                };
                            },
                            _ => unimplemented!("unimplemented attributes for magic")
                        }
                    }
                    if attr.path().is_ident("context") {
                        //println!("{attr:?}");
                        let context_attr: ContextAttr = attr.parse_args().unwrap();
                        let ty = context_attr.ty;   // MsgEntryV2
                        let args = context_attr.args; // [&data, lang_count]
                        for arg in args {
                            let name = arg.name;
                            let value = arg.value;
                            context_args.push(quote! {#name: #value});
                        }
                        // Now you can generate code:
                        // e.g. Context::new(args...)
                        let name = format_ident!("{}Context", ty);
                        context_args_name = Some(name);
                    }
                }
                let ctx = if let Some(context_args_name) = context_args_name {
                    let ctx = quote!{
                        // wtf lol
                        let mut ctx2 = #context_args_name { #(#context_args),* };
                        let ctx = &mut ctx2;
                    };
                    ctx
                } else {
                    quote! {}
                };
                //println!("CONDITION {}", condition.unwrap());
                if let Some(condition) = condition {
                    quote! {
                        #ctx
                        let #ident = if #condition {
                            #val_res
                        } else {
                            None
                        };
                    }
                } else {
                    quote!{
                        #ctx
                        let #ident = #val_res;
                    }
                }
            });

            let header_fields = fields.named.iter().map(|f| {
                let ident = f.ident.as_ref();
                quote!{#ident}
            });

            if context.is_some() {
                quote! {
                    #context

                    impl<'a> StructRW<#context_name<'a>> for #name {
                        fn read<R: std::io::Read + std::io::Seek>(reader: &mut R, ctx: &mut #context_name) -> std::result::Result<Self, Box<dyn std::error::Error>> {
                            #(#header_fields_set)*
                            Ok(#name {
                                #(#header_fields),*
                            })
                        }
                    }
                }
            } else {
                quote! {
                    impl<C> StructRW<C> for #name {
                        fn read<R: std::io::Read + std::io::Seek>(reader: &mut R, ctx: &mut C) -> std::result::Result<Self, Box<dyn std::error::Error>> {
                            #(#header_fields_set)*
                            Ok(#name {
                                #(#header_fields),*
                            })
                        }
                    }
                }
            }

        },
        _ => unimplemented!("StructRW only works on named fields")
    };
    //println!("{}", result);
    result.into()
}

