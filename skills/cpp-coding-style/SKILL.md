---
name: cpp-coding-style
description: C++ programming style rules. Use when working with C++ code.
---

# C++ Coding Style

* Prefer C-style C++: treat the language as C with selected conveniences, not idiomatic modern C++.
* Prefer a procedural style: plain structs holding data, and free functions for operations on them.
* Avoid classes with behavior, inheritance hierarchies, and operator overloading unless a boundary genuinely requires them.
* Keep allocation and deallocation visible at the call site; do not hide memory management behind constructors, destructors, or smart-pointer-returning factories.
* Pass memory explicitly: prefer arenas for allocations that share a lifetime, and use `unique_ptr` as the default owner only for objects that outlive any shared lifetime.
* Do not write templates or generic code; use concrete types and a small fixed set of standard containers.
* Do not use exceptions or iostreams; have functions that can fail return a project-defined `Result` type distinguishing `Ok` from `Error`, with `Ok = 0`, and return payloads through out-parameters.
* Isolate third-party libraries behind thin wrappers when doing so creates a meaningful boundary.
