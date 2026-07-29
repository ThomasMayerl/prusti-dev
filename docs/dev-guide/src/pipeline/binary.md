# Binary stage

- The user invokes `cargo prusti` or `prusti-rustc <file.rs>`.
- The launcher sets up important environment variables, then invokes `prusti-driver`.

This is a short stage responsible for setting up the correct environment variables for the Rust compiler and the Prusti-supporting libraries. The launcher binaries (`prusti-rustc`, `cargo-prusti`, `prusti-server`) live in the [`prusti-launch`](https://github.com/viperproject/prusti-dev/tree/master/prusti-launch) crate and share helpers from `prusti_utils::launch`. To invoke `prusti-driver` correctly, the following arguments are set up:

 - `--sysroot`, pointing to the Rust sysroot where its libraries are stored (`launch::prusti_sysroot`).
 - `-L`, adding the directory containing the compiled `prusti-contracts` crate into the library search path.
 - `--extern` arguments (via `launch::PRUSTI_LIBS`) for the compiled dynamic libraries containing the Prusti specification proc-macros, so that user code does not need an explicit `extern crate prusti_contracts;` (the driver also injects a synthetic one, see [`PrustiCompilerCalls::after_crate_root_parsing`](https://github.com/viperproject/prusti-dev/blob/master/prusti/src/callbacks.rs)).

`cargo-prusti` additionally shells out to `cargo` itself (respecting the [`CARGO_COMMAND`](../config/flags.md#cargo_command) flag), setting `RUSTC_WRAPPER`/environment so that `prusti-driver` is invoked in place of `rustc` for each crate in the dependency graph.

> - [`prusti-launch/src/bin/prusti-rustc.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-launch/src/bin/prusti-rustc.rs)
> - [`prusti-launch/src/bin/cargo-prusti.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-launch/src/bin/cargo-prusti.rs)
> - [`prusti-utils/src/launch/mod.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-utils/src/launch/mod.rs)
