# crates/

The Rust workspace. Read `docs/practices/rust.md` before writing here — it holds
the crate boundaries, the type-system-first pattern, and the reasons.

- No `serde_json::from_*` outside `store` and `ipc`.
- No vendor literal outside `adapters`.
- `armada check test <name> [<name>…]` (every name in one call, one build) or `armada check test --changed` (paths on stdin), never `cargo test` or `nextest` bare.

All of these are enforced by a hook before the write and by the gate after it.

Run an `armada check` with `run_in_background` and end the turn. In the
foreground it holds the job for minutes, and the owner cannot reach it.
