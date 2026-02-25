// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{braced, parse2, Error, Expr, ExprPath, Ident, Result, Token, Type};

/// A minimal parsed representation of the `fsm!` DSL.
///
/// First pass supports:
/// - Top-level `state:`, `event:`, `context:`, `action:`, `initial:`.
/// - An optional top-level `unhandled => handler;` block.
/// - Multiple `state` blocks with:
///   - `on Event::Variant => handler;`
///   - `timeout <expr> => handler;`
///   - `on_entry handler;`
///   - `on_exit handler;`
pub struct FsmSpec {
    pub state_ty: Type,
    pub event_ty: Type,
    pub context_ty: Type,
    pub action_ty: Type,
    pub initial_expr: Expr,
    pub states: Vec<StateBlock>,
    pub unhandled: Option<Expr>,
}

pub struct StateBlock {
    pub state_path: ExprPath,
    pub handlers: Vec<OnHandler>,
    pub timeout: Option<TimeoutSpec>,
    pub entry: Option<Expr>,
    pub exit: Option<Expr>,
}

pub struct OnHandler {
    pub event_path: ExprPath,
    pub handler: Expr,
}

pub struct TimeoutSpec {
    pub duration: Expr,
    pub handler: Expr,
}

impl FsmSpec {
    pub fn parse(tokens: TokenStream) -> Result<Self> {
        parse2::<FsmSpec>(tokens)
    }
}

impl Parse for FsmSpec {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // state: Type;
        input.parse::<Ident>().and_then(|id| {
            if id == "state" {
                Ok(id)
            } else {
                Err(Error::new_spanned(id, "expected `state:`"))
            }
        })?;
        input.parse::<Token![:]>()?;
        let state_ty: Type = input.parse()?;
        input.parse::<Token![;]>()?;

        // event: Type;
        input.parse::<Ident>().and_then(|id| {
            if id == "event" {
                Ok(id)
            } else {
                Err(Error::new_spanned(id, "expected `event:`"))
            }
        })?;
        input.parse::<Token![:]>()?;
        let event_ty: Type = input.parse()?;
        input.parse::<Token![;]>()?;

        // context: Type;
        input.parse::<Ident>().and_then(|id| {
            if id == "context" {
                Ok(id)
            } else {
                Err(Error::new_spanned(id, "expected `context:`"))
            }
        })?;
        input.parse::<Token![:]>()?;
        let context_ty: Type = input.parse()?;
        input.parse::<Token![;]>()?;

        // action: Type;
        input.parse::<Ident>().and_then(|id| {
            if id == "action" {
                Ok(id)
            } else {
                Err(Error::new_spanned(id, "expected `action:`"))
            }
        })?;
        input.parse::<Token![:]>()?;
        let action_ty: Type = input.parse()?;
        input.parse::<Token![;]>()?;

        // initial: Expr;
        input.parse::<Ident>().and_then(|id| {
            if id == "initial" {
                Ok(id)
            } else {
                Err(Error::new_spanned(id, "expected `initial:`"))
            }
        })?;
        input.parse::<Token![:]>()?;
        let initial_expr: Expr = input.parse()?;
        input.parse::<Token![;]>()?;

        let mut states = Vec::new();
        let mut unhandled: Option<Expr> = None;

