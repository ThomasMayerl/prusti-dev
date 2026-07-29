# Type encoding

Like the pre-rewrite encoder, Prusti encodes most Rust types into Viper *twice*, in two different flavours, because Rust functions can be used both as ordinary (impure) procedures and, if marked `#[pure]`, as heap-independent Viper functions usable inside specifications:

- an **impure** ("heap-based") encoding, using Viper [predicates](http://viper.ethz.ch/tutorial/?page=1&section=#predicates) over `Ref`s — used for local variables and procedure state, where permissions need to be tracked;
- a **pure** ("snapshot") encoding, using Viper [domains](http://viper.ethz.ch/tutorial/?page=1&section=#domains) and native Viper [ADTs](http://viper.ethz.ch/tutorial/?page=1&section=#adts) — a heap-independent value type usable inside Viper functions and quantifiers, where predicates cannot appear.

Where the pre-rewrite encoder built the pure encoding's domains by hand, writing out constructor functions and injectivity axioms itself, the new encoder makes use of Viper's native ADT support directly (declared constructors/destructors/discriminators, with Viper generating the required axioms), which is both less encoding code to maintain and lets Silicon reason about them more efficiently.

This logic lives under [`prusti-encoder/src/encoders/ty/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-encoder/src/encoders/ty).

## Classifying a Rust type

Before either encoding runs, [`ty/rust_ty.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/ty/rust_ty.rs) decomposes a Rust `ty::Ty` (together with its generic arguments in scope) into a `RustTyDecomposition`/`RustTyData` — a shape shared by both encodings: is it a primitive, a struct-like type (struct or single-variant enum), an enum-like type (multiple variants), an array/slice, an immutable or mutable reference, a raw pointer, a type parameter, or an opaque/unsupported type? The two Viper encodings (`ty::pure`, `ty::impure`) are then generic over this shape.

## `Purity` and the two `TyDatas`

`ty::mod::ViperTyDatas<P: Purity>` (where `Purity` is either the `Pure` or `Impure` marker type from `prusti-encoder/src/encoders/mod.rs`) implements a `TyDatas` trait with one associated type per shape (`StructData`, `EnumData`, `MutRefData`, `ArrayData`, `PrimitiveData`, ...). This is instantiated as:

- [`ty/pure.rs` - `PureTyDatas`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/ty/pure.rs) - every shape carries real content: domain/ADT names, constructor/destructor function identifiers, and so on, because the pure encoding needs to actually build the domain/ADT declarations.
- [`ty/impure.rs` - `ImpureTyDatas`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/ty/impure.rs) - several shapes carry no data at all (`type StructData = ()`, `type BuiltinData = ()`, ...), because all the impure encoding needs for those is a predicate name; only shapes with heap-relevant structure (arrays, mutable references, enum discriminants/variants) carry extra data.

The per-shape encoding logic itself lives in [`ty/kinds/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-encoder/src/encoders/ty/kinds) (`structlike.rs`, `enumlike.rs`, `arraylike.rs`, `mutref.rs`, `immref.rs`, `param.rs`, `opaque.rs`, `raw.rs`, `primitive.rs`, `builtin.rs`), with each function generic enough to be called from both `TyPureEnc` and `TyImpureEnc`. For instance, [`kinds/structlike.rs` - `ty_pure`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/ty/kinds/structlike.rs) encodes each field's type via `TyUsePureEnc`, then calls an `AdtBuilder::constructor` helper to register the corresponding native Viper ADT constructor/destructors.

## "Def" vs "use": generics and casts

Encoding a type's *shape* (`TyPureEnc`/`TyImpureEnc`, keyed by the Rust item's `DefId`) is separate from encoding a *specific instantiation* of it with concrete generic arguments ([`ty/use_pure.rs` - `TyUsePureEnc`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/ty/use_pure.rs), [`ty/use_impure.rs` - `TyUseImpureEnc`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/ty/use_impure.rs)). The "use" encoders are responsible for generic-parameter substitution and for generating the cast functions/methods needed when a generic field's snapshot/predicate type must be converted between its type-parameter form and the concrete argument's type ([`ty/generics/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-encoder/src/encoders/ty/generics), notably `casters.rs`/`use_casters.rs`). This split is also where trait dispatch is encoded: [`ty/generics/trait.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/ty/generics/trait.rs), `trait_fn.rs`, and `trait_impls.rs` encode a trait's contract once and each implementation's obligation to satisfy it.

## Other type-related encoders

- [`ty/lifted/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-encoder/src/encoders/ty/lifted) - `TyConstructorEnc`/`TypeOfEnc` reify Rust *types themselves* as Viper domain values (needed wherever generic/dynamically-dispatched code has to refer to "the type of this value" inside the encoding).
- [`ty/interpretation/`](https://github.com/viperproject/prusti-dev/tree/master/prusti-encoder/src/encoders/ty/interpretation) - encodings that map onto Viper/SMT backend-native types instead of a domain: `bitvec.rs` (used when [`ENCODE_BITVECTORS`](../config/flags.md#encode_bitvectors) is set) and `float.rs` (floating-point types via Viper's native float support).
- [`ty/viper_tuple.rs` - `ViperTupleEnc`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/ty/viper_tuple.rs) - a small helper encoding for grouping several Viper values as one (e.g. Rust tuples, or bundling multiple return/argument values).
- [`ty/indirect.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/ty/indirect.rs) - handling for indirection (e.g. `Box`-like heap allocation).

## Name mangling

Viper identifiers are more restrictive than Rust identifiers (e.g. no `<`, `>`, spaces, or `:`), and Prusti also needs distinct Viper names per monomorphized instantiation of a generic item. [`vir::ViperIdent::sanitize`](https://github.com/viperproject/prusti-dev/blob/master/vir/src/viper_ident.rs) replaces disallowed characters with fixed textual substitutes (e.g. `<` becomes `$lt$`) and validates the result, and is used whenever an encoder derives a Viper name from a Rust path or type.
