---
name: go-coding-style
description: Go programming style rules. Use when working with Go code.
---

# Go Coding Style

* Prefer a procedural style: plain structs holding data, and package-level functions that operate on them.
* Avoid interface hierarchies and polymorphism;
* Do not model domains as objects with behavior; data structs plus functions keep call graphs visible and state easy to reason about.
* Keep goroutines and channels explicit and locally owned; avoid hidden concurrency behind library calls, and prefer sequential code when order is clear.
* Prefer concrete types and standard library containers; avoid generics.
* Isolate third-party packages behind thin wrappers when doing so creates a meaningful boundary.
