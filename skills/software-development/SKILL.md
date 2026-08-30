---
name: software-development 
description: Use this skill when doing any software development or software engineering - it provides general guidelines. 
---

## General Development Guidelines

* Make the smallest change that completely solves the requested problem.
* Do not add speculative features, configurability, extensibility, or abstractions for hypothetical future requirements.
* Preserve existing conventions unless there is a concrete reason to change them.
* Prefer modifying existing systems over introducing parallel implementations.
* Do not add dependencies unless explicitly asked to do so.
* Avoid magic values; give meaningful constants names when the meaning is not obvious.
* Avoid comments.
* Before finishing a change, build the affected target and fix errors or warnings introduced by the change.
* Do not write unit, integration or any kind of automated code tests unless explicitly asked to do so.
* Do not rewrite working code merely to match personal preferences.
* Do not silently change behavior outside the requested scope.
* When several solutions are viable, prefer the simplest implementation with the fewest moving parts.
* When unsure about an existing system, inspect its implementation and usages before modifying it.
* Prefer simple, explicit code over clever abstractions.
* Optimize for readability, predictability, and understanding what the program will do.
* Avoid hidden control flow, surprising implicit behavior, and unnecessary indirection.
* Use assertions for violated internal invariants; use explicit error handling for expected runtime failures or invalid external input.
* Prefer clear sequential code; do not split functions merely to make them shorter.
* Keep call hierarchies shallow and avoid unnecessary abstraction layers.
* Prefer concrete operations and straightforward data structures over elaborate abstraction hierarchies.
* Make the concrete solution work end-to-end before optimizing or generalizing it.
* Generalize only when real usage reveals a useful common pattern.
* Keep useful low-level primitives available beneath higher-level convenience APIs.
* Prefer APIs with little setup, bookkeeping, hidden state, or teardown.
* Keep one canonical representation of state and compute cheap derived values when practical.
* Design data structures around how the program processes the data.
* Prefer deleting or simplifying code over adding new abstractions.
* Do not build generalized systems when a small specialized solution clearly solves the actual problem.

## Style

- When working with C code, use the `c-coding-style` skill.
- When working with C++ code, use the `cpp-coding-style` skill.
- When working with Rust code, use the `rust-coding-style` skill.
- When working with Zig code, use the `zig-coding-style` skill.
- When working with JavaScript or TypeScript use the `js-coding-style` skill
