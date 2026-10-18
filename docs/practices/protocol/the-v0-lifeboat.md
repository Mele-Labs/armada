# The v0 lifeboat

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

When the ID check refuses, Bridge doesn't get nothing. It gets four routes that
don't depend on the IDs agreeing:

| Operation | Route |
|---|---|
| List Jobs with status | `GET /v0/jobs` |
| Kill a Job | `POST /v0/jobs/:id/kill` |
| Stop Fleet | `POST /v0/stop` |
| Report Fleet's version | `GET /v0/version` |

That's the whole surface. Bridge's recovery screen is built on exactly these
four: show what's running, name both IDs so the human can tell what's
mismatched, and offer per-Job kill so nothing is left burning tokens
unsupervised while someone goes and fixes the mismatch.

**None of it is built yet.** `crates/api/src/routes/served.rs` says the lifeboat
belongs to the Ship milestone, and nothing in Bridge calls `/v0`. Until it lands,
a refused Bridge shows the mismatch notice and nothing else. The paragraphs
below are the contract it will be built to.

The lifeboat's entire value proposition is being the one thing guaranteed to
work when everything else — the ipc types, the codegen, the ID check — has
already failed or gone stale. That guarantee has exactly one precondition:
**the lifeboat itself never needs to change.** Concretely, that means:

- **Hand-written, not derived.** No `ipc` types, no shared serialization
  helper, no codegen. If the machinery that generates the rest of the
  protocol breaks, that must not be able to take the lifeboat down with it.
- **`curl`-testable.** Plain JSON over plain HTTP, no auth handshake beyond
  whatever's already required to reach Fleet at all, no client library
  required to exercise it.
- **No events, no streaming.** A WebSocket is exactly the kind of stateful,
  versioned, buffered thing the lifeboat exists to not depend on.
- **No new dependency, ever.** A second reason gRPC was rejected for the main
  protocol was that it would have put a codegen toolchain underneath the one
  route table that's supposed to have none. Don't reintroduce that by way of
  the lifeboat.

What would break the guarantee: adding a fifth operation because it seemed
convenient, pointing any of the four routes at `ipc` types "just to reuse the
struct," giving the lifeboat its own version number that then itself needs
negotiating, or letting it grow an auth or job concept that the main
protocol also has to keep in sync with. Every one of those is a small,
reasonable-sounding change that turns four static routes into a second thing
that can be stale. If a change to this document's four rows is ever proposed,
that's the signal to slow down, not speed up.
