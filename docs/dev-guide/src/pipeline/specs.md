# Specification collection

- Specifications, still present in the HIR as serialized data in dummy attributes, are collected and type-checked into a `DefSpecificationMap`.

Prusti makes use of [procedural macros](https://doc.rust-lang.org/reference/procedural-macros.html) as the interface for user-provided specifications (`#[requires(...)]`, `#[ensures(...)]`, `#[pure]`, `#[invariant(...)]`, `predicate!{...}`, `#[extern_spec]`, ...). These macros live in the nested [`prusti-contracts/`](../layout.md#specification-parsing) Cargo workspace, not in the main workspace.

## Specification syntax

Prusti [specification syntax](https://viperproject.github.io/prusti-dev/user-guide/syntax.html) is a superset of Rust boolean expressions. The parsing of Prusti-specific syntax is achieved with a custom preparser; parsing of ordinary Rust syntax is delegated to the [`syn`](https://crates.io/crates/syn) crate.

 - [`prusti-contracts/prusti-specs/src/specifications/preparser.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-contracts/prusti-specs/src/specifications/preparser.rs)

The preparser accepts the following EBNF grammar:

```ebnf
assertion ::= prusti_expr ;
pledge ::= pledge_lhs, ",", prusti_expr ;
pledge_lhs ::= [ ? actual rust expression ?, "=>" ], prusti_expr ;

prusti_expr ::= conjunction, [ "==>", prusti_expr ] ;
conjunction ::= entailment, { "&&", entailment } ;
entailment ::= primary | ? actual rust expression ?, [ "|=", [ "|", ? args as parsed by syn2 ?, "|" ], "[", [ ( requires | ensures ), { ",", ( requires | ensures ) } ], "]" ] ;
primary ::= "(", prusti_expr, ")"
          | "forall", "(", "|", ? one or more args as parsed by syn2 ?, "|", prusti_expr, [ ",", "triggers", "=", ? array as parsed by syn2 ? ] ")"
          ;
requires ::= "requires", "(", prusti_expr, ")" ;
ensures ::= "ensures", "(", prusti_expr, ")" ;
```

## From proc-macro expansion to typed specs

The proc-macro entry points themselves ([`prusti-contracts/prusti-contracts-proc-macros/src/lib.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-contracts/prusti-contracts-proc-macros/src/lib.rs)) forward, when the `prusti` feature is enabled, into `prusti-specs`' `rewrite_prusti_attributes` ([`prusti-contracts/prusti-specs/src/lib.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-contracts/prusti-specs/src/lib.rs)), which:

1. Parses the Prusti attribute on the item, generating a fresh specification ID per assertion.
2. Emits a **dummy spec function** containing the type-checked assertion body, tagged with `#[prusti::spec_id = "..."]`.
3. Leaves a small marker attribute on the *original* item referencing that ID, e.g. `#[prusti::pre_spec_id_ref = "..."]` for `requires`, `#[prusti::post_spec_id_ref = "..."]` for `ensures`, `#[prusti::pure]`/`#[prusti::pure_spec_id_ref]`, `#[prusti::trusted]`, `#[prusti::type_invariant_spec]` for `invariant`, and so on.

Because procedural macros are executed in a separate, sandboxed process, it is not straightforward to pass data from procedural macros to the rest of the [Prusti pipeline](summary.md). This dummy-function-plus-marker-attribute trick is how the collected specification data survives macro expansion and shows up again, still attached to real HIR nodes, once the driver runs.

On the driver side, [`prusti-interface/src/specs/mod.rs` - `SpecCollector`](https://github.com/viperproject/prusti-dev/blob/master/prusti-interface/src/specs/mod.rs) walks the crate's HIR looking for these markers (using helpers in [`prusti-interface/src/utils.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-interface/src/utils.rs)), accumulating raw spec-ID references per item. `SpecCollector::build_def_specs` then resolves each spec ID to its dummy function, type-checks/interprets its MIR body, and produces the final, **typed** [`DefSpecificationMap`](https://github.com/viperproject/prusti-dev/blob/master/prusti-interface/src/specs/typed.rs) — a map from `DefId` to `ProcedureSpecification`/`LoopSpecification`/`TypeSpecification` (plus standalone assertions/assumptions/refutations) that the rest of Prusti (in particular `prusti-encoder`) consumes.

For specifications on items defined in *other* crates (e.g. the standard library specs shipped in `prusti-contracts`, or specs in an upstream dependency), a second mechanism exports the already-built typed `DefSpecificationMap` to a sidecar file next to the crate's compiled metadata, and imports it again when that crate is used as a dependency — see [`prusti-interface/src/specs/cross_crate.rs`](https://github.com/viperproject/prusti-dev/blob/master/prusti-interface/src/specs/cross_crate.rs) (`CrossCrateSpecs::import_export_cross_crate`, called from `prusti/src/callbacks.rs`).
