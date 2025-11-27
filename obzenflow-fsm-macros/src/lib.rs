//! Proc-macro helpers for `obzenflow-fsm`.
//!
//! This crate is an implementation detail of `obzenflow-fsm` and is not
//! intended to be used directly. End users should depend on `obzenflow-fsm`
//! and use:
//!   - `#[derive(StateVariant, EventVariant)]`
//!   - `fsm! { ... }`

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive `::obzenflow_fsm::StateVariant` for an enum.
#[proc_macro_derive(StateVariant)]
pub fn derive_state_variant(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_ident = input.ident;

    let data_enum = match input.data {
        syn::Data::Enum(e) => e,
        _ => {
            return syn::Error::new_spanned(
                enum_ident,
                "StateVariant can only be derived for enums",
            )
            .to_compile_error()
            .into();
        }
    };

    let arms = data_enum.variants.iter().map(|variant| {
        let ident = &variant.ident;
        // Ignore fields; we only care about the variant name.
        quote! {
            Self::#ident { .. } => stringify!(#ident),
        }
    });

    let expanded = quote! {
        impl ::obzenflow_fsm::StateVariant for #enum_ident {
            fn variant_name(&self) -> &str {
                match self {
                    #( #arms )*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive `::obzenflow_fsm::EventVariant` for an enum.
#[proc_macro_derive(EventVariant)]
pub fn derive_event_variant(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_ident = input.ident;

    let data_enum = match input.data {
        syn::Data::Enum(e) => e,
        _ => {
            return syn::Error::new_spanned(
                enum_ident,
                "EventVariant can only be derived for enums",
            )
            .to_compile_error()
            .into();
        }
    };

    let arms = data_enum.variants.iter().map(|variant| {
        let ident = &variant.ident;
        quote! {
            Self::#ident { .. } => stringify!(#ident),
        }
    });

    let expanded = quote! {
        impl ::obzenflow_fsm::EventVariant for #enum_ident {
            fn variant_name(&self) -> &str {
                match self {
                    #( #arms )*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// High-level typed FSM builder DSL.
///
/// This is a first, minimal implementation that focuses on the core pieces:
/// - Top-level state/event/context/action/initial declarations.
/// - `state` blocks with `on` clauses.
///
/// The macro expands to a `FsmBuilder` pipeline ending in `.build()`.
#[proc_macro]
pub fn fsm(input: TokenStream) -> TokenStream {
    // Parse the incoming token stream into our simplified FSM spec,
    // then expand it into a `FsmBuilder` chain that ends in `.build()`.
    let input_ts = proc_macro2::TokenStream::from(input);

    let parsed = match crate::parse::FsmSpec::parse(input_ts) {
        Ok(spec) => spec,
        Err(err) => return err.to_compile_error().into(),
    };

    let expanded = crate::codegen::expand_fsm_spec(&parsed);
    TokenStream::from(expanded)
}

mod parse;
mod codegen;