        while !input.is_empty() {
            let kw: Ident = input.parse()?;
            if kw == "state" {
                let state_path: ExprPath = input.parse()?;
                let content;
                braced!(content in input);

                let mut handlers = Vec::new();
                let mut timeout: Option<TimeoutSpec> = None;
                let mut entry: Option<Expr> = None;
                let mut exit: Option<Expr> = None;

                while !content.is_empty() {
                    let inner_kw: Ident = content.parse()?;
                    if inner_kw == "on" {
                        let event_path: ExprPath = content.parse()?;
                        content.parse::<Token![=>]>()?;
                        let handler: Expr = content.parse()?;
                        content.parse::<Token![;]>()?;

                        handlers.push(OnHandler {
                            event_path,
                            handler,
                        });
                    } else if inner_kw == "timeout" {
                        if timeout.is_some() {
                            return Err(Error::new_spanned(
                                inner_kw,
                                "duplicate `timeout` for state",
                            ));
                        }
                        let duration: Expr = content.parse()?;
                        content.parse::<Token![=>]>()?;
                        let handler: Expr = content.parse()?;
                        content.parse::<Token![;]>()?;
                        timeout = Some(TimeoutSpec { duration, handler });
                    } else if inner_kw == "on_entry" {
                        if entry.is_some() {
                            return Err(Error::new_spanned(
                                inner_kw,
                                "duplicate `on_entry` for state",
                            ));
                        }
                        let handler: Expr = content.parse()?;
                        content.parse::<Token![;]>()?;
                        entry = Some(handler);
                    } else if inner_kw == "on_exit" {
                        if exit.is_some() {
                            return Err(Error::new_spanned(
                                inner_kw,
                                "duplicate `on_exit` for state",
                            ));
                        }
                        let handler: Expr = content.parse()?;
                        content.parse::<Token![;]>()?;
                        exit = Some(handler);
                    } else {
                        return Err(Error::new_spanned(
                            inner_kw,
                            "expected `on`, `timeout`, `on_entry`, or `on_exit`",
                        ));
                    }
                }

                states.push(StateBlock {
                    state_path,
                    handlers,
                    timeout,
                    entry,
                    exit,
                });
            } else if kw == "unhandled" {
                if unhandled.is_some() {
                    return Err(Error::new_spanned(
                        kw,
                        "duplicate `unhandled` handler at top-level",
                    ));
                }
                input.parse::<Token![=>]>()?;
                let handler: Expr = input.parse()?;
                input.parse::<Token![;]>()?;
                unhandled = Some(handler);
            } else {
                return Err(Error::new_spanned(kw, "expected `state` or `unhandled`"));
            }
        }

        Ok(FsmSpec {
            state_ty,
            event_ty,
            context_ty,
            action_ty,
            initial_expr,
            states,
            unhandled,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_only() {
        let src = r#"
            state:   SimpleState;
            event:   SimpleEvent;
            context: SimpleContext;
            action:  SimpleAction;
            initial: SimpleState::Idle;
        "#;

        let tokens: TokenStream = syn::parse_str(src).expect("tokenize");
        let spec = FsmSpec::parse(tokens).expect("parse FsmSpec");

        assert!(spec.states.is_empty());
    }

    #[test]
    fn parses_event_expr_and_handler_expr() {
        // Ensure syn can parse the kinds of expressions we expect to see in
        // `on` clauses.
        let _event: Expr = syn::parse_str("SimpleEvent::Start").expect("parse event expr");
        let _handler: Expr = syn::parse_str(
            r#"|_state: &SimpleState, _event: &SimpleEvent, _ctx: &mut SimpleContext| {
                Box::pin(async move {
                    Ok(Transition {
                        next_state: SimpleState::Running,
                        actions: vec![SimpleAction::MarkStarted],
                    })
                })
            }"#,
        )
        .expect("parse handler expr");
    }

    #[test]
    fn parses_full_sample_like_runtime_test() {
        let src = r#"
            state:   SimpleState;
            event:   SimpleEvent;
            context: SimpleContext;
            action:  SimpleAction;
            initial: SimpleState::Idle;

            state SimpleState::Idle {
                on SimpleEvent::Start => |_state: &SimpleState, _event: &SimpleEvent, _ctx: &mut SimpleContext| {
                    Box::pin(async move {
                        Ok(Transition {
                            next_state: SimpleState::Running,
                            actions: vec![SimpleAction::MarkStarted],
                        })
                    })
                };
            }
        "#;

        let tokens: TokenStream = syn::parse_str(src).expect("tokenize");
        let _spec = FsmSpec::parse(tokens).expect("parse FsmSpec");
    }
}
