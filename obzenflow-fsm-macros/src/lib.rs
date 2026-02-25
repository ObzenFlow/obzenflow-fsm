// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Proc-macro helpers for `obzenflow-fsm`.
//!
//! This crate is an implementation detail of `obzenflow-fsm`. End users should depend on
//! `obzenflow-fsm` and use the re-exported macros from that crate:
//! - `#[derive(obzenflow_fsm::StateVariant, obzenflow_fsm::EventVariant)]`
//! - `obzenflow_fsm::fsm! { ... }`

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive `::obzenflow_fsm::StateVariant` for an enum.
///
/// The generated `variant_name()` matches on the enum and returns the variant identifier as a
/// `&'static str`, ignoring any payload.
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
        match &variant.fields {
            syn::Fields::Unit => quote! { Self::#ident => stringify!(#ident), },
            syn::Fields::Unnamed(_) => quote! { Self::#ident(..) => stringify!(#ident), },
            syn::Fields::Named(_) => quote! { Self::#ident { .. } => stringify!(#ident), },
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
///
/// The generated `variant_name()` matches on the enum and returns the variant identifier as a
/// `&'static str`, ignoring any payload.
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
        match &variant.fields {
            syn::Fields::Unit => quote! { Self::#ident => stringify!(#ident), },
            syn::Fields::Unnamed(_) => quote! { Self::#ident(..) => stringify!(#ident), },
            syn::Fields::Named(_) => quote! { Self::#ident { .. } => stringify!(#ident), },
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
/// This macro is re-exported as `obzenflow_fsm::fsm!` and is the recommended way to construct a
/// `obzenflow_fsm::StateMachine`.
///
/// Supported syntax (first pass):
/// - Top-level `state:`, `event:`, `context:`, `action:`, `initial:`.
/// - Optional `unhandled => handler;` at top-level.
/// - `state <State::Variant> { ... }` blocks containing:
///   - `on <Event::Variant> => handler;`
///   - `timeout <expr> => handler;`
///   - `on_entry handler;`
///   - `on_exit handler;`
///
/// Handler shapes:
/// - `on` handlers take `(&State, &Event, &mut Context)` and return a `Transition`.
/// - `timeout` handlers take `(&State, &mut Context)` and return a `Transition`.
/// - `on_entry` / `on_exit` handlers take `(&State, &mut Context)` and return `Vec<Action>`.
/// - `unhandled` handlers take `(&State, &Event, &mut Context)` and return `()`.
///
/// The macro currently expands to a legacy `FsmBuilder` pipeline ending in `.build()`.
/// As a result, dispatch is performed by `StateVariant::variant_name()` and
/// `EventVariant::variant_name()` (not by full type paths).
///
/// ```rust,ignore
/// use obzenflow_fsm::{fsm, Transition};
///
/// #[derive(Clone, Debug, PartialEq, obzenflow_fsm::StateVariant)]
/// enum State {
///     Idle,
/// }
///
/// #[derive(Clone, Debug, obzenflow_fsm::EventVariant)]
/// enum Event {
///     Start,
/// }
///
/// #[derive(Clone, Debug, PartialEq)]
/// enum Action {
///     Noop,
/// }
///
/// #[derive(Default)]
/// struct Context;
///
/// impl obzenflow_fsm::FsmContext for Context {}
///
/// #[async_trait::async_trait]
/// impl obzenflow_fsm::FsmAction for Action {
///     type Context = Context;
///
///     async fn execute(&self, _ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
///         Ok(())
///     }
/// }
///
/// let _machine = fsm! {
///     state:   State;
///     event:   Event;
///     context: Context;
///     action:  Action;
///     initial: State::Idle;
///
///     unhandled => |_s: &State, _e: &Event, _ctx: &mut Context| {
///         Box::pin(async move { Ok(()) })
///     };
///
///     state State::Idle {
///         on Event::Start => |_s: &State, _e: &Event, _ctx: &mut Context| {
///             Box::pin(async move {
///                 Ok(Transition {
///                     next_state: State::Idle,
///                     actions: vec![Action::Noop],
///                 })
///             })
///         };
///     }
/// };
/// ```
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

mod codegen;
mod parse;
