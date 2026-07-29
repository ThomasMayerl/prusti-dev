# Introduction

This is the developer guide for [Prusti](https://github.com/viperproject/prusti-dev/), intended to make Prusti more approachable for new *contributors*. For installation instructions and a tutorial on using Prusti, see the [user guide](https://viperproject.github.io/prusti-dev/user-guide). Note: This is a temporary version of the dev-guide written by Claude Code for testing purposes. This version is not officially maintained by the Prusti team.

This guide documents the architecture found on the `rewrite-2023` branch, which replaced Prusti's original MIR-to-Viper encoder (`prusti-viper`'s `encoder` module, `prusti-common`'s `vir`) with a new pipeline built around a generic, memoizing [task-encoder](encoding/task-encoder.md) framework, a new intermediate representation ([`vir`](encoding/vir.md)), and a borrow-aware permission analysis ([the PCG](encoding/procedures.md)) that replaces the old fold/unfold inference. Almost everything between "Prusti has type-checked the code" and "here is a Viper program" works differently now; the [Architecture at a Glance](architecture.md) chapter is the best starting point for spotting the differences from the previous architecture.

> Direct links to code are provided in boxes like this. Note that we sometimes link to specific lines in the linked files, which requires that the links refer to a specific commit in Prusti's history. When reading through the code using this guide, make sure to check if the code has since changed.

## How to use this guide

- For a mental map of the whole system and a pointer to "which crate do I touch to change X", start with [Architecture at a Glance](architecture.md) and [Repository Layout](layout.md).
- To understand what happens, in order, when you run `cargo prusti`, read the [Verification Pipeline](pipeline/summary.md) chapter.
- To understand *how* Rust code becomes a Viper program (the most involved part of Prusti), read the [Encoding Architecture](encoding/summary.md) chapter.
- To build Prusti, run its tests, or debug a failing verification, see [Development](development/summary.md).
