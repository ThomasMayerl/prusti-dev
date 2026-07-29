# Encoding stage

- The MIR of functions that should be checked is obtained.
- MIR is encoded into VIR - Prusti's intermediate representation - by a large collection of memoizing task encoders.

> - [`prusti/src/verifier.rs` - `verify`](https://github.com/viperproject/prusti-dev/blob/master/prusti/src/verifier.rs) - calls into this stage.
> - [`prusti-encoder/src/lib.rs` - `test_entrypoint`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/lib.rs) - top-level entry point for this stage.

This is by far the most involved stage of the pipeline, and has its own chapter: [Encoding Architecture](../encoding/summary.md). This page only summarizes the shape of the process; see that chapter for how the pieces work.

## MIR and VIR

Prusti starts with Rust's [MIR (mid-level intermediate representation)](https://rustc-dev-guide.rust-lang.org/mir/index.html). The MIR is a [CFG](https://en.wikipedia.org/wiki/Control-flow_graph)-based representation that encodes Rust in a highly simplified (and therefore easier to programmatically process) manner.

The output of this stage is VIR (Prusti's own intermediate representation, in the [`vir`](https://github.com/viperproject/prusti-dev/tree/master/vir) crate). VIR is similar to Viper's own AST, but arena-allocated and CFG-based, with a few extra node kinds (e.g. lazily-constructed expressions) to make encoding more convenient. Converting VIR into Viper's actual AST is a separate, later step (see [Viper verification stage](viper.md)) and is comparatively mechanical. See [The VIR intermediate representation](../encoding/vir.md).

## Driving the encoding: task encoders

Rather than a single pass that walks the whole crate, `test_entrypoint` calls [`encode_all_in_crate`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_fn/mod.rs), which enqueues every function/method/trait/impl in the crate with the relevant top-level `TaskEncoder`s (e.g. `MethodEnc` for impure functions, `TraitEnc`/`TraitImplEnc` for traits). Everything else — types used by those functions, functions transitively called by them, casts, constants, and so on — is pulled in lazily as each encoder recursively `require`s the output of other encoders. Once the queue is drained, every encoder's accumulated cache is flushed into a single `vir::Program` via `emit_outputs`. See [Task encoders](../encoding/task-encoder.md).

## Two closely related encodings: pure and impure

Because Prusti specifications can refer to Rust functions ("pure" functions used only for their return value, with no visible side effects) as well as check Rust functions for memory safety and correctness ("impure" functions, i.e. ordinary procedures), most kinds of Rust item (types in particular) end up with two different Viper encodings side by side: a heap-independent one used inside Viper functions/specifications, and a heap/permission-based one used for procedure bodies. See [Type encoding](../encoding/types.md), [Procedure encoding and the PCG](../encoding/procedures.md), and [Pure function encoding](../encoding/pure.md).

## Fold/unfold placement during encoding

Unlike the pre-rewrite pipeline, `fold`/`unfold` (and equivalent) statements are not inferred by a separate fixpoint pass running on the already-encoded Viper CFG. Instead, the impure procedure encoder queries a borrow-aware permission analysis (the PCG) *while* walking MIR, and emits the required Viper statements directly. See [Procedure encoding and the PCG](../encoding/procedures.md).
