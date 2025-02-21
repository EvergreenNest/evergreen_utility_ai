use bevy_macro_utils::derive_label;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::format_ident;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(FlowLabel)]
pub fn derive_flow_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = evergreen_utility_ai_path();
    trait_path.segments.push(format_ident!("label").into());
    let mut dyn_eq_path = trait_path.clone();
    trait_path.segments.push(format_ident!("FlowLabel").into());
    dyn_eq_path.segments.push(format_ident!("DynEq").into());
    derive_label(input, "FlowLabel", &trait_path, &dyn_eq_path)
}

#[proc_macro_derive(ScoreLabel)]
pub fn derive_score_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = evergreen_utility_ai_path();
    trait_path.segments.push(format_ident!("label").into());
    let mut dyn_eq_path = trait_path.clone();
    trait_path.segments.push(format_ident!("ScoreLabel").into());
    dyn_eq_path.segments.push(format_ident!("DynEq").into());
    derive_label(input, "ScoreLabel", &trait_path, &dyn_eq_path)
}

#[proc_macro_derive(ActionLabel)]
pub fn derive_action_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = evergreen_utility_ai_path();
    trait_path.segments.push(format_ident!("label").into());
    let mut dyn_eq_path = trait_path.clone();
    trait_path
        .segments
        .push(format_ident!("ActionLabel").into());
    dyn_eq_path.segments.push(format_ident!("DynEq").into());
    derive_label(input, "ActionLabel", &trait_path, &dyn_eq_path)
}

fn evergreen_utility_ai_path() -> syn::Path {
    syn::Path::from(syn::Ident::new("evergreen_utility_ai", Span::call_site()))
}
