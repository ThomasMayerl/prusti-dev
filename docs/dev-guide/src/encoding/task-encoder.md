# Task encoders

Almost all encoding logic in `prusti-encoder` is expressed as an implementation of the `TaskEncoder` trait, defined in the small, standalone [`task-encoder`](https://github.com/viperproject/prusti-dev/tree/master/task-encoder) crate ([`task-encoder/src/lib.rs`](https://github.com/viperproject/prusti-dev/blob/master/task-encoder/src/lib.rs)). This is the central abstraction of the whole rewrite, so it is worth understanding before reading any specific encoder.

## The problem it solves

Encoding a Rust item (a function, a type, a cast, ...) into Viper is rarely self-contained: encoding a function body needs the Viper representation of the types it uses, which in turn may need the encodings of other types they contain, and so on. The pre-rewrite encoder handled this with a single mutable `Encoder` struct and an explicit work queue. The new architecture instead models each *kind* of encoding task as its own type implementing `TaskEncoder`, with:

- a **task description** (`TaskDescription`) — whatever a caller naturally has on hand to identify what should be encoded (e.g. a `DefId` plus generic arguments);
- a **cache key** (`TaskKey`, defaults to the task description) — used to memoize work, in case multiple descriptions should be normalized to the same encoding;
- an **output reference** (`OutputRef`) — a lightweight, `Clone`-able handle other encoders can hold onto to refer to this task's result (e.g. the Viper name of a predicate) without needing the full encoding to be finished yet;
- a **local output** (`OutputFullLocal`) — the part of the encoding that should appear exactly once in the final Viper program (e.g. the predicate/function/method declaration itself);
- a **dependency output** (`OutputFullDependency`) — the part that every *dependent* encoder needs a copy of (e.g. a struct describing a type's fields, useful to many callers but not itself part of the emitted program).

## Requesting other encoders' output

Inside `do_encode_full`, an encoder is given a `&mut TaskEncoderDependencies` handle, used to pull in the results of other task encoders:

```rust
fn require_ref<EOther: TaskEncoder>(&mut self, task: EOther::TaskDescription<'vir>)
    -> Result<EOther::OutputRef<'vir>, EncodeFullError<'vir, Self>>;
fn require_local<EOther: TaskEncoder>(&mut self, task: EOther::TaskDescription<'vir>)
    -> Result<EOther::OutputFullLocal<'vir>, EncodeFullError<'vir, Self>>;
fn require_dep<EOther: TaskEncoder>(&mut self, task: EOther::TaskDescription<'vir>)
    -> Result<EOther::OutputFullDependency<'vir>, EncodeFullError<'vir, Self>>;
```

(`task-encoder/src/dependencies.rs`). Each call transparently triggers the other encoder if it has not run yet for that task, returns its cached result if it has, detects encoding cycles (`check_cycle`), and — if the dependency's encoding failed — turns that failure into a chained `EncodeFullError::DependencyError` recording every encoder/task in the chain, so the *root cause* (e.g. "unsupported feature X") can still be reported at the point that actually needs it, rather than an opaque "some dependency failed".

## A minimal example

[`PairUseEnc`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/custom/pair.rs) (a small custom encoder providing a generic Viper ADT for pairs) shows the shape of a typical `TaskEncoder` impl:

```rust
pub struct PairUseEnc;

impl TaskEncoder for PairUseEnc {
    task_encoder::encoder_cache!(PairUseEnc);
    const ENCODER_NAME: &'static str = "pair use encoder";
    type TaskDescription<'vir> = Vec<vir::TypeDyn<'vir>>;
    type OutputFullDependency<'vir> = PairUse<'vir>;

    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        task.clone()
    }

    fn do_encode_full<'vir>(
        task_key: &Self::TaskKey<'vir>,
        deps: &mut task_encoder::TaskEncoderDependencies<'vir, Self>,
    ) -> task_encoder::EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(task_key.clone(), ())?;
        let tuple = deps.require_dep::<PairEnc>(task_key.len())?;
        // ... build a `PairUse` from `tuple` and return it
    }
}
```

`task_encoder::encoder_cache!` is a small helper macro that wires up the per-encoder static cache (`with_cache`); most encoders use it rather than implementing caching by hand.

## Driving encoders and collecting output

- `EOther::enqueue(task)` schedules a task without waiting for its result — used by the top-level crate walk ([`encode_all_in_crate`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/encoders/mir_fn/mod.rs)) to seed the process with every function/trait/impl in the crate.
- `EOther::encode_ref(task, span)` / `EOther::encode(task, need_output, span)` are the lower-level entry points that `require_*` build on; they are rarely called directly outside of `test_entrypoint`.
- Once all tasks are enqueued/encoded, `EOther::emit_outputs(&mut program)` (or the provided default, `all_outputs_local_no_errors`) drains every cached `OutputFullLocal` for that encoder into the shared `task_encoder::Program`, and turns any recorded errors into diagnostics (see [`test_entrypoint`](https://github.com/viperproject/prusti-dev/blob/master/prusti-encoder/src/lib.rs), which calls this for every encoder in turn).

Because caching, dependency tracking, and error propagation are handled generically by `task-encoder`, adding support for a new Rust feature usually means writing one focused `TaskEncoder` implementation and wiring a handful of `require_*` calls into the encoders that should use it — not modifying a central dispatch function.
