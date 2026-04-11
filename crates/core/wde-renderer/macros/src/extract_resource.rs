use proc_macro::{TokenStream};
use quote::quote;
use syn::{DeriveInput, parse_macro_input, parse_quote};

pub fn derive_extract_resource(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let wde_renderer_path: syn::Path = crate::wde_renderer_path();

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Clone });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #wde_renderer_path::sync::ExtractResource for #struct_name #type_generics #where_clause {
            type Source = Self;

            fn extract(source: &Self::Source) -> Self {
                source.clone()
            }
        }
    })
}
