# The run sockets: one run, one server

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`GET /sessions/run/:run_id/observe`, `GET /sessions/checks/:run/observe`,
`GET /manifest/runs/:run_id/observe`, `GET /merge_lines/checks/observe` and
`GET /servers/:server_id/observe` are the one query shape in `operations/` whose
transport is the socket, and `crates/api/src/watching_run.rs` relays them. Each
answers with what a log already holds, then the lines the subject prints next,
then `closed`. **A subject that has ended opens too**, with its whole log, which
is how one that fell over is read.

**Who reads them.** A person, through Bridge, on the machine Fleet is running on,
looking at that run or server. `agent_access` is `No`.

**Per subject, not per owner.** The sheet holds the run's id from the start's
answer, a run has an end to close on, and an owner has one run out at a time. A
per-owner channel would need a hand-over between runs, for a reader nobody has.

| What happened | What the viewer is told |
| --- | --- |
| The subject is going | `opened` with `live: true`, the log so far, then lines, then `closed` / `finished` |
| The subject has ended | `opened` with `live: false`, the log, `closed` / `finished` |
| The log will not read | `opened`, then `closed` / `unreadable` |
| The id names no subject of this owner | **422 before the upgrade**, through the error contract |

**Back-pressure.** The channel is per subject and drop-oldest, and a viewer that
falls behind is sent `missed` with the count and answers it by reopening the
socket, so the backfill redraws the pane whole. The log keeps every line and the
matching `get_*_output` reads it, so a drop costs a live line and never the
record. Each live line carries the byte offset just past it, which is what keeps
a line the opening read sent from being sent again. Watching a subject cannot
change its outcome: the send is synchronous and never blocks, and what a slow
viewer slows is its own socket task.

**It is deliberately not `/events`.** Output is the fastest thing a run or a
server makes, and on the global stream it would evict the facts Bridge draws the
Cockpit from — an eviction there is not a lost row but a `Missed` and a full
resync, paid continuously. `/events` carries a run's end (`checkout_run.finished`)
and a server's three lifecycle facts — `server.starting`, `server.serving`,
`server.exited` — each with the whole instance, and never a line of output. **A
row is replaced rather than patched**, which is why a server that goes behind its
checkout is published again under the kind its phase is rather than under a
fourth kind.
