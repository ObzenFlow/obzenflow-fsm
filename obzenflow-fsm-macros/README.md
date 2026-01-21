# obzenflow-fsm-macros

Proc-macro helpers for `obzenflow-fsm` (derives + `fsm!` DSL).

This crate is an implementation detail of `obzenflow-fsm`. Most users should depend on
`obzenflow-fsm` and use the re-exported macros from that crate.

## Usage

```rust
use obzenflow_fsm::{fsm, EventVariant, StateVariant};

#[derive(StateVariant, EventVariant)]
enum State {
    Idle,
    Active,
}

#[derive(EventVariant)]
enum Event {
    Start,
}

let _machine = fsm! {
    state State;
    event Event;
    context ();
    action ();

    initial State::Idle;

    State::Idle {
        on Event::Start => stay;
    }
};
```

## License

Licensed under either of:

- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT license (`LICENSE-MIT`)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.

