use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input, parse_quote};

pub fn derive_extract_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let wde_renderer_path: syn::Path = crate::wde_renderer_path();

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Clone });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let filter = if let Some(attr) = ast
        .attrs
        .iter()
        .find(|a| a.path().is_ident("extract_component_filter"))
    {
        let filter = match attr.parse_args::<syn::Type>() {
            Ok(filter) => filter,
            Err(e) => return e.to_compile_error().into()
        };

        quote! {
            #filter
        }
    } else {
        quote! {
            ()
        }
    };

    let sync_target = if let Some(attr) = ast
        .attrs
        .iter()
        .find(|a| a.path().is_ident("extract_component_sync_target"))
    {
        let sync_target = match attr.parse_args::<syn::Type>() {
            Ok(sync_target) => sync_target,
            Err(e) => return e.to_compile_error().into()
        };

        quote! {
            #sync_target
        }
    } else {
        quote! {
            Self
        }
    };

    TokenStream::from(quote! {
        impl #impl_generics #wde_renderer_path::sync::SyncComponent for #struct_name #type_generics #where_clause {
            type Target = #sync_target;
        }

        impl #impl_generics #wde_renderer_path::sync::ExtractComponent for #struct_name #type_generics #where_clause {
            type QueryData = &'static Self;
            type QueryFilter = #filter;
            type Out = Self;

            fn extract_component(item: #wde_renderer_path::sync::QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
                Some(item.clone())
            }
        }
    })
}
