<img width="150" height="141" alt="droid-bay" src="https://github.com/user-attachments/assets/c5c24af5-7a9d-4faf-82c8-9eb3e93d9ab5" />

# Droid Bay

This is a repository of agents and skills (for personal use). It's primarily focused on my personal needs and interests:

- Native systems software development with C, Zig, Rust, Go
- Academic research and writing
- YouTube video research, writing and production

## Tools

I don't like tying my solutions and tools to any vendor or software (e.g. Codex or something like Hermes, OpenHands).

Skills are standard and usable by all harnesses - so there's no problem there. I usually use the Pi harness.

Each of my "agents" is custom software for a couple of reasons:

- **Deterministic processing** - I can do everything that can be done programmatically and plug LLM generation as a small part of the process where necessary.
- **Provider-agnostic** - I can extend them to be used with any LLM provider - OpenRouter, Codex or any API.
- **Simple, resource-efficient and easy to reason about** - I do not like bloated large software, especially large vibe-coded apps which can do unexpected things with LLMs. I think you should be able to trace every instruction that gets injected into the LLM's context during generation. 

My agents are Go programs which you can compile and run natively on any system. The great thing is that they are lightweight and efficient - they can be deployed on a server to run remotely as well.

## Contributions

No contributions are expected as this is my personal repository that I'm keeping public just in case someone finds it useful. Still, if you have a suggestion or a fix for something - feel free to open an issue.
