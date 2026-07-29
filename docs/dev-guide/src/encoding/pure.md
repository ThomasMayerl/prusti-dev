# Pure function encoding

To encode specifications and side-effect-free functions (marked `#[pure]`), Prusti generates Viper [functions](http://viper.ethz.ch/tutorial/?page=1&section=#functions) or, for specification clauses themselves, plain Viper boolean expressions. A Viper function consists of a single expression, so a Rust function body's control flow must be folded into one nested expression — this is only possible for loop-free bodies; the encoder rejects loops in pure code ("MIR pure encoding does not support loops", in [`prusti-encoder/src/encoders/mir_pure.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_pure.rs)).

## From a MIR CFG to a single expression

[`MirPureEnc`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_pure.rs) walks the body's control-flow graph from the return block backwards (`encode_body`/`encode_cfg`): a `SwitchInt` terminator (the MIR form of `if`/`match`) becomes a Viper ternary expression (`vcx.mk_ternary_expr`) combining the encodings of its branches, and this recurses until the whole CFG has been folded into one expression. Conceptually, a Rust function like

```rust
#[pure]
fn abs_diff(a: i32, b: i32) -> i32 {
    if a > b { a - b } else { b - a }
}
```

becomes a single Viper function body built from nested ternaries over the encodings of `a - b` and `b - a`, guarded by the encoding of `a > b`.

`MirPureEnc` is reused for several different "flavours" of pure encoding, distinguished by `PureKind` (`prusti-encoder/src/encoders/mir_pure.rs`): a genuine `#[pure]` function, a specification closure, a promoted constant, or a "spec block" (an assert/assume embedded inline in an otherwise-impure body). Basic arithmetic/comparison operations and casts are delegated to the shared [`builtin/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-encoder/src/encoders/builtin) encoders (`MirBuiltinBinOpEnc`, `MirBuiltinUnOpEnc`, `MirBuiltinUseCastEnc`), which are also used from the impure encoder — see [Procedure encoding and the PCG](procedures.md).

## Encoding specifications as reusable expression templates

Pre/postconditions, loop invariants, and pledges are encoded by [`pure/spec.rs` - `MirSpecEnc`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/pure/spec.rs), which is built directly on top of `MirPureEnc`. A specification's Rust closure body refers to the annotated function's arguments and (for postconditions) its result and `old(...)` values — none of which are concrete Viper expressions yet at the point the specification is encoded (which happens once per `DefId`, not once per call site). This is precisely the situation the VIR [`Gen`/`Reify` machinery](vir.md#generic-gen-nodes-and-reification) exists for: `MirSpecEnc` produces an expression generic over an `ExprInput` (a map from each MIR local to its eventual snapshot expression), e.g. `vir::ExprGenBool<'vir, ExprInput<'vir>, _>`, which every call site then `reify`s with the actual argument/result expressions substituted in.

## Where the result is used

- A genuine `#[pure]` function's encoding becomes the body of an actual Viper `function` declaration, emitted by `FunctionCallEnc`/`MethodCallEnc` (see [Encoding stage](../pipeline/encoding.md)) so it can be called both from other pure code and from specifications.
- A specification's encoding is *inlined* as a `requires`/`ensures`/invariant clause wherever it's needed — in the impure encoder's method contract ([Procedure encoding and the PCG](procedures.md)), and, for pure functions, also as their Viper function's own contract.
- Whether a given `DefId` should be treated as pure or trusted is looked up via [`spec.rs` - `is_function_trusted`/`with_proc_spec`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/spec.rs), which reads the `ProcedureSpecification` produced during [specification collection](../pipeline/specs.md).
