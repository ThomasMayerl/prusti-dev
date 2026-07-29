# Reporting stage

- Viper positions in the results are mapped back to Rust source spans.
- The Prusti client reports compilation and verification errors.

In this final stage, any errors and warnings that occurred during verification are reported to the user as ordinary `rustc` diagnostics, so they appear alongside normal compiler errors (with the correct source file, line, and column) in the terminal, in `cargo`'s JSON diagnostic output, and in the "Prusti Assistant" VS Code extension.

Because verification happens on the Viper program, errors initially only reference *Viper* positions (the identifiers assigned when [converting VIR to Viper AST](viper.md#encoding-vir-to-viper)). These need to be mapped back to Rust source spans before they mean anything to a user:

> - [`prusti-encoder/src/lib.rs` - `backtranslate_error`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/lib.rs) - given a Viper error/position id, looks up the corresponding VIR span(s) via `vir::VirCtxt::backtranslate`, producing one or more `PrustiError`s.
> - [`prusti-server/src/lib.rs` - `handle_stream`](https://github.com/viperproject/prusti-dev/blob/master/prusti-server/src/lib.rs) - consumes `ServerMessage`s from the (possibly remote) verification run, calling `backtranslate_error` for failures and turning quantifier-instantiation/block-reached messages into diagnostics or IDE information as configured.
> - [`prusti-interface/src/prusti_error.rs` - `PrustiError::emit`](https://github.com/viperproject/prusti-dev/blob/master/prusti-interface/src/prusti_error.rs) - emits a `PrustiError` as a message with a source span into the compiler's diagnostic context (`EnvDiagnostic`).

Diagnostics unrelated to verification failures proper are emitted the same way throughout the pipeline: type errors in specifications, unsupported-feature errors raised by encoders (surfaced via `prusti_encoder::early_errors`, see [Task encoders](../encoding/task-encoder.md)), and the JSON payloads consumed by the VS Code extension (see [`ide`](../layout.md)), which are also emitted as specially-prefixed diagnostic messages rather than over a separate channel.
