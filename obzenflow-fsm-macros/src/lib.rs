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

/// Derive `::obzenflow_fsm::StateVariant` for an enum, generic or not.
///
/// The generated `variant_name()` returns the variant identifier, ignoring any payload. `fsm!`
/// accepts only state types that derive it.
#[proc_macro_derive(StateVariant)]
pub fn derive_state_variant(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_variant(
        input,
        quote! { ::obzenflow_fsm::StateVariant },
        quote! { ::obzenflow_fsm::internal::DerivedStateVariant },
        quote! {
            ::core::clone::Clone
                + ::core::fmt::Debug
                + ::core::cmp::PartialEq
                + ::core::marker::Send
                + ::core::marker::Sync
                + 'static
        },
        "StateVariant",
    )
    .into()
}

/// Derive `::obzenflow_fsm::EventVariant` for an enum, generic or not.
///
/// The generated `variant_name()` returns the variant identifier, ignoring any payload. `fsm!`
/// accepts only event types that derive it.
#[proc_macro_derive(EventVariant)]
pub fn derive_event_variant(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_variant(
        input,
        quote! { ::obzenflow_fsm::EventVariant },
        quote! { ::obzenflow_fsm::internal::DerivedEventVariant },
        quote! {
            ::core::clone::Clone
                + ::core::fmt::Debug
                + ::core::marker::Send
                + ::core::marker::Sync
                + 'static
        },
        "EventVariant",
    )
    .into()
}

fn derive_variant(
    input: DeriveInput,
    trait_path: proc_macro2::TokenStream,
    marker_path: proc_macro2::TokenStream,
    supertraits: proc_macro2::TokenStream,
    trait_name: &str,
) -> proc_macro2::TokenStream {
    let DeriveInput {
        ident,
        generics,
        data,
        ..
    } = input;

    let syn::Data::Enum(data_enum) = data else {
        return syn::Error::new_spanned(
            ident,
            format!("{trait_name} can only be derived for enums"),
        )
        .to_compile_error();
    };

    let arms = data_enum.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        quote! { Self::#variant_ident { .. } => ::core::stringify!(#variant_ident), }
    });

    let (impl_generics, ty_generics, _) = generics.split_for_impl();
    let mut where_clause = generics.where_clause.clone();
    if !generics.params.is_empty() {
        where_clause
            .get_or_insert_with(|| syn::parse_quote!(where))
            .predicates
            .push(syn::parse_quote!(Self: #supertraits));
    }

    quote! {
        impl #impl_generics #trait_path for #ident #ty_generics #where_clause {
            fn variant_name(&self) -> &str {
                match self {
                    #( #arms )*
                }
            }
        }

        impl #impl_generics #marker_path for #ident #ty_generics #where_clause {}
    }
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
/// The macro expands to the library's builder, ending in `.build()`. Dispatch is performed by
/// `StateVariant::variant_name()` and `EventVariant::variant_name()`, and the state and event
/// types must derive both, so every name is a variant identifier. Each `state` block and `on`
/// row also names its variant as a pattern, so a variant its enum does not have fails to
/// compile at that block or row.
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
