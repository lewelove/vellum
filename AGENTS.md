# AGENTS.md

## Coding Principles

The goal is to write simple maintainable code anyone can understand on first glance.

- Keep git diffs minimal
- Use self-documenting function and variables names
- If there's a way to implement something using standard tooling, do it instead of reinventing the wheel

Before implementing **anything** stop and ask yourself:

- Is this an anti-pattern?
- Can this logic be expressed in fewer LoC?
- Am i using the robust industry standard way to solve this problem?
- Do i have enough context to know what user really wants from me?

Don't be afraid to ask direct questions if confused. Your job is to write sane stable code, not immediately satisfy the user with quick slop

## Rust Code Style and Formatting

- Keep new files added under 300 LoC

## Clippy Rules to Always Satisfy

When outputting code make sure you always satisfy these clippy rules, so i don't have to paste the compile errors back to you

- `clippy::collapsible-if`
- `clippy::uninlined-format-args`
- `clippy::option_if_let_else`
