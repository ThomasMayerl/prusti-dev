# Verification Pipeline

At a high level, Prusti is a plugin to the [Rust compiler](https://rustc-dev-guide.rust-lang.org/) that converts Rust code enriched with Prusti specifications into [Viper code](https://viper.ethz.ch), verifies the code with an external verifier, and then reports the results back to the user. See [Architecture at a Glance](../architecture.md) for a one-page picture of the whole system.

This chapter summarizes the steps that take place when the user runs Prusti on a given Rust file. The steps are described in greater detail in the subsequent sections. Although we present the steps in separate "stages", this distinction only exists for the purposes of this guide, and is not clearly mirrored in the codebase.

1. [Binary stage](binary.md)
    - The user invokes `cargo prusti` or `prusti-rustc <file.rs>`.
    - The launcher sets up important environment variables, then invokes `prusti-driver`.
2. [Rust compilation stage](rust.md)
    - `prusti-driver` registers callbacks and calls the Rust compiler.
    - The compiler expands all procedural macros, which includes the specifications.
    - The compiler does type-checking and (NLL/Polonius) borrow-checking; Prusti overrides the relevant queries to retain the borrow-checking facts it needs later.
    - The Prusti callback is called.
3. [Specification collection](specs.md)
    - Specifications, still present in the HIR as serialized data in dummy attributes, are collected and type-checked into a `DefSpecificationMap`.
4. [Encoding stage](encoding.md)
    - The [MIR](https://rustc-dev-guide.rust-lang.org/mir/index.html) of functions that should be checked is obtained.
    - MIR is encoded into VIR - Prusti's intermediate representation - by a large collection of memoizing task encoders, using the PCG borrow/permission analysis to place `fold`/`unfold`-equivalent statements directly during encoding.
5. [Viper verification stage](viper.md)
    - VIR is converted into an actual Viper AST (JVM objects).
    - (With Prusti server) Verification runs against a persistent, JVM-warm server process; results may come from a cache.
    - The Viper verifier (Silicon) is called and the results are obtained.
6. [Reporting stage](report.md)
    - Viper positions in the results are mapped back to Rust source spans.
    - The Prusti client reports compilation and verification errors.

Prusti supports tracing which can be useful to get an idea of the most important functions in stages 3 to 6. To enable this, set the [`log`](../config/flags.md#log) flag, for example:

> `./x.py run --bin prusti-rustc -- --edition=2021 prusti-tests/**/nfm22/bst_generics.rs -Plog=debug`

Prusti then generates a `log/trace.json` file which can be opened in [ui.perfetto.dev](https://ui.perfetto.dev/) to visualize the trace.
