---
name: zig-coding-style
description: Zig programming style rules. Use when working with Zig code.
---

# Zig Coding Style

* Prefer a procedural style: plain structs and enums holding data, and free functions that operate on them.
* Pass allocators explicitly to functions that allocate; never hide allocation behind a global or an implicit context.
* Prefer arena allocators when allocations naturally share a lifetime.
* Return errors as error unions; use `try` and `catch` explicitly and never discard errors with `catch unreachable` outside of truly unreachable cases.
* Isolate third-party C libraries behind thin Zig wrappers.
* Always wrap third-party dependencies with a thin API adapter layer.
