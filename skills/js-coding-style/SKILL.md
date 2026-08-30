---
name: js-coding-style
description: JavaScript (and TypeScript) programming style and architecture rules. Use whenever writing, modifying, reviewing, or designing JS/TS code.
---

# JavaScript Coding Style

- Prefer simple, explicit code over clever abstractions.
- Favor readability, predictability, and obvious runtime behavior.
- Avoid hidden control flow and unnecessary indirection.
- Prefer clear sequential code; do not split functions merely to make them shorter. Longer functions are fine when they keep related logic and state together.
- Prefer concrete operations and simple data structures over abstraction hierarchies.
- Make concrete code work before generalizing it.
- Generalize only after real usage reveals a common pattern.
- Keep one canonical representation of state when practical.
- Design data around how it is processed.
- Prefer deleting or simplifying code over adding abstractions.

## JS-specific

- Use `const` by default; use `let` only for variables that are reassigned. Never use `var`.
- Prefer plain functions and modules over classes; use classes only when instances truly need encapsulated identity and state.
- Prefer `async`/`await` over raw Promise chains and callbacks.
- Avoid deep destructuring or chaining that hides where values come from.
- Use descriptive, consistent names; avoid abbreviations.
- Keep modules focused; export a small, clear surface rather than re-exporting everything.
- Avoid default exports for anything other than a single obvious module entry point; prefer named exports.
- Handle errors explicitly; do not silently swallow rejected promises or thrown errors.
- Avoid mutating function arguments or shared state; prefer returning new values.
- Prefer plain objects and arrays over introducing new abstractions (e.g., custom collection classes) unless the language's built-ins are clearly insufficient.
- Keep async control flow explicit; avoid unnecessary `Promise.all` fan-out or concurrency when sequential logic is clearer.
- Let the type checker (TypeScript, JSDoc types) catch invalid states rather than adding defensive runtime checks for internal code paths.
