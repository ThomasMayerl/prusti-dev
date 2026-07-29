# Architecture at a Glance

Prusti is a `rustc` plugin: it hooks into the Rust compiler, waits until Rust code has been type-checked, then translates ("encodes") the checked code and its specifications into a program in [Viper](https://viper.ethz.ch), an intermediate verification language, and asks a Viper backend (Silicon, an SMT-based symbolic execution engine) to prove it correct. Errors reported by Viper are translated back into Rust source positions and reported to the user as compiler diagnostics.

## The big picture

```text
 Rust source (with #[requires], #[ensures], #[pure], ... attributes)
        │
        │  rustc: parsing, procedural macro expansion (prusti-contracts)
        ▼
 Type-checked HIR/MIR + serialized specs stashed in dummy attributes
        │
        │  prusti-interface: collect + type-check specifications
        ▼
 Environment (TyCtxt access) + DefSpecificationMap
        │
        │  prusti-encoder (using task-encoder, vir, pcg)
        ▼
 vir::Program   (Prusti's own IR: methods, functions, predicates, domains, adts)
        │
        │  prusti-viper: vir::Program -> Viper AST (JVM objects, via `viper`/`viper-sys`)
        ▼
 Viper program (viper::Program)
        │
        │  prusti-server + viper: run Silicon/Carbon inside an embedded/pooled JVM
        ▼
 Verification result (per-method success/failure + counterexamples)
        │
        │  prusti-server + prusti-encoder::backtranslate_error: map Viper positions back to Rust spans
        ▼
 rustc diagnostics shown to the user
```

Each arrow above is owned by a different set of crates; see [Repository Layout](layout.md) for the full list and [Verification Pipeline](pipeline/summary.md) for a stage-by-stage walkthrough.

## The core idea of the rewrite: encoders as memoized tasks

The pre-rewrite encoder walked a function's MIR once, top to bottom, producing Viper code directly, with a single mutable `Encoder` threading (nearly) all shared state. The new architecture instead expresses the whole encoding process as a graph of small, independent **task encoders** (see [Task encoders](encoding/task-encoder.md)): each implements the `TaskEncoder` trait from the [`task-encoder`](https://github.com/viperproject/prusti-dev/tree/master/task-encoder) crate, is keyed by a hashable "task" (e.g. *encode this `DefId` with these generic arguments*), and can recursively request ("require") the output of other task encoders. Results are cached per task, cycles are detected, and errors from a dependency propagate up as a chain that can be reported at the point that needs it.

This has a few consequences that matter when navigating the code:

- Encoding logic for a given *concern* (e.g. "how is a struct's layout represented in Viper", "how is a function call encoded") lives in one place — a single `TaskEncoder` impl in [`prusti-encoder/src/encoders/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-encoder/src/encoders) — rather than being interleaved with the code that consumes it.
- There is no single "main loop" that visits the whole crate in order; instead, [`encode_all_in_crate`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_fn/mod.rs) enqueues the crate's functions/traits, and everything else (types used by those functions, functions called by those functions, ...) is pulled in on demand by `deps.require_ref`/`require_local`/`require_dep` calls scattered across the encoders.
- If you want to change how some Rust feature is encoded, search for the `TaskEncoder` impl responsible (its name usually matches the feature, e.g. `TyUseImpureEnc`, `MirBuiltinBinOpEnc`, `TraitImplEnc`) rather than looking for a single "big" encoding function.

## The other core idea: borrow-aware permissions via the PCG

The pre-rewrite encoder inferred Viper `fold`/`unfold` statements with a separate fixpoint pass (`prusti-viper`'s `foldunfold` module) operating on the already-encoded Viper CFG, plus a hand-rolled "reborrowing DAG" for tracking borrows. The new architecture instead runs a dedicated analysis — the **Place Capability Graph** (PCG, in the [`pcg`](https://github.com/viperproject/prusti-dev/tree/master/pcg) crate) — directly over MIR, using the same NLL/Polonius borrow-checking facts `rustc` itself computed. For each program point, the PCG tracks which places currently have which capability (unique/exclusive, read, or none) and how borrows/reborrows connect places across the CFG. The impure procedure encoder ([`MethodEnc`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_fn/method.rs)) walks the MIR body block-by-block, and at each statement asks the PCG's precomputed results for the repack operations (place expand/collapse) needed at that point, translating them directly into Viper `fold`/`unfold`/`Assert`/predicate-access statements instead of inferring them afterwards. See [Procedure encoding and the PCG](encoding/procedures.md).

## "Where do I change...?" quick index

| I want to change... | Look at |
| --- | --- |
| The compiler front-end / driver setup, callbacks into `rustc` | [`prusti/`](https://github.com/viperproject/prusti-dev/tree/master/prusti) |
| How `cargo prusti` / `prusti-rustc` are invoked from the command line | [`prusti-launch/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-launch) |
| Specification syntax (`requires`, `ensures`, `pure`, `predicate!`, ...) | [`prusti-contracts/prusti-specs/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-contracts/prusti-specs) (see [Specification collection](pipeline/specs.md)) |
| How specifications are collected/type-checked from HIR | [`prusti-interface/src/specs/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-interface/src/specs) |
| Configuration flags (`-P...`, `Prusti.toml`) | [`prusti-utils/src/config.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-utils/src/config.rs), see [Configuration](config/summary.md) |
| How a Rust type is represented in Viper | [`prusti-encoder/src/encoders/ty/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-encoder/src/encoders/ty), see [Type encoding](encoding/types.md) |
| How a function/method body is turned into Viper statements | [`prusti-encoder/src/encoders/mir_impure.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_impure.rs), [`mir_fn/method.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_fn/method.rs) |
| How permissions/borrows are tracked to place `fold`/`unfold` | [`pcg/`](https://github.com/viperproject/prusti-dev/tree/master/pcg), see [Procedure encoding and the PCG](encoding/procedures.md) |
| How pure Rust expressions (used in specs) are encoded | [`prusti-encoder/src/encoders/mir_pure.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_pure.rs), see [Pure function encoding](encoding/pure.md) |
| The intermediate representation itself (add a new VIR node kind) | [`vir/src/data.rs`](https://github.com/viperproject/prusti-dev/blob/master/vir/src/data.rs), [`vir/src/gendata.rs`](https://github.com/viperproject/prusti-dev/blob/master/vir/src/gendata.rs), see [The VIR intermediate representation](encoding/vir.md) |
| How VIR is turned into actual Viper AST (JVM objects) | [`prusti-viper/src/lib.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-viper/src/lib.rs) |
| JVM/Silicon invocation, caching, background server | [`prusti-server/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-server), [`viper/`](https://github.com/viperproject/prusti-dev/tree/master/viper) |
| Low-level JNI bindings to Viper's Java/Scala classes | [`viper-sys/`](https://github.com/viperproject/prusti-dev/tree/master/viper-sys) (generated by [`jni-gen/`](https://github.com/viperproject/prusti-dev/tree/master/jni-gen)) |
| VS Code / "Prusti Assistant" integration (hover info, selective verification) | [`ide/`](https://github.com/viperproject/prusti-dev/tree/master/ide) |
| SMT/Z3 performance debugging | [`prusti-smt-solver/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-smt-solver), [`smt-log-analyzer/`](https://github.com/viperproject/prusti-dev/tree/master/smt-log-analyzer) |
