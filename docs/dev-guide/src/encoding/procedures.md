# Procedure encoding and the PCG

This page describes how an ordinary (impure) Rust function or method body is encoded into a Viper method with a CFG body — and, in particular, how Prusti decides where to place `fold`/`unfold`/permission-manipulating statements, which is the hardest part of translating Rust's ownership model into Viper's permission logic.

## From MIR to a Viper CFG

The top-level encoder is [`MethodEnc`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_fn/method.rs). For a local, non-trusted function with a body, it:

1. Fetches the MIR body *together with its NLL/Polonius borrow-checking facts* (`vcx.body_mut().get_impure_fn_body_with_facts`) — available because [`PrustiCompilerCalls::config`](../pipeline/rust.md) arranged for these facts to be retained during the compiler's own borrow-checking pass.
2. Runs the **Place Capability Graph** analysis over the body (see below).
3. Walks the MIR body's basic blocks with an [`ImpureEncVisitor`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_impure.rs), translating each MIR statement/terminator into one or more VIR statements, consulting the PCG's precomputed results at each step to decide which permission-manipulating statements are needed.
4. Assembles the pre/postconditions (from the item's `ProcedureSpecification`, see [Pure function encoding](pure.md)) plus any [magic wands](#pledges-and-magic-wands) needed for mutable-reference pledges, and emits a `vir::Method`.

## The Place Capability Graph (PCG)

The [`pcg`](https://github.com/viperproject/prusti-dev/tree/master/pcg) crate (pulled in as a git submodule, see [Setup](../development/setup.md)) computes, for every program point in a MIR body, which *places* (memory locations/projections, e.g. `x`, `(*x).field`) currently have which *capability* — exclusive/write, read-only, or none — together with a graph of borrows and reborrows connecting places across the CFG. It supersedes the pre-rewrite architecture's separate fold/unfold fixpoint pass (which ran on the *already-encoded* Viper CFG) and its hand-rolled "reborrowing DAG": because the PCG runs directly on MIR with the real NLL/Polonius facts, it knows precisely when a place is blocked by a live loan and when it becomes available again once that loan expires, and it can derive the exact "repack" operations (place expand/collapse) and borrow-graph edges the encoder needs, without a second inference pass.

The analysis is run once per function, as a genuine fixpoint over the whole CFG (using `rustc`'s own dataflow framework), *before* the MIR body is walked for encoding:

```rust
let pcg_ctxt = pcg_creator.new_nll_ctxt(&body_with_facts);
let fpcs_analysis = pcg::run_pcg(pcg_ctxt);
```

(`prusti-encoder/src/encoders/mir_fn/method.rs`). The resulting `PcgOutput` is threaded through `ImpureEncVisitor` and queried per basic block (`fpcs_analysis.get_all_for_bb(block)`, in [`mir_impure.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_impure.rs)); per-statement `RepackOp::Expand`/`RepackOp::Collapse` values read off the analysis are translated directly into the equivalent Viper `unfold`/`fold` (and related predicate-manipulating) statements.

If you are debugging an incorrect permission/fold-unfold encoding, the PCG's own `visualization/` module can dump the computed capability/borrow graphs (dot/JSON) per program point — see [Debugging](../development/debug.md).

## Pledges and magic wands

Mutable references and their "pledges" (`#[after_expiry(...)]`-style postconditions that hold once a borrow expires) are encoded using genuine Viper [magic wands](http://viper.ethz.ch/tutorial/?page=1&section=#magic-wands), rather than the pre-rewrite reborrowing DAG:

> - [`prusti-encoder/src/encoders/impure/fn_wand.rs` - `WandEnc`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/impure/fn_wand.rs) - encodes a function/method's magic-wand obligations for its mutable-reference parameters/return values.
> - [`prusti-encoder/src/encoders/impure/wand.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/impure/wand.rs) - lower-level wand construction helpers.

## Loops

Loop invariants (specified with `body_invariant!` or a loop-head `#[invariant(...)]`) are collected via [`prusti-encoder/src/encoders/mir_fn/spec_blocks.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_fn/spec_blocks.rs) and encoded as a Viper `while` loop's invariant list by [`prusti-encoder/src/encoders/impure/loop.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/impure/loop.rs), using loop-head/back-edge information from `prusti-encoder/src/loops.rs`'s `LoopAnalysis` (and the PCG's own loop analysis, used to determine which places need re-packing at the loop head).

## Calls, traits, and generics

A `Call` terminator to a normal function goes through [`FunctionCallEnc`/`MethodCallEnc`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_fn/mod.rs); a call to a trait method is first resolved (where possible) to a concrete `TraitFnEnc` obligation via `CallTaskDescription::trait_call`, so that trait dispatch is encoded once per trait (its contract) and once per implementation (its obligation to satisfy that contract), rather than duplicated at each call site. See [Type encoding](types.md#def-vs-use-generics-and-casts) for how the underlying generic-parameter substitution and casts work.
