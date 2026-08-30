---
name: rust-coding-style
description: Rust programming style rules. Use when working with Rust code.
---

# Rust Coding Style

* Prefer a procedural style: plain structs and enums holding data, and free functions that operate on them.
* Use enums and exhaustive `match` instead of trait hierarchies; avoid `dyn Trait` and virtual-dispatch-style designs unless a boundary genuinely requires runtime polymorphism.
* Do not model domains as objects with behavior; data structs plus functions keep call graphs visible and state easy to reason about.
* Use `Option` instead of sentinel values and `Result` for recoverable failures; propagate with `?` and never `unwrap()` or `expect()` in library paths.
* Isolate third-party crates behind thin wrappers when doing so creates a meaningful boundary.
