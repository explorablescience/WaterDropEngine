//! This crate contains the derive macros for the [`ExtractComponent`] and [`ExtractResource`] macros.
//! See the documentation of [`wde_renderer`] for more details on how to use these macros.

mod extract_component;
mod extract_resource;
mod manifest;

use proc_macro::TokenStream;

use crate::manifest::WdeManifest;

pub(crate) fn wde_renderer_path() -> syn::Path {
    WdeManifest::shared(|manifest| manifest.get_path("wde_renderer"))
}

#[proc_macro_derive(ExtractResource)]
pub fn derive_extract_resource(input: TokenStream) -> TokenStream {
    extract_resource::derive_extract_resource(input)
}

#[proc_macro_derive(ExtractComponent, attributes(extract_component_filter))]
pub fn derive_extract_component(input: TokenStream) -> TokenStream {
    extract_component::derive_extract_component(input)
}
