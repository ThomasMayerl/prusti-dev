# Rust compilation stage

- `prusti-driver` registers callbacks and calls the Rust compiler.
- The compiler expands all [procedural macros](specs.md), which includes the specifications.
- The compiler does type-checking and borrow-checking.
- The Prusti callback is called.

In this stage, the actual Rust compiler is invoked to make sure the given code is valid Rust code, both syntactically and semantically. Prusti integrates with the Rust compiler using [`rustc_driver`](https://rustc-dev-guide.rust-lang.org/rustc-driver.html). This allows running the Rust compiler with [callbacks](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_driver/trait.Callbacks.html) triggered when important stages of processing are completed. In Prusti, we are interested in a few hooks:

 - [`prusti/src/callbacks.rs` - `PrustiCompilerCalls::config`](https://github.com/viperproject/prusti-dev/blob/master/prusti/src/callbacks.rs) - overrides the `mir_borrowck` and `mir_promoted` query providers *before* the compiler runs them. The overridden `mir_borrowck` calls `rustc_borrowck::consumers::get_bodies_with_borrowck_facts` and stashes the resulting NLL/Polonius facts (region inference context, borrow set, location table, ...) into a thread-local store (`prusti_interface::environment::mir_storage`). This is essential groundwork for the [PCG analysis](../encoding/procedures.md) used later during encoding, which needs exactly these facts; without this override they would not be retained by a normal compiler run.
 - [`prusti/src/callbacks.rs` - `PrustiCompilerCalls::after_crate_root_parsing`](https://github.com/viperproject/prusti-dev/blob/master/prusti/src/callbacks.rs) - injects a synthetic `extern crate prusti_contracts;` if the user did not add one, so the crate (and its default extern specs for things like `assert!`) is always loaded.
 - [`prusti/src/callbacks.rs` - `PrustiCompilerCalls::after_expansion`](https://github.com/viperproject/prusti-dev/blob/master/prusti/src/callbacks.rs) - for debugging purposes (the [`PRINT_DESUGARED_SPECS`](../config/flags.md#print_desugared_specs) flag), used to print the Rust AST after the Prusti specification macros are expanded.
 - [`prusti/src/callbacks.rs` - `PrustiCompilerCalls::after_analysis`](https://github.com/viperproject/prusti-dev/blob/master/prusti/src/callbacks.rs) - once type-checking is done and the code is determined to be type safe. This is where Prusti actually takes over: collecting specifications ([next section](specs.md)) and, unless verification is disabled, calling into `prusti_encoder`/`prusti_server` ([Encoding stage](encoding.md)). Code that is syntactically invalid or has type errors is not relevant for Prusti applications.

Because `rustc`'s incremental compilation would otherwise skip calling `mir_borrowck` for unchanged code, `prusti-driver` also unconditionally disables incremental compilation (see `driver.rs`).
