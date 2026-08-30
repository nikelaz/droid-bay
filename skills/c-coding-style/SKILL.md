---
name: c-coding-style
description: C programming style rules. Use when working with C code.
---

# C Coding Style

* Use descriptive, consistent names and hierarchical API names such as `module_object_operation()`.
* Use symmetric operation names such as `create/destroy`, `open/close`, and `begin/end`.
* Use macros sparingly; do not use them to disguise ordinary syntax or control flow.
* Prefer contiguous, cache-friendly data layouts and avoid pointer-heavy structures without a concrete reason.
* Be conscious of struct layout, alignment, padding, and frequently accessed data.
* Minimize unnecessary allocations.
* Have functions that can fail return a project-defined `Result` type distinguishing `Ok` from `Error`, with `Ok = 0`; return payloads through out-parameters.
* Organize dynamic memory by lifetime and prefer arenas when allocations naturally share a lifetime.
* When the project uses arenas, pass `Arena *` to APIs that produce arena-owned results so the caller controls their lifetime.
* When the project uses arenas, use scratch arenas or arena checkpoints for temporary allocations and reclaim memory in lifetime-sized batches.
* Separate platform-specific code from platform-independent application code, keeping the platform layer small.
* Separate permanent, transient, and temporary state when those lifetimes naturally exist.
* Design structures so zero-initialization represents a valid default state whenever practical.
* Isolate third-party or platform APIs when doing so creates a meaningful boundary.
