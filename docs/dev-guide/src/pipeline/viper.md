# Viper verification stage

- VIR is converted into an actual Viper AST (JVM objects).
- (With Prusti server) Verification runs against a persistent, JVM-warm server process; results may come from a cache.
- The Viper verifier (Silicon) is called and the results are obtained.

The call chain entry point is [`prusti/src/verifier.rs` - `verify`](https://github.com/viperproject/prusti-dev/blob/master/prusti/src/verifier.rs), which calls `prusti_server::verify_programs(&env.diagnostic, vec![program])` with the `vir::ProgramRef` produced by the [encoding stage](encoding.md).

```text
vir::Program (VIR)
  -> prusti-viper::program_to_viper()   [VIR -> Viper AST, via viper::AstFactory]
  -> viper::Verifier::verify()          [runs Silicon inside a JVM]
  -> viper::VerificationResultKind      [Success / Failure / ConsistencyErrors / JavaException]
```

## Encoding VIR to Viper

VIR is an intermediate representation separate from Viper's own AST; in this step the conversion from one to the other happens. This is done by the small [`prusti-viper`](https://github.com/viperproject/prusti-dev/tree/master/prusti-viper) crate:

> - [`prusti-viper/src/lib.rs` - `program_to_viper`](https://github.com/viperproject/prusti-dev/blob/master/prusti-viper/src/lib.rs) - entry point. Builds lookup maps for the program's domains/ADTs (needed because domain function applications must be constructed with information not stored on the VIR node itself), then walks the whole `vir::Program`.
> - [`prusti-viper/src/lib.rs` - `ToViper`/`ToViperVec` traits](https://github.com/viperproject/prusti-dev/blob/master/prusti-viper/src/lib.rs) - implemented for every VIR node kind (`Method`, `Function`, `Stmt`, `Expr`, `BinOp`, `Domain`, `Adt`, `Type`, ...), each calling the matching `viper::AstFactory` method to allocate the corresponding Viper JVM object. VIR spans are converted into Viper `Position`s carrying an identifier, which is how backend error positions get mapped back to Rust spans in the [reporting stage](report.md).

## The `viper` crate: JVM lifecycle and verifier invocation

[`viper/`](https://github.com/viperproject/prusti-dev/tree/master/viper) is the hand-written, ergonomic Rust layer over the low-level [`viper-sys`](https://github.com/viperproject/prusti-dev/tree/master/viper-sys) JNI bindings (themselves generated from the Viper/Silicon/Carbon jars by [`jni-gen`](https://github.com/viperproject/prusti-dev/tree/master/jni-gen); see [Repository Layout](../layout.md)).

> - [`viper/src/viper.rs` - `Viper::new_with_args`](https://github.com/viperproject/prusti-dev/blob/master/viper/src/viper.rs) - starts an embedded JVM, given `VIPER_HOME` (Viper's jars) on the classpath.
> - [`viper/src/verification_context.rs` - `VerificationContext::new_verifier`](https://github.com/viperproject/prusti-dev/blob/master/viper/src/verification_context.rs) - attaches the current OS thread to the JVM and builds a configured `Verifier` (requires `Z3_EXE`, and `BOOGIE_EXE` for the Carbon backend).
> - [`viper/src/ast_factory/`](https://github.com/viperproject/prusti-dev/tree/master/viper/src/ast_factory) - `AstFactory`, the builder used by `prusti-viper` to construct Viper AST nodes.
> - [`viper/src/verifier.rs` - `Verifier::verify`](https://github.com/viperproject/prusti-dev/blob/master/viper/src/verifier.rs) - runs Viper consistency checks, hands the program to Silicon/Carbon, and classifies the outcome into a `VerificationResultKind` (`Success` / `Failure(Vec<VerificationError>)` / `ConsistencyErrors` / `JavaException`).
> - [`viper/src/cache.rs` - `PersistentCache`](https://github.com/viperproject/prusti-dev/blob/master/viper/src/cache.rs) - a disk-persisted, `bincode`-serialized cache of `VerificationResult`s keyed by a hash of the program, shared between local and server-mode verification (see [`ENABLE_CACHE`](../config/flags.md#enable_cache)/[`CACHE_PATH`](../config/flags.md#cache_path)).

## Prusti server

[Prusti server](https://github.com/viperproject/prusti-dev/pull/43) is an optional component of Prusti that can significantly reduce verification times by keeping a JVM/Silicon instance warm across invocations, either in a background thread of the same process or in a separate long-running process (started with `prusti-server`, see [`SERVER_ADDRESS`](../config/flags.md#server_address)).

> - [`prusti-server/src/lib.rs` - `verify_programs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-server/src/lib.rs) - the function called from `prusti/src/verifier.rs`. Builds a `VerificationRequest` per program, then dispatches either to `verify_requests_server` (remote/child server over a websocket) or `verify_requests_local` (in-process), checking the `PersistentCache` first in both cases.
> - [`prusti-server/src/verification_request.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-server/src/verification_request.rs) - holds the single process-global JVM instance (`static VIPER: OnceLock<Viper>`). `VerificationRequest::build_request` is where the VIR-to-Viper conversion (`prusti_viper::program_to_viper`) actually happens, producing a JNI `GlobalRef` that can cross thread boundaries.
> - [`prusti-server/src/process_verification.rs` - `VerificationRequestProcessing`](https://github.com/viperproject/prusti-dev/blob/master/prusti-server/src/process_verification.rs) - a single dedicated OS thread that processes verification requests *sequentially* (parallelism instead comes from within a single Silicon run, via the `number_of_parallel_verifiers` setting in `prusti_utils::config`, and from Z3 itself).
> - [`prusti-server/src/backend.rs` - `Backend::verify`](https://github.com/viperproject/prusti-dev/blob/master/prusti-server/src/backend.rs) - optionally polls Silicon's reporter on a side thread while verification runs, to stream per-method results and quantifier-instantiation statistics back to the client for incremental IDE feedback (see [`ide`](../layout.md)).
> - [`prusti-server/src/server.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-server/src/server.rs) - the `warp`-based websocket server exposing `/json/verify`, `/bincode/verify`, and `/save`, used when running `prusti-server` as a standalone process.

## SMT-level debugging

For performance debugging of the SMT solver itself, Prusti can route Z3 through an instrumented wrapper ([`prusti-smt-solver`](https://github.com/viperproject/prusti-dev/tree/master/prusti-smt-solver)) that captures full trace logs, later analyzed by [`smt-log-analyzer`](https://github.com/viperproject/prusti-dev/tree/master/smt-log-analyzer) (see [Debugging](../development/debug.md)).
