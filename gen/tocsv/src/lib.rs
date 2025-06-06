extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn;


#[proc_macro_derive(ToCsv)]
pub fn tocsv_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_hello_macro(&ast)
}

fn impl_hello_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident; // Get the name of the struct

    let r#gen = quote! {
        impl ToCsv for #name {
            type DataType = #name;
            fn to_csv(data: Self::DataType) -> Vec<u8> {
                println!("Hello, Macro! My name is {}", stringify!(#name));
                Vec::new()
            }
        }
    };
    r#gen.into()
}
