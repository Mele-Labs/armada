# Protocol 15.1: a Helm job can be carried to whoever could fix it

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1367`. One new route, `GET /helm/debug`, answering `HelmDebugInfo` — the repository, its
authority and model; the brief as it was sent; the tools the door offered by name; the thread,
bounded, with each turn's cost, its calls and its refusals; what the job's last poll was
answered; and the protocol Fleet speaks beside the Fleet process that answered. Additive: a new
operation and a new DTO family, and nothing an older peer already parses changes.

**Fields and not text**, for `ErrorNotice/payload.ts`'s reason: Bridge formats them, so one
producer writes the artifact and the expanded view renders the same string the clipboard takes. A
Fleet that formatted the record and a Bridge that framed it would be two producers of one artifact.

**`polled` is kept as the door answers a job's `get_events_since`**, not recounted when the
record is taken — a window counted later is not the window a turn read. It is in memory for one run
of Fleet, so it is absent on a job that has not polled since Fleet started, and the record says
so rather than claiming the job never polled.

**The thread is bounded and says what it cut**, the way a log tail does, and a long reply is cut
with its own length beside it. `agent_access` is `No`: a job must not read another job, and
reading its own brief and roster would be reading a record kept about it.
