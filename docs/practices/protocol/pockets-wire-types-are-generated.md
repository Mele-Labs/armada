# Pocket's wire types are generated

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Pocket's wire types in `apps/pocket/src/wire/` are generated from the Rust types in `crates/ipc/src`.
Edit nothing there.
Pocket is the phone app and the one TypeScript client, so it reads the Rust types directly.
Classic, the Electron Bridge, kept a hand-written copy in `classic-final:packages/protocol`, which is deleted with it.

**Regenerate with `cargo xtask verify-wire --write`, and commit what it writes.**
Without `--write` the same command fails on a stale file and names it.
`cargo xtask verify-foundations` runs the same check as *Pocket's wire types are generated from crates/ipc*, and CI's generated-files step runs `--write` and fails the job when it changed anything.
A hand edit is as stale as an old file, because both mean the copy no longer says what `crates/ipc` says.

## What is written

Only the closure of `ROOTS` in `xtask/src/wire.rs`: the types `apps/pocket/src` imports by name, and every type those name.
A type Pocket starts reading is added to that list, and `tsc` says when one is missing.
One file is written per Rust source file, in source order, and `index.ts` re-exports them all.

## Why the generator reads source

`xtask` has no dependencies and the gate must run on a checkout with nothing built, so the generator reads the `.rs` text the way the other `xtask` rules do.
`ts-rs`, `specta` and `schemars` each put a derive or an attribute on every type in the closure, in files other Jobs are editing, and each is a dependency of `ipc`.
Reading the source touches nothing in `crates/ipc`.

**What it does not understand it refuses, by name.**
A type with no `Serialize` derive, a `rename_all` rule it has no reading of, and a type nothing defines each stop the run with the type's name.
A type whose serde impl is written by hand is listed in `WRITTEN_BY_HAND` in `xtask/src/wire/emit.rs`, with what the impl puts on the wire.

## What the TypeScript says

It says what serde puts on the wire, so Pocket reads and sends the same type.

| Rust | TypeScript |
|---|---|
| a field with `skip_serializing_if` | `name?: T`, with no `null` where the test is `is_none` |
| a field with `default` and nothing else | `name: T`, because Fleet still sends the key |
| an `Option` with no `skip_serializing_if` | `name: T \| null` |
| a `wire_enum!` or `wire_id!` type | `string`, since the spellings are the registry's |
| any other `enum` | the literal union its serde attributes spell |
| `#[serde(flatten)]` | an intersection |
| `serde_json::Value` | `unknown` |
| a map | `Record<string, V>` |

**A bool that `skip_serializing_if` leaves out when false is `boolean`, not `true`.**
The same type is a request body in places, and `false` is legal there.

A message between a sealed page and its host is `PageMessage<PageToHost>` or `PageMessage<HostToPage>`, as Rust has it.
The `armada: 1` envelope is part of the first, and `classic-final:packages/protocol` folds it into `PageToHost`.
