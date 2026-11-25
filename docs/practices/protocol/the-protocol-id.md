# The protocol ID

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**There is no version number to bump.** Every build carries a protocol ID, a
16-digit hash of the wire surface, taken at build time. Two branches that each
change the wire change different files, so they never conflict on a number, and
the ID follows whichever files they touched.

**Equal IDs connect. Anything else is refused.** Bridge opens no socket, and the
`/v0` lifeboat is the fallback. There is no order between two IDs: a mismatch
says the sides differ, not which one is older.

## What it hashes

| Source | Why it is wire |
|---|---|
| `crates/ipc/src/**/*.rs`, except `tests/` | The DTOs, and every other type Fleet serializes |
| `crates/ipc/operations/*.toml`, except `_header.toml` | Which operations and event kinds exist |
| `crates/api/src/routes/**/*.rs` | The method and path each operation is served on |

Files are taken in path order. Within one, a line that is only a comment is
dropped and trailing whitespace is trimmed, so editing prose moves nothing. The
bytes then go through FNV-1a with 64 bits. The hash is the same on every
machine and every checkout, because it reads paths relative to the repository
root and never a commit hash.

## What it cannot see

The hash is over source text, not over meaning, so it is wrong in two
directions and both are chosen.

- **It moves on changes that are not wire.** A code line edited in `ipc`
  that changes no JSON, or a trailing `// comment` after code, moves the ID. A
  false mismatch is a refusal and a restart; the other way round is a silent
  hole, which is why it is built to err this way.
- **It does not see a value set that lives elsewhere.** A `SessionRow` kind and
  a Requester's `kind` are strings on the wire whose spellings come from
  `core-model`'s registries and the generated vocabulary. A new spelling there
  moves no ID. That is safe only while Bridge renders such a value instead of
  branching on it. `crates/ipc/src/requester.rs` is where `kind` argues it,
  and a set Bridge matches on belongs in a `wire_enum!`, which is `ipc` source
  and moves the ID.
- **It cannot tell that a copy of the types was updated.** Bridge links
  `crates/ipc`, so it has no copy. Pocket's types are generated from `ipc`
  and checked by `cargo xtask verify-wire`, so they are the one copy the gate
  holds to the Rust.

## Where each side gets it

- **Rust.** `crates/ipc/build.rs` hashes the files on every build that touched
  them and embeds `ProtocolId::current()`. The hashing is
  `crates/ipc/wire_hash.rs`, a file `build.rs` includes.
- **The restart script.** `scripts/protocol-id` takes the same hash of a tree
  before there is a binary to ask, so `scripts/restart` knows the ID of what it
  is about to build. **It is not a checked-in file**: a literal in git is a line
  every branch that touches the wire edits, and two of them conflict, which is
  the problem the ID exists to end.
- **They are two implementations of one hash**, written twice because a build
  script and a restart script cannot share code. `cargo xtask verify-foundations`
  runs both on the working tree and fails when they disagree
  (`xtask/src/rules_protocol/id.rs`), and the Rust half it runs is the file
  `build.rs` includes.

## The ID on the wire

It crosses as one string field, `protocol_id`, on the runtime file and on each
socket's opening message. The runtime file from a Fleet that predates IDs has no
such field and reads as an ID nothing equals, so an old Fleet reaches the
mismatch screen instead of reading as a file nothing wrote.

## Contributor workflow

1. Change the DTOs in `crates/ipc`. If Pocket reads the type, run
   `cargo xtask verify-wire --write` and commit what it writes.
2. Add or change the operation in `crates/ipc/operations/` and the route in
   `crates/api/src/routes/served/`.
3. Nothing else. There is no number to choose, no need to declare and no
   generated file to commit, so a branch that changes the wire never conflicts
   with another on account of it.
4. Run `cargo xtask verify-foundations` and read the line naming the protocol.
   The rest of its output is about other rules.
