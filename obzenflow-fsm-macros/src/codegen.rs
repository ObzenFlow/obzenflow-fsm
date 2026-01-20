use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, LitStr};

use crate::parse::{FsmSpec, OnHandler, StateBlock, TimeoutSpec};

pub fn expand_fsm_spec(spec: &FsmSpec) -> TokenStream {
    let state_ty = &spec.state_ty;
    let event_ty = &spec.event_ty;
    let context_ty = &spec.context_ty;
    let action_ty = &spec.action_ty;
    let initial_expr = &spec.initial_expr;

    // We expand into a block that:
    // - Creates a mutable builder,
    // - Applies optional top-level `unhandled` handler,
    // - Applies each state's entry/exit/timeout/on clauses,
    // - Finishes with `builder.build()`.
    let mut stmts: TokenStream = quote! {
        let mut builder = ::obzenflow_fsm::internal::FsmBuilder::<#state_ty, #event_ty, #context_ty, #action_ty>::new(#initial_expr);
    };

    if let Some(unhandled) = &spec.unhandled {
        stmts.extend(quote! {
            builder = builder.when_unhandled(#unhandled);
        });
    }

    for StateBlock {
        state_path,
        handlers,
        timeout,
        entry,
        exit,
    } in &spec.states
    {
        // Build the `.on(...)` chain for this state.
        let mut on_chain: TokenStream = TokenStream::new();
        for OnHandler {
            event_path,
            handler,
        } in handlers
        {
            // Use the final path segment as the stable string name.
            let ident = event_path
                .path
                .segments
                .last()
                .expect("event path has at least one segment")
                .ident
                .to_string();
            let lit = LitStr::new(&ident, event_path.path.span());
            let event_name_expr = quote! { #lit };

            on_chain.extend(quote! {
                .on(#event_name_expr, #handler)
            });
        }

        // Use the final path segment as the state name.
        let state_ident = state_path
            .path
            .segments
            .last()
            .expect("state path has at least one segment")
            .ident
            .to_string();
        let state_lit = LitStr::new(&state_ident, state_path.path.span());

        // Optional timeout chain: `.timeout(duration, handler)` after `.when`.
        let timeout_chain: TokenStream = if let Some(TimeoutSpec { duration, handler }) = timeout {
            quote! { .when(#state_lit).timeout(#duration, #handler) }
        } else {
            quote! { .when(#state_lit) }
        };

        // Optional entry/exit handlers as separate builder calls.
        let mut entry_exit_stmts: TokenStream = TokenStream::new();
        if let Some(entry_handler) = entry {
            entry_exit_stmts.extend(quote! {
                b = b.on_entry(#state_lit, #entry_handler);
            });
        }
        if let Some(exit_handler) = exit {
            entry_exit_stmts.extend(quote! {
                b = b.on_exit(#state_lit, #exit_handler);
            });
        }

        stmts.extend(quote! {
            builder = {
                let mut b = builder;
                #entry_exit_stmts
                b = b
                    #timeout_chain
                    #on_chain
                    .done();
                b
            };
        });
    }

    quote! {{
        #[allow(deprecated)]
        {
            #stmts
            builder.build()
        }
    }}
}
