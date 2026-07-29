# Encoding Architecture

This chapter details the architecture of the [`prusti-encoder`](https://github.com/viperproject/prusti-dev/tree/master/prusti-encoder) crate: how it is organized, the abstractions it is built on, and how particular features of the Rust language end up encoded into Viper. This is the largest and most frequently-touched part of Prusti, so understanding its shape pays off quickly.

 - [The VIR intermediate representation](vir.md) - the data structures everything in this chapter ultimately produces.
 - [Task encoders](task-encoder.md) - the generic, memoizing abstraction (`TaskEncoder`) that all encoding logic is built from.
 - [Type encoding](types.md) - how a Rust type becomes Viper predicates, domains, and native ADTs.
 - [Procedure encoding and the PCG](procedures.md) - how a function/method body becomes a Viper CFG, using the Place Capability Graph for permissions.
 - [Pure function encoding](pure.md) - how pure Rust expressions (used both as functions and inside specifications) are encoded as Viper expressions.

All of this logic lives under [`prusti-encoder/src/encoders/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-encoder/src/encoders), organized roughly by concern:

```text
prusti-encoder/src/encoders/
├── mir_fn/         function/method-level encoding: signatures, MethodEnc, FunctionCallEnc,
│                   MethodCallEnc, spec blocks (loop invariants, assertions embedded in bodies)
├── mir_impure.rs   walks a MIR body's statements/terminators into VIR CFG statements
├── mir_pure.rs     encodes a MIR body as a single Viper *expression* (pure functions, specs)
├── mir_shared.rs   logic shared between the pure and impure MIR walkers
├── impure/         loop encoding, magic wands (pledges/reborrows)
├── pure/           encoding of specifications (pre/postconditions, invariants) as Viper exprs
├── spec.rs         lookup of a DefId's ProcedureSpecification (trusted/pure/pre/post/...)
├── ty/             type encoding: predicates, domains/ADTs, casts, generics, traits (see types.md)
├── builtin/        encodings of MIR built-in operations: casts, unary/binary ops, intrinsics
├── const.rs        constant encoding
├── custom/         small hand-written support types (e.g. a generic `Pair` ADT)
├── addr.rs         encoding of raw address-of/reference-identity values
└── local_def.rs    per-local-variable encoding info shared across a function's encoders
```

`prusti-encoder/src/lib.rs` (`test_entrypoint`) is the single entry point tying this all together; see [Encoding stage](../pipeline/encoding.md) in the pipeline chapter for how it is invoked and how its output flows onward to Viper.
