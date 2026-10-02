# Protocol: the Fleet/Bridge seam

Armada has exactly one place where Rust stops and TypeScript starts: the wire
between Fleet (the daemon, Rust) and Bridge (the desktop app, Electron). Every
other boundary in the system is a function call or a file. This one is a
process boundary between two binaries with independent release cadence and
independent lifetimes, and everything in this document exists because that
combination has already gone wrong once, in v1, and cost real debugging time
figuring out which side was lying.

If your change touches `protocol-version.toml`, the (forthcoming) `ipc` crate,
anything under `apps/desktop/src/preload`, or the WebSocket event stream, read
this first.

## The single source of truth

`protocol-version.toml`, at the repo root, holds a major and a minor:

```toml
major = 4
minor = 0
```

**Which of the two moves decides what a mismatch does**, and the table further
down is what the code implements. `major` moves when a message an older peer
already parses stops parsing the same way. `minor` moves when the change is
additive only, and resets to zero whenever `major` moves.

The pair crosses the wire as **one field carrying both numbers** —
`"protocol_version": {"major": 4, "minor": 0}` — rather than as two fields. Two
would let either side compare the majors and forget the minors, which is the
defect this shape replaced: the version was one integer, `connection.ts`
compared it with `!==`, and every bump was a full refusal. A bare integer is
still read, as that major at minor zero, because version 4 shipped as one and a
Fleet from before the pair should reach the skew screen rather than read as a
runtime file nothing wrote.

That file is read on both sides, but not the same way:

- **Rust** reads it at compile time. `crates/ipc/build.rs` parses it and emits
  the two numbers, which `crates/ipc/src/version.rs` assembles into the
  `PROTOCOL_VERSION` constant the rest of the Rust workspace compiles against.
  This half is self-correcting by construction — `build.rs` runs on every
  `cargo build`, so the embedded constant cannot go stale relative to the file.
  There is no step to forget here.
- **TypeScript** cannot read a `build.rs`. The plan is a codegen step, driven
  off the same `ipc` crate that defines the DTOs, that emits the matching
  TypeScript types and the version number into `packages/` (see
  `packages/README.md`: "the generated IPC types" is named as the reason that
  directory exists). **Both generated outputs — the Rust constant's TS mirror
  and the DTO types — are checked into the repo, not generated at build time
  on the TS side.** A generated file that's `.gitignore`d looks fine locally
  and is wrong on every machine that didn't just run codegen.

Because the TypeScript half is generated-then-committed, it can drift from its
source the same way any generated-then-committed file can: someone edits the
`ipc` crate and doesn't rerun codegen, or edits the generated `.ts` file by
hand because it was faster. That drift has shipped once. The major moved to 6,
the constant stayed at 5.7, and a Fleet and a Bridge built from the same commit
refused each other into the lifeboat with every check green.

**`cargo xtask verify-foundations` holds the two numbers together now.** It
reads `protocol-version.toml` and the generated constant, refuses a pair that
disagrees, and names both files, both versions and the command that writes the
file. `xtask/src/rules_protocol/version.rs` is the rule.

**It refuses; it does not rewrite.** A gate that ran the codegen itself would
leave nobody knowing the step exists, which is the same defect one release
later — and the next registry to grow a generated half would ship it again.

Two things about the generated half are still checked by nothing, and
`[verify-protocol-task]` below is where they are named:

1. The checked-in generated TypeScript matches what codegen would produce from
   the current `ipc` source, right now.
2. Nothing outside the generated file hard-codes the protocol version as a
   literal.

That second check would answer a violation this document was written
against: `apps/desktop/src/preload/index.ts` returned a hand-typed `1`, with no
mechanism forcing it to move when the source file did. It reads the generated
constant now, and nothing in Bridge restates either number — but the check is
what keeps it that way, because the literal is a one-line shortcut that looks
harmless in review.

**Contributor workflow, in order:**

1. Change the DTOs in `crates/ipc` (add a field, add a variant, whatever the
   change is).
2. Decide which number moves, from the table below, and move it in
   `protocol-version.toml`. Additive-only moves `minor`; anything else moves
   `major` and resets `minor` to zero. **The table is the decision, not a
   guideline** — a minor bump that removes or retypes a field makes Bridge's
   banner a lie and breaks it while a Job runs.
3. Regenerate the TypeScript with `pnpm --filter @armada/desktop codegen`. It
   needs `pnpm install` to have run and nothing else; it rewrites
   `packages/protocol/src/generated/` and `packages/components/src/generated/`
   from `protocol-version.toml` and
   `crates/core-model/domain/`, and prints one line per generated file plus any
   registry row it could not render. **It emits the version mirror and the
   enum vocabulary, not the DTO types** — nothing generates those from the
   `ipc` source yet, so a shape change is still hand-mirrored on the TS side
   and that is the gap `[protocol-codegen]` names.
4. **Run `cargo xtask verify-foundations`.** It refuses a generated constant
   that disagrees with `protocol-version.toml` and names both versions, which
   is the whole of step 3 for the version. It needs nothing built, and it
   reports on rules that have nothing to do with the protocol, so read the line
   naming the version rather than the exit code. That nothing outside the
   generated file hard-codes the version is still verified by reading —
   `[verify-protocol-task]` below.
5. Commit the `ipc` source change and the regenerated files in the same
   change. A generated-file diff with no corresponding source diff, or vice
   versa, is the thing review should bounce.

## DTOs, not domain types

`WireError` is a DTO like any other, and `docs/contracts/error-contract.md` is
what specifies it —
which fields are guaranteed, why `level` and `component` are not among them,
and why removing an error code is a minor bump. The v0 lifeboat below is
deliberately outside that contract.

`ipc` speaks its own vocabulary. It does not re-export `core_model::Job` and
put it on the wire. The conversion is explicit, one direction, and lives at
the Fleet boundary:

```rust
// crates/ipc — the DTO. Only what Bridge is allowed to see.
pub struct JobSummary {
    pub id: JobId,
    pub status: JobStatus,
    pub drone_label: String,
    pub started_at: DateTime<Utc>,
    // no working directory, no adapter credentials, no raw transcript path
}

// Fleet — the conversion, and the only place it happens.
impl From<&core_model::Job> for ipc::JobSummary {
    fn from(job: &core_model::Job) -> Self {
        ipc::JobSummary {
            id: job.id,
            status: job.status,
            drone_label: job.label.clone(),
            started_at: job.started_at,
        }
    }
}
```

The reason this conversion has to exist, rather than serializing
`core_model::Job` directly, isn't code cleanliness — it's that `From` is where
someone has to decide, field by field, what a Bridge (running on someone's
laptop, potentially screen-shared, potentially logged) is allowed to see.
`core_model::Job` will accrete fields as Fleet's needs grow: filesystem paths,
adapter tokens, internal retry state. If that type is `#[derive(Serialize)]`
and put straight on the wire, every new field is redacted or not by accident —
whichever `serde` does by default. `From<core_model::Job> for
ipc::JobSummary` forces a human to look at the new field and write a line of
code, one way or the other. A domain type on the wire is a redaction decision
nobody made.

This cuts the other way too: `ipc` types have no business back in
`core-model`. If Fleet-side code needs a `JobSummary` to build a response,
that's an argument for a thin builder function, not for teaching `core-model`
about the wire's shape.

## Minor vs. major

A minor bump means: **every message an older peer already knows how to parse
still parses the same way.** That is the entire mechanism behind Bridge
running against a newer Fleet with nothing worse than a banner — Bridge parses
fields it recognizes and ignores fields it doesn't, so an additive change is
invisible to it. The moment a bump changes the meaning or presence of a field
an old client already reads, "ignore what you don't recognize" stops being a
safe strategy, and that's a major bump — the lifeboat, not a banner.

| Change | Minor or major | Why |
|---|---|---|
| Add a new DTO / new route / new event type | Minor | Old peer never looks for it, never sees it |
| Add an optional field to an existing DTO | Minor | Old peer ignores unknown fields; new peer treats absence as valid |
| Add a new enum variant, where the enum is only ever *written* by this side and *read as opaque* by the other | Minor, with a caveat — see below | Depends entirely on how the other side matches |
| Add a new enum variant the other side is expected to `match` on | **Major** | An exhaustive `match` on the old side has no arm for it — compile error in Rust, silent `undefined` branch in TS |
| Make a required field optional | **Major** | Anything already relying on its presence (including old Bridge's own type assumptions) now sees a value that used to be guaranteed |
| Make an optional field required | **Major** | Old messages that omitted it become invalid under the new contract |
| Rename a field or a variant | **Major** | Identical to removing the old name and adding a new one — the old name silently stops arriving |
| Change a field's type (including widening, e.g. `u32` → `u64`) | **Major** | "Widening" is a Rust-only intuition; on the wire it's a different JSON shape and a different TS type, and the old side's deserializer doesn't know it's "compatible" |
| Remove anything | **Major** | The obvious case, included for completeness |

**An optional field is left out when it is empty, never sent as `null`.**
Bridge types it `field?: T` and compares against `undefined`, which `null`
passes. Every `Option` on a DTO Fleet writes carries `skip_serializing_if`, or
`deserialize_with = "stated"` where `null` is a value a person chose.
`xtask/src/rules_protocol/nulls.rs` is the rule.

The three people get wrong most often: widening an enum "because it's just
adding cases," making a field `Option<T>` "to be safe," and renaming a variant
"for clarity." All three feel non-breaking from inside the change and are not.
If you catch yourself writing "this shouldn't break anything, it's just
adding/loosening X" — that sentence is the tell. Stop and check whether the
other side's code has an exhaustive match, a presence assumption, or a name
lookup anywhere near the thing you're touching.

**The caveat row has exactly two instances, and both are deliberate.**
`FleetCapacity.held_by` — which one of the concurrency bound, memory or disk
is stopping the next Drone — is a `String` on the wire rather than a
`wire_enum!`, and `crates/ipc/src/capacity.rs` is where that is argued. Fleet is
the only writer, Bridge looks the value up in the generated vocabulary rather
than matching on it, and that map already answers `undefined` for a key it does
not hold. So a fifth reason is a `core-model` variant, a row in
`enum-verbs.toml` and a codegen run, and it moves neither number here.

**`JobSummary.queued_reason` is the second**, since `frozen` joined it. Bridge
types it `string` and reads it through the same generated vocabulary, and the
only Rust readers of a `JobSummary` are this repository's own tests, built at
the same version — so a new reason is minor while nothing branches on it.

**The condition is what makes it minor, not the type.** The moment something on
either side branches on this value rather than rendering it, the row above it
applies instead and widening the set is a major bump. Every other closed set on
this seam is the strict kind and refuses a spelling the registry does not have,
which is right for `JobStatus` — Bridge picks a screen from it.

**And the test is where the set's growth comes from, not how it is read.**
`JobSummary.resumption` — which act a person took to put a `queued` Job back —
is rendered exactly as opaquely as `held_by` and is still a strict
`wire_enum!`, because its three values are the shapes the inner step machine
can be in. A fourth would mean that machine grew a state, which is a change
every reader has to be told about. `held_by`'s set grows every time Fleet
learns to read another resource, which is a change no reader needs to be told
about at all. Ask which of those two a new set is before making it open.

## What Bridge does with the version it reads

Bridge is the side that decides. It reads Fleet's version out of the runtime
file **before it opens a socket**, so a refusal is a screen naming both versions
rather than a malformed first message, and it checks the same fact again on the
resync — a client that reached the socket without reading the file has had no
check at all, and a Fleet restarted under a live socket is not the Fleet the
file described.

Four readings, and only the first two connect.

| Reading | What is true | What Bridge does |
|---|---|---|
| Same | The majors and the minors agree | Connects. The Fleet panel says nothing about versions |
| Fleet ahead | Same major, Fleet's minor is higher | **Connects, and carries a banner.** Everything drawn is current; Fleet has additions this Bridge cannot ask for |
| Fleet behind | Same major, Fleet's minor is lower | **Refuses.** The screen names both versions and says to restart Fleet when no Job is running |
| Incompatible | The majors differ, either way round | **Refuses.** The screen names both versions and says to update both to the same commit. This is what the v0 lifeboat is for |

**The middle two rows are the same gap in opposite directions and they are not
the same situation.** Additive-only says the newer side's additions are things
the older side never asks for and never reads. A newer *writer* is therefore
safe: Fleet sends a field, Bridge ignores it, and nothing Bridge draws is
wrong. A newer *reader* is not: Bridge reads a field an older Fleet was built
before sending, and additive-only promises nothing about that. The hole would
arrive mid-Job rather than at startup, on a Job Board that gives no sign it is
missing anything — which is worse than not connecting.

The banner therefore says the connection is fine and names what it cannot
reach. It goes in the Fleet panel beside the running dot, as advice on a
healthy connection, and **not** as a failure notice: a minor gap Bridge can
survive is not a fault, and drawing it as one tells somebody something is broken when it is
working. `packages/shell/src/fleet.ts` carries the sentences and
`packages/protocol/src/version.ts` carries the rule; `crates/ipc/src/version.rs`
is the same rule in Rust, where the four readings are tested.

**The rule is spelled twice, and that is a known cost.** Bridge decides, so the
rule has to exist in TypeScript; the desktop app has no test runner, so the only
place the four readings can be proved is Rust. Two spellings of one rule is
exactly what this repository calls a second vocabulary, and it is written down
here rather than left to be discovered.

## Why skew is dangerous here specifically

Version skew is usually a deploy-time annoyance: you restart the old thing,
it's fine. That's not what happens here, because **Fleet outlives Bridge by
design.** Fleet is a daemon; Bridge is a window someone closes to go to lunch.
A Job runs unattended, with Drones spending real tokens against real API
budgets, for however long it takes — hours, sometimes. Fleet gets upgraded
during that window because that's when upgrades happen: nobody's watching.

So the skew window isn't "between deploys," it's "for the entire duration of
whatever Job happens to be running when someone updates Fleet." A major-bump
skew discovered mid-Job doesn't get a graceful restart — the connection that
was streaming Drone events goes bad while a Drone is mid-tool-call, burning
budget, with nobody able to see what it's doing until Bridge reconnects
through the lifeboat and can offer nothing better than "kill it." That's the
cost minor-bump-additive-only is bought against: a minor bump has to be safe
to hit *mid-Job*, unattended, with money on the line, not just safe to hit at
startup.

**And the same lifetimes make the refusing direction the likely one.** A
running Fleet's version does not change when someone updates the app — the
daemon that was started last week is still speaking last week's protocol, and
the Bridge relaunched after the update is the newer of the two. So "Fleet
behind" is what an ordinary update produces and "Fleet ahead" is the rarer
case, reached by restarting Fleet without relaunching Bridge. The banner is not
the common path. The refusal is, and its screen has to say plainly that the
daemon is the thing to restart.

## The v0 lifeboat

When the version check refuses — either of the bottom two rows above — Bridge
doesn't get nothing. It gets four routes that don't depend on version
agreement:

| Operation | Route |
|---|---|
| List Jobs with status | `GET /v0/jobs` |
| Kill a Job | `POST /v0/jobs/:id/kill` |
| Stop Fleet | `POST /v0/stop` |
| Report Fleet's version | `GET /v0/version` |

That's the whole surface. Bridge's recovery screen is built on exactly these
four: show what's running, name both versions so the human can tell what's
mismatched, and offer per-Job kill so nothing is left burning tokens
unsupervised while someone goes and fixes the mismatch.

The lifeboat's entire value proposition is being the one thing guaranteed to
work when everything else — the ipc types, the codegen, the version
negotiation — has already failed or gone stale. That guarantee has exactly one
precondition: **the lifeboat itself never needs to change.** Concretely, that
means:

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
negotiating, or letting it grow an auth or session concept that the main
protocol also has to keep in sync with. Every one of those is a small,
reasonable-sounding change that turns four static routes into a second thing
that can be stale. If a change to this document's four rows is ever proposed,
that's the signal to slow down, not speed up.

## The first socket: every Job's state

`GET /events` is the stream Bridge draws the Board from. Its shape is
`crates/ipc/src/event.rs`, which points here.

**A reconnection resyncs; it does not replay.** Every connection opens with a
`Resync` carrying the current state of every Job and the cursor that state is
current as of. Replay from the beginning was rejected on what it costs mid-Job:
a Bridge reopened after lunch would fold hours of transitions before it could
draw anything, into a Board Fleet could state in one message.

| A resync rebuilds | A resync cannot |
|---|---|
| Where every Job is now, and why | The path taken — no transition history |
| Which step it is on | An instant for anything that already happened |
| Whether a Drone is on it | Anything about a Job retained out after a terminal status |

A surface drawing a timeline reads `get_job_events`, and its timeline begins at
the connection.

**What a resync cannot rebuild, Bridge reads back — every region together.** A
resync is a Board, so nothing under the open Job comes with it, and the
receiving side is what makes the open Job's screen whole again. That reading is
one list in `apps/desktop/src/main/screen.ts`, and it is one list on
purpose: the reads it names were once classified at the call site, one region at
a time, and the region left out was the panel reporting the outage. A read added
to that screen is added to the list.

**A resync arrives on two occasions, and they are not the same recovery.** The
first message on a connection is Fleet coming back, and every read attempted
while the socket was down failed — so a surface can be holding a failure nothing
else will clear. A resync following a `Missed` is a gap under a connection that
held, where HTTP answered throughout, so only what events keep current can be
stale. Bridge tells them apart by which resync it is on the socket.

| Taken again on a gap | Taken again only on a reconnection |
|---|---|
| Everything an event re-reads: the Job whole, what it holds, its history | The reads only a press makes, and only where one is showing a failure |
| Either per-Job socket that is down | |

**A reconnection is not a refresh.** Re-reading every open surface on every
resync spends the bytes the `get_job`/`get_diff` split exists to save, and a
flapping Fleet turns that into a fetch every retry. What comes back is what a
screen is showing and cannot repair itself.

**The stream is global, and a client subscribes to nothing.** Bridge holds
exactly one connection and the Board renders every Job on it. A per-Job
subscription would put state on a connection whose whole value is being cheap
to drop and remake, and would need a subscribe message, an unsubscribe message,
and a rule for what a resync means when the set changes mid-stream. The one
place a per-Job subscription *is* right is the second socket, below.

### A kind exists when something produces it

**An event kind that never fires reads as a stream that is working**, so a kind
`crates/ipc/operations.toml` names is not stubbed until a record exists for it
to carry. The kinds still waiting describe records this workspace has no type
for.

Two of the produced kinds are there because their absence was a specific
defect, and both are the same defect: what changed most during a run was what
the stream did not carry. A Job running four steps emitted one event until
`job.step_advanced` arrived; a Board could show which Drone was on a Job only
by re-reading the Job until the Drone lifecycle pair did.

`job.files_changed` is the only kind describing a worktree rather than a
record. **Bridge does not read a worktree** — no surface on the far side of
this seam opens a repository, so a file list reaches one only as an event.

### `job.created` is a kind, not a state change

A Job proposed while a client was connected never reached it: creation
published nothing, so nothing woke Bridge and the row appeared only when
something else forced a re-read.

Publishing a `JobStateChanged` instead was rejected on what that message would
have to say. The type has a `from` and a `to`, and a created Job has no `from`
— the honest fields would be `from: awaiting_approval, to: awaiting_approval`,
a transition the edge table does not contain, from a status the Job was never
in. Every client folding the stream would apply a move that did not happen. **A
creation is a row appearing, not a row moving.**

It carries the whole `JobSummary`, because a kind naming only an id would make
every client fetch the row it was just told about.

`job.forgotten` is the opposite message and carries the opposite payload: only
the id. A forget is a real deletion through `Store::forget_job`, and by the
time the event is published there is no row left to carry. A client drops it
rather than replacing it.

### A fact about the fleet is a reading, not only an event

`manifest.reread` says what Fleet's last read of `armada.yml` came to. It is the
second kind naming no Job — `proposal.moved` is the first — and unlike that one
it names no Drone and no step either: a Manifest is Fleet's own, so nothing on
the Board moves when it arrives.

**It is served as well as published, and that pairing is the point.** A refused
Manifest is not an instant that passes. The file on disk and the values Fleet is
running with go on disagreeing until somebody corrects the file, so the fact
outlives any window that happened to be open when the save landed. An event
alone would reach only whoever was looking; `get_manifest_reading` is what a
Bridge opened a minute later asks. `FleetCapacity` is the same shape one fact
over, and a doctor result would be the third.

The rule that falls out: **a fleet-wide fact that persists gets a route and an
event, not an event alone.** A fact that is true only in the instant it happens
— a call going out, a step advancing — needs no route, because there is nothing
left to ask about.

## The unmeasured risk: the WebSocket sink has no back-pressure

This hasn't bitten anyone yet, which is exactly why it's the most dangerous
item here — nobody has a number for it. `axum`'s WebSocket sink is unbounded
on the application side: if Fleet pushes events faster than Bridge's socket
drains them, the server-side buffer just grows. Nothing in the stack currently
pushes back.

Picture several Drones running against a Bridge that's been minimized to the
tray, or is on a slow connection, or is just a slow renderer under load.
Fleet keeps producing tool-call and status events at Drone speed. Bridge
drains them at UI-thread speed. There's no mechanism that notices the gap and
does anything about it — the buffer absorbs the difference until it doesn't.

The fix is built and the risk is still unmeasured, which is the state to read
this in. `crates/api/src/stream.rs` publishes through a **bounded broadcast
with drop-oldest**, so a slow consumer loses old events instead of growing
Fleet's memory without limit, and a drop comes with the message the paragraph
below asks for: *you missed N events, resync*. What nobody has is a number —
how many events a real fleet produces against a real minimised Bridge, and
whether `BACKLOG` absorbs it. The reason this matters as a protocol concern
and not just a
performance one is what happens without it — a reconnecting Bridge that
silently believes its event log is complete will render a Job Board that's
quietly wrong, and "quietly wrong" is worse here than "visibly stale," because
nothing on screen tells the person to distrust it.

If your change touches the event stream, say explicitly whether it makes this
better (bounds something, adds a resync signal) or worse (adds another
unbounded queue, another place assuming delivery is complete).

**`get_events_since` made it better, and is the shape to copy.** An agent
cannot hold a socket, so it polls, and what it polls is a second bounded
window beside the channel: positions and kind names rather than events,
`TALLIED` of them, and a caller whose cursor fell off the back is told how many
it cannot be told about. Nothing is queued per client and nothing is retained
for one that never comes back.

## The second socket: one Job's turns

`GET /jobs/:job_id/observe` is a WebSocket upgrade, and it is the one query in
`operations.toml` whose transport is the socket. It answers with the turns a
Job's Drones have already taken and then continues with the ones that follow,
so joining a Job already running takes one connection rather than a history
call and a subscription that have to be stitched together.

**Who reads it.** A person, through Bridge, on the machine Fleet is running on.
`agent_access` is `No`: a Drone's whole transcript streamed into a session
stays in that session for the rest of it, and `get_drone` serves the snapshot
instead.

**What it needs.** A running Fleet and a Job id. Nothing else — a Job with no
transcript is served, and so is one whose Drone is gone.

**What a viewer sees.** The first message is always `opened`, carrying the
protocol version, the Job, whether a Drone is writing right now (`live`) and
how many older rows the history left out (`skipped`, because the backfill is
bounded). Then the history, oldest first, across every Drone the Job has had —
a retry is a second `drone_id` under one `job_id` and both are the Job's
history. Then the live rows. The connection ends with a `closed` message
saying why, because a socket that simply stops is indistinguishable from one
that broke.

| What happened | What the viewer is told |
| --- | --- |
| A Drone is working | `opened` with `live: true`, the history, then rows, then `closed` / `drone_ended` when the Drone finishes |
| The Job was never dispatched | `opened` with `live: false`, nothing, `closed` / `nothing_writing` |
| Fleet restarted under a Drone that outlived it | The same. Fleet's writer does not reattach and `reconcile` escalates the Job as `interrupted`, so the history is whole and nothing is live |
| The Job id names nothing | **404 before the upgrade**, through the error contract, at the moment they asked |

**Back-pressure, and what is dropped.** Three bounds sit in a row and each is
stated rather than silent. The transcript's file queue drops a row it cannot
take and writes a `missed` row into the file among the rows it was lost
between. The per-Job broadcast channel is drop-oldest, and a viewer that has
fallen behind gets a `missed` message with the count, and Bridge answers it by
reopening the socket, so the backfill redraws the pane whole (#1759). Neither can slow Fleet's
line loop: the file queue is `try_send` and the channel's send is synchronous
and never blocks, so **watching a Job cannot change its outcome**. What a slow
viewer slows is its own socket task.

**It is deliberately not `/events`.** That stream is one drop-oldest channel of
fixed capacity carrying every Job, so transcript rows at Drone speed would
evict the state changes Bridge draws the Board from — and an eviction there is
not a lost row but a `Missed` and a full resync of every Job, paid
continuously. This is the one place a per-Job subscription is right, and
`docs/concepts/observe.md` is why.

## The run socket: one person's run

`GET /jobs/:job_id/runs/:run_id/observe` is the second socket's shape one
subject over: a run a person started from the run sheet, and
`crates/api/src/watching_run.rs`. It answers with what the run's log already
holds, then the lines the run prints next, then `closed`.

**Who reads it.** The run sheet showing that run, and nothing else.
`agent_access` is `No`.

**Per run, not per Job's runs.** The sheet holds the run's id from
`start_run`'s answer, a run has an end to close on, and a Job has one run out
at a time. A per-Job channel would need the second socket's hand-over between
runs, for a reader nobody has.

| What happened | What the viewer is told |
| --- | --- |
| The run is going | `opened` with `live: true`, the log so far, then lines, then `closed` / `finished` |
| The run has ended | `opened` with `live: false`, the log, `closed` / `finished` |
| The log will not read | `opened`, then `closed` / `unreadable` |
| The id names no run of this Job | **422 before the upgrade**, through the error contract |

**Back-pressure.** The channel is per run and drop-oldest, and a viewer that
falls behind is sent `missed` with the count. The run's log keeps every line
and `get_run_output` reads it, so a drop costs a live line and never the
record. Each live line carries the byte offset just past it, which is what
keeps a line the opening read sent from being sent again.

**It is deliberately not `/events`.** Output is the fastest thing a run makes,
and on the global stream it would evict the state the Board is drawn from.
`/events` carries the run's end, `run.finished`, because that is a fact about a
Job a surface may care about; it never carries a line.

## The server socket: one server's output

`GET /servers/:server_id/observe` is the run socket's shape per server
instance, and `crates/api/src/watching_run.rs` relays both. It opens with what
the server's log holds, then the lines it prints next, then `closed` once the
server has ended. **A server that has ended opens too**, with its whole log,
which is how one that fell over is read.

`/events` carries a server's three lifecycle facts — `server.starting`,
`server.serving`, `server.exited` — each with the whole instance, and never a
line of its output, for the run socket's reason. **A row is replaced rather
than patched**, which is why a server that goes behind its checkout is
published again under the kind its phase is rather than under a fourth kind. A Job's servers publish
`server.exited` before the Job's own terminal `job.state_changed`, because they
are torn down before its span is released.

## The Helm socket: one repository's conversation

`GET /helm/observe?manifest_id=` is the second socket's shape with a
conversation for its subject, and `crates/api/src/conversing.rs`. It opens with
the thread so far, then carries every message after it. `POST /helm/ask` takes a
message and answers 202 at once, because the reply is the socket's;
`POST /helm/start_fresh` forgets the stored session and the thread.

**Who reads it.** Bridge's Helm thread. `agent_access` is `No` on all three: a
session must not read or drive another session.

**No end to close on.** A Drone ends and a run finishes; a conversation outlives
every process that answers in it. So the socket carries reply after reply for as
long as a viewer holds it, and `closed` is sent only when a person starts fresh.

| What happened | What the viewer is told |
| --- | --- |
| A person asked | `asked`, then the session's own `row`s, ending in `ended` with what the reply cost |
| The stored session was gone | `fresh` / `session_not_found`, then the rows of the new session |
| No reply came | `unanswered`, with why |
| A person started fresh | `closed` / `started_fresh` |
| The Manifest is not served | **422 before the upgrade** |

**A row is a `Shown` row**, so a reply is read with the vocabulary a Drone's turn
already has. **Back-pressure** is the second socket's: a channel per
conversation, drop-oldest, `missed` with the count, and the thread's file keeps
what a slow viewer lost.

**It is deliberately not `/events`**, for the second socket's reason.

## Protocol 10.11: the review Fleet composed

Job detail's review area and the pull request's description said nearly the
same thing from two independent builders — `#665`. `ipc::JobReview`, additive
on `get_job`'s `JobDetail`, is the fix: Fleet composes one review from the one
reading of the record, in four parts — `why` (the brief, in the requester's
own words), `outcome` (what the worktree changed, as far as a diff can say
it), `risks` (what nothing checked, and what the base carries that this Job
did not write), `evidence` (every step and every Check that ran, with its
outcome) — and renders it two ways: the pull request's Markdown body,
unchanged in content, and this DTO. `crates/fleet/src/review.rs` is the one
builder both readings come from.

Composed at the delivering step's entry, as before, and now also at any other
`human_always` gate, so a workflow that never opens a pull request still
reaches a person with the same four sections. **Absent is a Job that has not
reached a gate yet**, not an empty review — a Job still running, or one that
finished with no `human_always` step at all, carries nothing here, and so does
every Job read from a Fleet older than 10.11.

`VerdictSheet` draws `risks` as its own card, "What was not checked", less the
paragraph Fleet opens it with. `why` it no longer draws: Overview's Brief card
already carries the request, and the record repeated it whole (#1680). The
Drone's own claims stay their own cards, "What was done" and "What was skipped"
— the owner's names of 2 Oct 2026, because several Drones now share one Job.
The pull request leaves them out on purpose; the old labels, "What the Drone
says it did", were what marked them as a self-report, and the new ones no
longer say whose account it is.

## Protocol 10.12: keeping a pull request current, and resolving its conflicts

`#663`. Fleet stopped closing and reopening a Job's pull request when main
moved under it — closing and reopening re-pinned the forge's comparison but
never touched the branch, so a pull request behind its base with conflicts
stayed that way. Fleet now rebases the branch and pushes it: clean, in place;
conflicted, the branch is left exactly as it was and a person is told.

`ipc::Currency`, additive on `PullRequestDetail`, names the base a branch was
last brought up to (`rebased_onto`, `rebased_at`) and, where the attempt
conflicted, the files (`conflict_files`) — absent is a branch that has never
needed to move. `resolve_pull_request_conflict` is a new route and `Commands`
method at the review gate: a person sends the branch back for a Drone that
can edit files to bring it current, on the step before the one that
delivers, never the gate's own — #660 found a picked pull-request comment
landing on a summarising step's Drone, which has no git and no code to edit.

## Protocol 11.2: a delivery that skipped its push says so

`#691`. A delivering step's catch-up can conflict with its base after the
step has already made its commit, and `fleet::delivery::deliver` was right to
leave the branch unpushed there — a pull request opened over a conflict is a
review request nobody can act on. What it did not do was say so: the commit
still landed, the Job's log carried nothing about the push it skipped, and a
redelivery onto a pull request that already existed left that pull request
showing the commit before it, silently, all the way to a person approving
work it did not contain.

`ipc::JobDelivery.unpushed`, additive, names why the last commit here never
reached its remote — absent is a push that went out, a repository with no
remote, or a Job that has not reached a delivering step; present is the one
fact the field exists for. Fleet's own record no longer clears `pushed` and
`pull_request` on a skipped push either: nothing about the remote changed
that turn, so the store keeps naming the pull request `resolve_pull_request_conflict`
already knows how to send a Drone back to.

A delivering step whose own catch-up conflicted can no longer carry a Job to
`completed_success` silently: `Ruling::Finished` on such a step is held for a
person instead, and an approval or an override reaching the Job's own ending
while the last commit is unpushed is refused rather than completing over a
pull request that does not carry it.
## Protocol 11.3: a Judge that is unsure asks, rather than stopping the step

`docs/concepts/judge.md`'s asking design, closing #694. A refusal on a
criterion marked `refuse` still stops the step exactly as every refusal did
before this existed; the rest hold the step open at a question instead, and
`declared_plan_drift` can never be marked `refuse` at all.

`ipc::JudgeQuestion`, additive on `get_job`'s `JobDetail` beside
`command_waiting`: the refused criterion, the plain question it asked, the
Judge's own `expected` / `produced` / `consequence`, and when it was raised.
Absent is the ordinary case, and every Job read from a Fleet older than 11.3.

`answer_judge` is a new route and `Commands` method, taking `ipc::JudgeAnswered`
— `answer` (`agree`, `disagree_once` or `disagree_always`) and an optional
`note` that rides along for the record. `agree` fails the step exactly as it
would have without this design; either disagree advances it, and
`disagree_always` also stands the criterion down for the repository, so no
later Job is asked about it either. Refused with a 409 where the Job is not
holding a question open.

## Protocol 11.6: a recording is watched a span at a time

`#615`. A captured video over 20 MiB never played on the Job screen and a
smaller one downloaded whole before a frame of it drew, because every hop read
the file whole: `get_frame` answered the bytes in one response, Bridge's main
process took them into an array, and the renderer made a `blob:` of it. A
two-minute walk through an app runs past that in a minute, so the evidence most
worth watching was the evidence the screen skipped.

`get_frame` now honours `Range` — **on a frame whose media type is a video, and
on nothing else**. A satisfiable span answers 206 with `Content-Range`; one
beginning past the end answers 416 naming the length, which is the one fact a
player can ask again with. A range on any other kind is ignored and the file is
answered whole, which is always legal and keeps the executable kinds — SVG,
HTML — on the single path that has always answered them as bytes.

**Additive, and that is why the minor moved.** An older Bridge sends no `Range`
and gets exactly the response it got before, headers included. The route, the
id and the refusals are unchanged: `kept` still names a row, the record is
still the allowlist that resolves it before a file is opened, and a name no row
of the Job holds still reaches nothing whatever it spells.

**A long span is windowed rather than honoured**, because a player opens a
recording by asking for everything from byte zero. `showing::frame_part` seeks
and reads a bounded window, so the allocation is the window and never the
capture's length — answering fewer bytes than were asked for is legal, and the
player asks again.

Bridge's half is a privileged scheme in the main process, which forwards the
range to Fleet and streams the answer back. **The renderer still never reaches
Fleet's port** — `docs/practices/bridge.md` is where that half is written down.

## Protocol 12.0: a step's evidence is two questions

`#777`. `ipc::EvidenceType` loses `shown`, and removing a value from a set the
wire carries is a major bump by this document's own table — so the major moves
and the minor resets.

`shown` was never a claim a Drone handed in. Every other value names a work
product the gate measures against the step's own declaration; that one meant
*Fleet, run the repository's harness*, which is an instruction. Holding both in
one field is why a step could not hand in a patch **and** be captured, and a
WorkflowDef step now says the two separately: `evidence.submitted.type` is the
claim, `evidence.captured` is the instruction, and it gates nothing.

**No DTO gains or loses a field.** `ipc::Submitted.evidence_type` still carries
what a submission was recorded as, and that is still the workflow's word rather
than the Drone's. Being captured is a fact about the frozen step and not about a
submission, so nothing on this seam had to carry it — what a capture produced
already reaches Bridge as `frames` on the step, unchanged. The break is the
narrower set alone, which is why the refusal is worth the major: a Bridge built
before this looks `shown` up in the generated vocabulary and finds a word Fleet
can no longer send.

## Protocol 13.0: a step's flags are every attempt's

`#791`. `StepDetail.flagged` held the newest attempt's gaming flags, with no
attempt on them; it now holds every attempt's, each stamped with the run that
raised it — the change `check_runs` and `judged` took at 7.0.

**The field is not new, its meaning is, and that is the major.** A Bridge built
before this reads `flagged` as where the step stands now, and handed every
attempt's flags it would draw a run that is over as the reason the step is held.
The store always kept a flag's attempt; only the detail route dropped it.

## Protocol 13.1: a spec a person picks

`#619`. `show_again` gains an optional body naming which spec to run,
`ShowAgain` gains `specs` — every spec this Job's Drones named, latest first —
and `ShownSet` gains the `spec` a press ran. Additive in all three, so the minor
moves.

**The choices are the record's own words.** A spec reaches `evidence.run` as an
argument, so the wire had to answer what stops a path leaving the worktree. It
is not validation: Fleet refuses anything that is not in the list it just sent,
and every entry in that list is a `shown_by` a Drone submitted from inside the
worktree. A directory listing or a typed path would each have needed a rule
about `..` and about absolute paths; a list has none to break.

**A press with no body is unchanged**, which is what keeps this additive in
behaviour as well as in shape: an older Bridge sends nothing and runs the last
spec a Drone named, exactly as it did at 13.0.

## Protocol 13.2: the limits a person changes

`get_limits` and `save_limits` are new routes, so the minor moves. `cpu` leaves
`admission_hold` in the same change, and that is minor too: Bridge reads the set
as opaque, and a Bridge built before this simply never sees the word again.

**A value out of range does not decode.** `SaveLimits` holds each field as a
bounded integer, so the refusal is the ordinary undecodable 400 and Fleet is
never asked. Bridge bounds the field the same way, so a person meets the range
before the wire does.

## Protocol 13.3: the moment a Drone submits

`#813`. `evidence.submitted` is a new event kind, so the minor moves. The row
had been declared since `522dac92` with no variant behind it, and the mark that
said so came off in the same change.

**It is a pointer and the decision was that it stays one.** The stream is one
bounded drop-oldest broadcast every Job shares, which is the argument that had
kept this unbuilt — and it defeats a payload-carrying event and nothing else.
So the message names the Job, the step and what the frozen step asked the work
product to be, and `get_evidence` serves the three sentences to whoever opens
the Job. `job.step_advanced` is the shape it follows.

**Published where the fact is, which is the Evidence call.** The gate notices on
its next turn; a message sent from there would be dated wrong and would say what
`job.checking` already says one message later.

## Protocol 13.4: Always allow picks a rule, not a whole command

`#834`. `CommandInFlight` and `Refusal` each gain `rules` (the leading cuts of
the command, shortest first) and `suggested_rule` (the one pre-selected), and
`AnswerCommand` gains `rule` — the one a person picked, read only where the
answer is Always allow. All three are additive: an old Bridge neither reads
`rules` nor sends `rule`, so it keeps writing the whole command as the rule,
exactly as it always has, and the minor moves rather than the major.

## Protocol 13.7: Always allow stops writing to `armada.yml`

`#836`. Fleet's own commit of the always-allowed line was itself a change on
the Job's branch, so the absolute boundary on `armada.yml`
(`crates/verification/src/forbidden.rs`) refused every later step of the Job
the allow was pressed on. Always allow now commits nothing: the rule is kept
in a table of its own, per Manifest, and granted to every Job against it the
way a declared, non-destructive Command already is.

`get_repository_allowed_commands` and `remove_repository_allowed_command` are
new routes, so the minor moves. `ipc::JobDetail` gains
`repository_allowed_commands`, additive for the same reason — every rule a
person always-allowed for the Manifest, read-only there, beside the Job's own
`allowed_commands`. **`allowed_commands` itself changes what it means, not its
shape**: a `repository`-reach row there is now one an older Fleet wrote before
13.7, kept rather than migrated, and never one this Fleet writes going
forward — an old Bridge reading it as before still reads a real historical
row, so nothing about the field's shape or presence changed under it.

## Protocol 13.24: Armada's review of a change

`ipc::JobConfidence`, additive on `get_job`'s `JobDetail` as `confidence`, is the review a person reads at the stop before merging (#903): whether Armada is confident and why, the change's areas, the tests in it, and its findings sorted into needs you, small fixes and for context. `tests.opened_because` names a test removed or loosened with no reason, and that row is `flagged`.

**Not `review`.** `JobDetail.review` is the text Fleet composes for the pull request, from 10.11. **Absent is a Job with no accepted review**, which is every Job whose steps ask for none.

## Protocol 13.25: the code a review is about, as a View

`ipc::ViewStepRow`, additive on `JobConfidence` as `view` on an area and on a finding (#904): the files it is about, as steps in the order one change forces the next. Each step names its hunk by the patch's `@@` header, with a one-sentence summary and, except on the last, what ties it to the next.

**The hunk is named, never copied.** Bridge finds it in the patch `get_job_diff` serves, so a View cannot show code the branch no longer holds. **Left out where empty**, which is every area and finding the reviewer gave no View.

## Protocol 13.29: findings a person dismissed

`ipc::DismissedRow`, additive on `JobConfidence` as `dismissed`, and `dismiss_finding`'s body `ipc::FindingDismissed` (#907). A dismissed finding leaves `needs_you`, `small_fixes` and `for_context` and is listed in `dismissed` with the reason a person gave. **Left out where empty**, which is every Job nobody dismissed anything on.

## Protocol 13.30: a person's add and drop reach the plan

`#897`. `add_task` and `drop_task` are new routes, so the minor moves —
`13.27`, `13.28` and `13.29` having each reached `main` first for unrelated
changes. A person adds a task (`title`, `detail`, `after`) or drops one with
a reason (`task`, `reason`), each kept under `store::PlanHand::Person` so the
record shows who made the change. Both answer with `ipc::WorkPlan`, the plan
the change leaves.

**Delivery follows `redirect_drone`'s rule.** With a working Drone mid-step, a
turn is injected naming the task and, for a drop, the reason — its own
`Occasion::Plan`, so the log shows it was not a redirect. At a step boundary,
or with no session, nothing is sent, because the next brief's THE PLAN is
built from the record at every spawn and carries the change already. Neither
route ever respawns a Drone to deliver itself.

Refused by name: no plan recorded (`fleet.no_plan`), a task or a place to add
after the plan does not hold (`fleet.no_such_task`), a task already `done` or
already `dropped` (`fleet.task_already_settled`, a 409 — a person's drop does
not repeat a decision already made), and a blank title or reason, on
`redirect_drone`'s reuse of `fleet.unacceptable_proposal` for a value that
cannot work.

## Protocol 13.33: a Job's review model

`JobDetail.review_model_override` and `JobDetail.review_step`, additive, and the command `set_review_model`, which takes `set_model`'s body (#903). A person chooses the model the step that writes Armada's review runs on, and on that step it beats `model_override`. `review_step` is that step's label, **absent on a workflow with no review step**, which is where Bridge draws no review model at all.

## Protocol 13.34: a pull request's CI, and what a person does about it

`PullRequestDetail.checks`, additive: what the forge's own CI came to on the pull request, as the sweep last read it, with the names of the checks that failed (#905). Two commands take no body: `rerun_failed_checks` asks the forge to start the failed runs again, and `investigate_failed_checks` sends the Job back to the step before the one that delivers with the failed checks as the next Drone's note. **Neither posts anything on the pull request.**

## Protocol 13.36: what a review finding became

`JobConfidence.followed`, additive: each For context finding a person turned into a Job queued behind this one, by its id, or into an issue, by its address (#906). Two commands: `queue_after_finding` takes the finding and proposes a Job created waiting on this one, and `file_finding_issue` takes the finding with the title and body a person confirmed and files it on the forge. **Neither posts anything on the pull request.**

## Protocol 13.37: the agent door answers about the repository a session stands in

`?manifest_id=`, optional and additive, on `list_jobs`, `list_job_board`, `list_reviews`, `get_activity_feed`, `list_alerts`, `list_drones`, `list_worktrees`, `list_servers` and `get_events_since`: absent is every repository, as before, so Bridge's All view is unchanged (#987). A named `get_events_since` counts events about that Manifest, about a Job it owns, and those naming neither, which are the machine's. `armada mcp` names the Manifest it walked to on every call to `/agent/mcp`, and the door names it on each of those routes and on the Manifest reads. Through the door, a `:job_id` another Manifest owns is refused as `fleet.job_in_another_repository`, a call naming another Manifest is refused, and `propose_job` takes its owner from the scope. Bridge's own routes are unscoped.

## Protocol 13.38: a workspace's Command runs where the root has no Manifest

`?repository=<root>` on `start_checkout_run`, `undo_checkout_run`, `list_checkout_runs`, `get_checkout_run_output` and `get_checkout_run_diff`, refused beside `?manifest_id=` as Verify's routes are (#986). `StartCheckoutRun.workspace`, optional, names a directory whose own `armada.yml` declares the Command. `CheckoutRunSheet.workspaces` lists each workspace's Commands, and `workspace` rides on `CheckoutRunUnderway` and `CheckoutRunRecord`. All additive.

## Protocol 13.40: how many of a step's Checks run at once

`LimitValues.checks_at_once` and `SaveLimits.checks_at_once`, additive: a fourth limit on `get_limits` and `save_limits`, from 1 to 8 (#284). Fleet runs a step's Checks up to it, and before starting each one after the first reads the machine against the memory and disk limits, so a short machine makes the next Check wait for a running one to finish.

## Protocol 13.43: a gate reuses a passing dry run

`CheckRun.reused_from_dry_run`, additive: when a Check's result came from the Drone's own `run_checks` instead of the gate running it again, rather than absent for a Check the gate ran itself (#1014).

## Protocol 13.44: Helm puts an approval card in front of the person

`ask_person_to_approve`, additive: a new command, `POST /jobs/:job_id/ask_person_to_approve`, answering `AskedApproval { job_id, handle }`. `agent_access = "Drafts only"`, the door offers it to a Helm session alone (#1041). It writes nothing — `Resolved` already turns a Job id that names nothing into the ordinary 404, and past that the route hands the id and handle back untouched. `approve_dispatch` stays `agent_access = "No"` for every agent; this only names which Job `HelmThread` draws a card for, and the person's own press on it is still what releases the Job.

## Protocol 13.45: a Drone's own run of the Checks, shown as it runs

`StepDetail.dry_run` and the `job.dry_run` event, additive: the Checks a Drone asked for mid-step, in `ChecksUnderway`'s shape, from their start until the Drone asks again, submits or the step ends (#1062). `checking` stays the gate's alone, and an event kind of its own keeps a Bridge that does not know it from drawing a Drone's run as the gate's. `CheckUnderway.stopped_by`, on a Drone's run alone, names the Check whose failure stopped this one before it finished, and its `produced` says so in words.

## Protocol 13.46: what a worktree's build started from

`RunSheet.seeding` and `CheckoutRunSheet.seed`, additive (#1064). `seeding` is absent where the Job's Manifest declares no `setup.seed`; otherwise it is `seeded`, with the base commit and the directories cloned, `cold`, with Fleet's sentence for why, or `unrecorded` for a worktree cut before seeding existed. `seed` is absent where the Manifest declares none; otherwise it names the directories and the Commands that warm them, and says whether the seed at the current base commit is `warm`, `warming` or `cold`.

## Protocol 13.47: a Check waiting for room says what it waits behind

`CheckUnderway.waiting_behind`, additive: on a Check still waiting, how many Checks from other work hold the machine's places while its run waits for one (#1063). `LimitValues.checks_at_once` keeps its shape and range and now counts across the machine — every Job's gate, every Drone's own run, fix drafts and proofs after a merge share it — so a gate can wait on work that is not its own. Absent is a Check waiting on nothing but its own run.

## Protocol 13.48: where a Check runs

`DeclaredCheck.runs_at`, `StepDetail.held_for_handoff` and `WorkflowStep.held_for_handoff`, additive (#849). `runs_at` is `gate` for a Check a Drone's own run never asks and `handoff` for one that runs last, on the step before handoff, once every other Check there passes; absent is everywhere. `held_for_handoff` names the handoff-only Checks a step's gate leaves to a later step, so a step that passed is not read as having run them. A handoff-only Check that was not reached records `skipped`, with its own sentence in `produced`.

## Protocol 13.51: a Check's own weight

`CheckUnderway.places`, additive beside `waiting_behind`: how many of the machine's places this Check takes, absent where it takes one (#1102). A browser suite costing more than `format` now says so where its wait is; Bridge names it only for a Check taking more than one.

## Protocol 13.52: running a stopped step's Checks again

`rerun_checks`, additive: a new command, `POST /jobs/:job_id/rerun_checks`, with no body, answering `JobSummary` (#1105). `Stuck.recourse` gains `rerun_checks`, offered on a Job at `awaiting_repair` whose stopped step failed a mechanical Check. Bridge reads `recourse` as strings, so a Bridge that predates the value draws nothing for it. The request waits for the Checks, which Fleet runs on a task of its own.

## Protocol 13.53: what Fleet resolved Helm's action authority to

`FleetHealth.helm_action_authority`, additive (#1127). `settings.helm-action-authority-tier-1-redirect-enabled-vs-read-only` resolves once when Fleet starts, and until now nothing on the wire carried the answer — Settings' "This machine" section could only describe what the setting does, not say what Fleet actually decided. `GET /health` answers it now, alongside the probes it already carried.

## Protocol 14.0: a person no longer sends the conflict back

`#1131`. `resolve_pull_request_conflict` is gone — the route, the `Commands`
method and the button that pressed it (`Grounds.tsx`, `Decide.tsx`,
`verdict.tsx`). Removing an operation is a major bump by this document's own
table, so the major moves and the minor resets.

Fleet finds the same conflict where its sweep already reads one
(`fleet::currency`) and sends the Drone back itself, as `Actor::Fleet`, once
per base — `fleet::conflict_resolution::sent_to_clear_conflicts` is what a
person's press used to reach and is now reached only from there. A Bridge
built before this offered a press that answered `fleet.route_not_found`; there
is no road left for it to hit, and nobody presses anything now.

## Protocol 14.2: a sub-dispatched Job names its parent

`JobSummary.dispatched_by`, additive: the parent Job's id alone, where `origin` is `sub_dispatched` (#1165). `sub_dispatched`'s registry sentence, `"Sub-dispatched by {dispatched_by.job_id}"`, had nothing to fill its slot with, so Job detail's facts line drew nothing for it. `dependencies` and `gate_manifests` stay off the wire — the M1 decision they were withheld alongside `dispatched_by` for — since a caller reading every row still cannot draw the DAG either would.

## Protocol 14.3: where a loop returns to

`#1149`. `WorkflowStep.verdict_routing_target` and `WorkflowStep.iteration_cap`, and `StepDetail.verdict_routing_target` beside the existing `StepDetail.pass`, all additive. A person approving a dispatch could not see that a workflow loops, because `structure: loop` only labels the edge — `verdict_routing`, in `crates/config/src/workflow.rs`, is the only place it is named, and it had never crossed. `pass.of` already carried the cap on a running Job's step; the target it points at had not, on either DTO.

**On the step that sends the work back, not the step it is sent to** — `pass`'s own rule, and the same edge. `WorkflowStep` carries both fields together: a target with no cap could not stop, and a cap on a step routing nowhere answers a question a preview never asks.

## Protocol 14.4: an abandoned step's restart names a new trigger

`#1034`. `EscalationTrigger` gains `drone_gone`, the step-level trigger a
person's restart writes over a step whose Drone left before anybody acted —
`drone_killed`'s and `run_ended`'s third sibling. **Minor, on `queued_reason`'s
precedent**: `escalation_reason` carries no `wire_enum!` in `crates/ipc`, so
Bridge reads it as an opaque string through the generated vocabulary rather
than matching on it, and a new value is additive while nothing branches on it.

## Protocol 14.5: when each plan task was being worked

`#1185`. `PlanTask.working_windows`, additive and left out where empty: each
stretch a task was marked `working`, as `WorkingWindow { entered, left? }`,
oldest first. Fleet folds it from the plan's history — a move into `working`
opens one, any move out (`open`, `done`, `dropped`) closes it, and a new
recording starts every task with none. `left` is absent while the task is
still working. Bridge places a turn in the task whose window holds its
instant, so the Working area can group a step's activity by task — and where no
window holds an Edit, in the task whose `scope` names the file (`#1498`), which
is the only thing that places a turn without one. **A claim, like the state it
comes from**: a Drone that never calls `update_task` sends no windows, so
nothing but a declared path can place its work at all.

## Protocol 14.7: Helm's act on a Studio is its own event

`#1288`. `studio.helm_acted`, a new event kind, additive: published after the
`studio.changed` a write publishes, only where the door placed the call in a
Helm session, carrying `StudioHelmActed { studio_id, manifest_id, act, at }`
with `act` one of `added_node { node_id }`, `proposed_edge { edge_id }` or
`named { name }`. A person's act on a Studio publishes `studio.changed` alone,
so Helm's act is told apart by kind — `docs/concepts/helm.md`, *Audit trail*.

`StudioNode.added_by`, `StudioEdge.added_by` and `Studio.named_by`, additive
and left out where absent: `person` or `helm`, kept by store migration V78. A
row from before V78 has none, since Helm could already act under V77 and a
default would name an author nobody recorded.

`get_checkout_run_sheet`, `list_checkout_runs` and `get_checkout_run_output`
move from `No` to `Helm only`. **That half moves no number**: `agent_access`
decides what the agent door offers, which is not the Fleet/Bridge seam, and no
message either side parses changed.

## Protocol 14.8: a scout, and what its Finding read

`#1292`. Three commands and a Finding's fields, all additive. `ask_scout` (`Bridge only`) adds a Finding Gathering and starts its scout; `start_scout` (`Helm only`) starts a Finding already Proposed; `stop_scout` (`Bridge only`) is the stop on its node. A Finding's content keeps `asked` and gains `checkout` (`commit`, `uncommitted`), `read`, `searched`, `learned` and `ended` (`outcome` of `answered`, `stopped` or `failed` with `why`, and `cost_micros`), each left out until the scout records it — so a Proposed Finding is on the wire exactly as it was at 14.6.

**`cost_micros` is absent, never nought, where no cost was reported**: a scout whose group had to be ended rather than interrupted reports none, spike 017. `outcome` is a serde tag rather than a `wire_enum!`, for the node's own `kind`'s reason: it is the field the rest hang off. A Finding added through `add_studio_node` carrying any of the new fields is refused as `fleet.studio_finding_is_the_scouts`.

## Protocol 14.9: Helm is told which Studio a person has open

`#1287`. `HelmScreen` gains `studio`, the Studios surface, and `HelmContext` gains `studio` and
`node`, both optional and left out where empty: the Studio open on that surface, and the node
selected on its whiteboard, only beside its Studio. Fleet's line to the session names both by id,
so Helm can read the Studio with `get_studio` rather than guess one. **Minor, though Fleet matches
on the screen**: Bridge sends `studio` only to a Fleet at 14.9 or later, because a Fleet behind
Bridge is refused before an ask is ever sent.

## Protocol 14.10: counts on the live file list

`#1187`. `ChangedFile.lines`, additive and left out where absent: what a file
gained and lost, on `job.files_changed` only. Counting is the walk that renders
the patch, so Fleet counts on a due reading only once the Drone has made no call
since the reading before, something moved since the last count, and ten seconds
have passed since it. A reading between two counts carries the last count for
each file still listed, and none for a file that arrived since. Absent is not
zero, as on `TouchedFile.lines`.

## Protocol 14.11: Studio capture, and what a Note keeps of it

`#1290`. One command and one optional field, both additive. `capture_studio_note` (`agent_access`
`No`) puts a Note on a Studio where a person pointed in Bridge. A `note` node's content keeps
`said` and gains `capture`, left out on a Note that was typed rather than pointed — so a Note from
before this is on the wire exactly as it was at 14.6.

`capture` carries the development annotation layer's own fields — `component`, `owners`,
`selector`, `element`, `screen`, `layer`, `location`, `bounds` and `window` — and four the layer
does not record: `styles`, `markup`, `source` and `frame`. **`source` is absent, never guessed**:
React 19 fibers carry no `_debugSource`, so Bridge sends a path only where the build gives it one.

**The frame crosses as a staged file and reads back as a kept one.** The request's `frame` is
`staged_path`, `width` and `height` — the PNG Bridge's main process wrote where `stage_attachment`
writes one. Fleet copies it under `<machine>/studios/<studio_id>/` and the Note's `capture.frame`
names `filename`, `byte_size`, `width` and `height`. Nothing about where Bridge staged it reaches
a client, and no image crosses in a `studio.changed`. A staged frame over 4 MiB is
`fleet.studio_frame_too_large`, and one Fleet cannot read is `fleet.studio_frame_unreadable`.

## Protocol 14.12: a Note's frame, read back

`#1352`. One route, additive: `GET /studios/:studio_id/frames/:node_id` answers the picture a Note
kept as the file itself, the way `get_frame` answers a step's. 14.11 wrote the frame and gave a
client no way to read it.

**One path segment where a step's frame takes two.** A Studio keeps one frame per node, under the
node's own id, so the node names the file — and the name is read off the node's `capture.frame`
before anything is opened, which is what keeps a caller's text off a path. A node that is not on
the Studio is `fleet.no_such_studio_node`, a node that kept no frame is
`fleet.studio_frame_not_kept`, and a file that will not open is `fleet.studio_frame_unreadable`.

`agent_access` is `Bridge only`, where `get_frame` is `Yes`: a Note's frame is a photograph of the
window a person was working in, not of a harness's own page. **The bytes reach Bridge's renderer
over the preload and become a `blob:`** — the CSP's `img-src 'self' blob:` is unchanged, and no
scheme was added to it.

## Other things specific to this seam

**Bridge finds Fleet through a runtime file, not a fixed port.** The file
carries port, pid, and protocol version, and Bridge verifies the pid is still
alive before treating the port as live — a stale runtime file and a genuinely
unreachable Fleet look identical over a bare connection timeout, and the pid
check is what tells them apart. Any change to the runtime file's shape is a
protocol-adjacent change even though it never touches `ipc`: it's still a
contract two independently-versioned binaries agree on ahead of any
connection. Treat it with the same "what does an old reader do with an
unrecognized field" discipline as the DTOs.

**One route on the listener is not on this seam.** `/mcp` serves the Evidence
tool to a Drone — the only way a Job's work is ever reported. It shares the
port because a Drone reaches Fleet the same way Bridge does, and it shares
nothing else: the peer is a process Fleet itself spawned, the vocabulary is
MCP's rather than `ipc`'s DTOs, and the version negotiated is the MCP revision
the client asks for rather than `protocol-version.toml`'s. So it is
deliberately absent from `operations.toml` and from `SERVED`, and a row added
for either would claim Bridge can call it. It also means the rule below does
not cover it: the address is written into a Drone's `mcp.json` from `api`'s own
constant, and that shared value is what stands between a typo and a Drone that
can never report.

**The rule that reads `operations.toml` runs both ways now.** One direction
fails on a route serving a name the inventory does not have; the other fails on
a name the inventory has and nothing serves. The second carries an allowance,
and every entry in it states a reason the gate prints — a list of exemptions
with no sentence each is the silent default the column was given reasons to
remove.

**A second route on the listener is not on this seam either, and it is this
seam spoken differently.** `/agent/mcp` is the agent's door: an MCP client
reaches it, and every tool on it is one row of `operations.toml` served at the
route `SERVED` already names, so there is no second implementation to drift.
What it adds over the HTTP surface is a cap — a tool answer over 64 KiB is cut,
says so, and names the route that serves it whole — and a scope: every answer
is inside one Manifest, and the handshake says which. It answers 405 to `GET`
and `DELETE` for `/mcp`'s reason, so it adds nothing to the risk above.

**Who opens it is the repository somebody is standing in.** `armada mcp` is the
relay a repository's own `.mcp.json` names: it reads `fleet.json` for the port,
refuses on `Stale { PidHeldByAnother }` rather than connecting to a port another
process now holds, and resolves its own working directory to a Manifest — never
a request field, for the reason a Job id is not one on a Drone's tools. A
session started below a repository root walks up to it, and says at its
handshake which root it settled on. The walk ends at a repository that has no
Manifest of its own, and at the home directory, so it can reach neither a parent
repository nor an `armada.yml` sitting above every project on the machine.
A session that resolves to no Manifest, or to one this Fleet is not serving, is
answered rather than dropped: the handshake succeeds and carries the reason, and
`tools/list` is empty. **None of it is authentication** — the bind is loopback
with nothing in front of it, so this selects a Manifest and grants nothing.

**The route table is hand-written, and that's an accepted cost, not an
oversight.** A typo in a route path is a runtime 404/500, not a compile error,
on both the main protocol and the lifeboat. That trade was made deliberately
in exchange for not carrying codegen where it isn't earning its keep — see
gRPC's rejection above. It means route changes need a `curl` or integration
check in the same change, because the type system will not catch this class
of mistake for you.

## Protocol 14.13: what is on a Studio becomes work

`#1291`. Six commands and one optional field, all additive.
`group_studio_nodes` (`Bridge only`) accepts several nodes as one Cluster or
reads them in order as one Outline, with a `produced` edge from each in the
order given; `defer_on_studio` (`Bridge only`) adds a Deferral with an accepted
`blocks` edge to what it holds up; `write_up_studio_node` (`Helm only`) adds an
Issue draft; `edit_studio_draft` (`Bridge only`) replaces that draft's title and
body; `settle_contradiction` (`Bridge only`) ends a Contradiction as
`not_a_problem` or `resolved_here` with its answer; `dispatch_studio_draft`
(`Helm only`) sends the draft's text through the Job proposer and answers with
the Studio carrying a Job node per Job, each on a `produced` edge from the
draft.

A Contradiction's content gains `answer`, **absent unless a person ended it as
*Resolved here***, so a node written before this is on the wire exactly as it
was at 14.6. `HelmStudioAct` gains `wrote_up { from, node_id }` and
`dispatched { from, node_ids }`, the two acts Helm takes on a person's ask —
`docs/concepts/studio.md` publishes every act of Helm's, not only the unasked
ones.

**No new node kind, no new state and no migration.** Every kind a rung makes —
Cluster, Deferral, Issue draft, Outline, Job — and every state it sets was
already in `core-model` and in V77's `CHECK`, because `#1285` wrote the whole
vocabulary down. What was missing was the calls.

**Nothing here reaches a forge.** No operation files an issue, and dispatch
carries the draft's own text with nothing to point at: filing is optional and a
person's own act.

A Job dispatched from a Studio takes `manual` or `helm_drafted` for its
`origin`, by who pressed it, rather than the `auto_detected` every other request
through the proposer takes — **a value already on the wire, so it moves no
number.** What it changes is what a row says: *Found by Fleet* names work
Armada noticed by itself, and a draft somebody wrote up and sent is neither.

**Superseded at 16.4**, which takes the pair to `studio_dispatched` and
`studio_helm_drafted` so that the row says where from as well as who pressed.

## Protocol 14.14: a person names a Studio and puts a node on one

`#1364`. No shape moves. `rename_studio` and `add_studio_node` are reached from Bridge as well as
Helm — the `agent_access` column says which *agents* a route is offered to, and a person's own
call was never narrowed by it — and `add_studio_node` gains one refusal, `fleet.studio_node_not_a_persons`,
for a kind a person may not mint by hand. A person adds a `note`, a `link` or a `sketch`; every
other kind is made by the act that earns it, so the person's refusal and Helm's
`fleet.studio_node_not_helms` meet over the kinds neither side adds.

**Minor because a refusal code added is additive**, the way one removed is: an older Bridge reads
an unknown code as a refusal with the message beside it, which is what the error contract promises.
Bridge's own halves of both calls are new capabilities on the preload bridge and cross no wire of
their own.

## Protocol 14.15: a line of a person's own on a Link

`StudioNodeContent::Link` gains `said`, optional: the line a person wrote beside the address saying
why they kept it, and `edit_studio_link` is the operation that changes it afterwards. `#1378`.

**Additive on both counts.** The field is left out where there is none, which is exactly the shape
every Link written before it already has, so an older Bridge reads a Link as it always did. The new
route carries the line and never the address — a Link never stops being its address — and a blank
line clears it rather than being refused. A kind that is not a Link is refused as
`fleet.studio_not_a_link`, a code added the way every other refusal here was.
## Protocol 14.16: Helm writes a file in the checkout

`#1373`. `helm.changed_checkout`, a new event kind, additive: published as a Helm session's own
stream says it wrote a file, carrying `HelmChangedCheckout { manifest_id, tool, path, at }`.
Helm edits the repository's checkout directly on a person's ask, with no worktree and no Job
around the change, so this is what lets somebody who finds a file changed see that Helm changed
it — `docs/concepts/helm.md`, *Audit trail*, and the rule `studio.helm_acted` already follows.

**It names writes Armada can name, not every change Helm caused.** `tool` is one of the built-ins
that edits a file, so the path is known; a shell line may also have written something and nothing
in the stream says whether it did. Those calls stay on the conversation's own socket, where they
already were.

## Protocol 14.17: what a Link's address names on the forge

`StudioNodeContent::Link` gains `forge`, optional: `issue`, `pull_request` or `milestone`, and
absent where the address names nothing on the forge. `#1379`.

**Read off the address by Fleet on every Studio it sends, and never kept on the record.** Which
host is the forge is `crates/adapters`' to know — `verify-foundations` refuses the vendor's name
anywhere else, Bridge and `crates/ipc` included — so a rule about issue links could not be written
in TypeScript at all. This field is how Bridge knows to offer Dispatch on a Link naming an issue
without reading one.

`dispatch_studio_draft` takes such a Link as well as an Issue draft, and sends the address as the
request. **No shape moves on that route**: the request already carried `node_id` and `position`,
the gate is the same gate, and the Job node lands with a `produced` edge from the node it came
from either way. A Link naming anything else is refused as `fleet.studio_not_a_draft`, the code
that route already had.

**Additive on both counts.** An older Bridge reads a Link with no `forge` as the Link it always
read, and an older Fleet is refused by the skew rule as it always was.

**Superseded at 14.18**, which makes what an address names the node's own kind and drops this
field.

## Protocol 14.18: an Issue, a Pull request and an Epic are node kinds

`StudioNodeContent` gains `issue`, `pull_request` and `epic`, and `Link` loses `forge`. `#1394`.

Each of the three carries `address`, `number` and `said`, and a `title` absent until the node is
read in. An Issue and a Pull request carry `state` — `open`, `closed` or `merged` — and an Epic
carries `read_in`, `{ issues, total }`, how many of its issues are on the Studio of how many it
holds. Every field but `address` and `number` is optional and left out rather than sent as null.

**The kind is the concept and the adapter decides it.** `adapters::forge_node` reads an address
once, when the node is made, and answers with the node's content. Nothing reads an address again:
`Studio::of` no longer takes a classifier, and `dispatch_studio_draft` offers the three by kind.
A Link is what no adapter recognised — a board, a page, a document, a session — and dispatches
nothing.

**`add_studio_node` still takes a Link, and Fleet writes what it is.** Bridge cannot read an
address, so the seam carries the paste and not the kind; `StudioNodeByHand` is unchanged.

**Additive by 14.7's reading, which added `finding` the same way.** `forge` goes with nothing that
carries one: a Link whose address names something on the forge is converted on the boot that
applies store V79, so no message an older Bridge parses stops parsing the same way. An older
Bridge meeting one of the three leaves it off the whiteboard rather than failing, which is
`whiteboardEdges`' rule for an unknown edge and is what `cardOf` gained here.

## Protocol 14.19: a person answers a call Helm was refused

`#1389`. Two new event kinds, three new routes and a DTO family, all additive. `helm.asking_to_run`
carries `HelmAskingToRun { waiting: HelmCallInFlight }` when a Helm session reaches for something
the person's own agent settings do not cover; `helm.call_answered` carries `HelmCallAnswered {
call, manifest_id, tool, detail, rule, settled, at }` when the ask ends, whoever ended it. The
routes are `POST /helm/permission` (the agent door's own permission tool, reached by the CLI and
never by a model), `GET /helm/calls` and `POST /helm/calls/answer`.

**The ask carries no `job_id`, and that is what makes it a new type rather than a
`CommandInFlight`.** Every field of that one is about a Job — `step_id`, `allow_for_job`, a rule
written into `armada.yml` on the Job's branch — and a Helm call has none: nothing is queued, no
step is running, and what waits is one process inside one tool call. `HelmCallAnswer` is its own
closed set for the same reason, and a surface matches on it to pick controls, so a fourth value in
it is a major bump the way `WhenBlocked`'s third was.

## Protocol 15.0: a plan task carries its files and its evidence

`#1421`. `PlanTask` gains `scope`, `expects` and `shown`, and its `detail` is renamed `note`.
`AddTask` moves the same way. **The rename is what makes this a major**, and it is the whole of
what is not additive: an older Bridge reads `note` as absent and draws a task with no note at all.

A planning step already recorded which files each task touches — it wrote them into `detail` as
prose, and the step after it re-derived them by searching. `scope` is that list as a list, so the
next Drone starts from it. `expects` is what the planner says should prove the task and `shown` is
what the work says did; they are kept apart rather than reconciled, because the two disagreeing is
the fact worth seeing.

**`shown` is written by a later `update_task`, not with the recording.** It rides on the change, so
a task reopened and finished again keeps what its first pass showed.

## Protocol 14.20: an Epic read-in asks which of its issues to take

`#1405`. `ReadInLink` gains `take`, `everything` or `open`, and `EpicRead` gains `took`, `left_out`,
`kept` and `laid_out_from`. All additive, and `take` is asked on an Epic alone: every other kind has
one thing to read and nothing to ask about.

**Reading an Epic in again with the other answer widens or narrows what is on the Studio.** Fleet
makes the Issue nodes the answer wants and are not there, and takes back the ones it made that the
answer no longer wants — except any a person has since worked on, which the Epic counts as `kept`.

**An older peer is read as taking everything**, which is what reading one in used to do, so a Fleet
ahead of Bridge sends `took` and a Bridge behind it ignores it. An Epic read in before this version
carries no `took`, and is drawn saying nothing about an answer nobody gave it rather than claiming
one — which is the same rule `title` and `state` already follow at 14.18.

`laid_out_from` is Fleet's own bookkeeping on the wire: the corner of the block an Epic's issues sit
in, so a widening fills that block's gaps and a node dragged out of it is readable as dragged.

## Protocol 15.1: a Helm session can be carried to whoever could fix it

`#1367`. One new route, `GET /helm/debug`, answering `HelmDebugInfo` — the repository, its
authority and model; the brief as it was sent; the tools the door offered by name; the thread,
bounded, with each turn's cost, its calls and its refusals; what the session's last poll was
answered; and the protocol Fleet speaks beside the Fleet process that answered. Additive: a new
operation and a new DTO family, and nothing an older peer already parses changes.

**Fields and not text**, for `ErrorNotice/payload.ts`'s reason: Bridge formats them, so one
producer writes the artifact and the expanded view renders the same string the clipboard takes. A
Fleet that formatted the record and a Bridge that framed it would be two producers of one artifact.

**`polled` is kept as the door answers a session's `get_events_since`**, not recounted when the
record is taken — a window counted later is not the window a turn read. It is in memory for one run
of Fleet, so it is absent on a session that has not polled since Fleet started, and the record says
so rather than claiming the session never polled.

**The thread is bounded and says what it cut**, the way a log tail does, and a long reply is cut
with its own length beside it. `agent_access` is `No`: a session must not read another session, and
reading its own brief and roster would be reading a record kept about it.
## Protocol 16.0: one delete on a Studio's nodes, one node or eighteen

`#1411`. `remove_studio_node` is gone — the route, the DTO, the `Studios` method, the store write
and the capability Bridge reached it by. `remove_studio_nodes` replaces it, `POST
/studios/:studio_id/remove_nodes` carrying `RemoveStudioNodes { node_ids }`, `Bridge only`.
**Removing an operation is a major bump by this document's own table**, as at 14.0, so the major
moves and the minor resets.

**This was written as 15.0 and is 16.0**, because 15.0 landed underneath it while the branch was
open. Both files read `major = 15, minor = 0`, so git merged them clean and the collision was
invisible — two changes claiming one version, which is the failure this file's own numbering
exists against. A version taken on a branch is a claim about the base it was taken from, and it is
re-read at every merge of `main`.

**Two routes for one act is two paths that drift, and these had.** The single-node write left a
captured Note's frame on disk; the selection write deletes it. Retiring the first closes that leak
rather than writing it down. A Bridge built before this presses a route that answers
`fleet.route_not_found`, which the major is what stops it reaching.

**All of them or none is the store's transaction, not the caller's care.** Every name is checked
before anything is deleted, so a selection carrying one name the Studio does not hold refuses with
every node still on it — including the Studio's own `touched_at`, which rolls back with the rest.
A name given twice removes that node once. A call naming no node at all is refused as
`fleet.studio_no_nodes_named` rather than taken as a write that does nothing.

**The frames go after the write, never before it.** A Note's picture is a file beside the records
and the record is what names it, so the file is deleted once the row that named it is gone. A
refused write leaves every picture where the Note that keeps it can still draw it.

## Protocol 16.1: Kit's MCP servers, and what a Drone here is handed

`#1275`. `get_kit_servers`, `add_kit_server`, `forget_kit_server`, `set_kit_server_reach` and
`set_manifest_server_reach` — five routes under `/kit/servers`, the Manifest riding as
`?manifest_id=` the way `get_repository_allowed_commands` already carries it. Additive: no existing
field moved, and an older Bridge reads none of them.

`KitServerRow` carries **both tiers and the answer they come to**. `drones` is Kit's default,
`manifest` is this repository's word — left out where it has none, which is not a third spelling but
Kit's default answering — and `resolves` is `core_model::a_drone_resolves` over the two. It crosses
rather than being computed on the far side because the same call writes the `--mcp-config` document
a Drone is spawned against, and a surface that recomputed it could draw a server as reaching a Drone
that no Drone is handed.

`ServerAddress` is tagged by `transport`, so a reader matches one field rather than guessing which
of two optional keys turned up.

`SetManifestServerReach.reach` is `Option`, **and the key must be present**: `null` is the
take-back, and `SetModel` is the precedent. A Bridge that dropped the field would throw away a
person's word about who may reach a server and be answered 200.

Every one of the five is `agent_access = "No"`. What a Drone may reach is the owner's decision, and
a tool that read the set is a step toward one that changes it.

## Protocol 16.2: a job that was replaced names the one that replaced it

`#1439`. `JobDetail.replaced_by`, additive: `{ job_id, handle }` for the Job a redispatch minted to
replace this one, and absent on nearly every Job. A killed and redispatched Job was a dead end —
it said killed, and nothing on any surface said the work had carried on somewhere else.

**No new record, and deliberately no second column.** `JobSummary.redispatched_from` already
crosses and is the one fact; Fleet reads it as a predicate over `jobs` rather than writing the
forward edge, so the two directions cannot disagree and forgetting a replacement takes the link
with it. `store::lineage` is the read and `V82` is its index.

**On the detail and not on the summary.** The Board draws a row per Job and already folds a
lineage into one; a field here is one indexed read on the open of a Job, where a read per row
would be a query per row on a list that redraws on every event.

**The direct successor, never the end of a chain.** A replacement that was itself redispatched
carries its own, so a chain is walked by opening Jobs rather than by anything on the wire — and
there is no walk to loop. Two Jobs naming one predecessor is legal today and answers with the
newer.

**Additive, so a Fleet ahead of a Bridge sends a field it ignores** and the callout simply does
not draw, which is what an older Bridge already does with every Job.

**This was written as 16.1 and is 16.2**, because 16.1 landed underneath it while the branch was
open — the same collision 16.0's own note records, caught at the merge this time.

## Protocol 16.3: the web app, started from a Studio

`#1345`. `start_studio_server`, `POST /studios/:studio_id/start_server`, carrying
`StartStudioServer { name, position, produced_by? }` and answering `StudioServerStarted { studio,
node_id, server, already_up }`. `Helm only`, as `start_studio_run` is. Additive: a new route, a new
pair of DTOs, and one new optional field.

**Beside `start_studio_run` rather than inside it**, as `start_server` is beside `start_run`
everywhere else. A server has no exit until something stops it, it is read back by `list_servers`
and `observe_server`, and it is ended with `stop_server` — three different readers from a run's. A
single call that could answer with either would hand a caller an id and leave it to work out which
reader to ask, which is exactly the confusion the new field below exists to prevent.

`StudioNodeContent` for kind `run` gains `held`, absent or `checkout` or `server`. **Absent is a run
in the checkout**, which is every node written before this, so nothing an older Bridge already reads
changes. A Bridge that does not know the field draws a server node as an unread run — its id as a
fact, and the word *Not read yet* — which is what that build could truthfully say about an id it has
no reader for.

**A server is a Run node and not a fifteenth kind.** `docs/concepts/studio.md` says a Run node holds
a Manifest command started from the Studio, and a Command with `serve` is one; Run is also one of
only two kinds that may take status colour, which *starting* and *serving* need. A new kind would
have meant a `studio_nodes` `CHECK` rebuilt, a fifteenth row in the kinds registry, and that colour
rule amended — three costs for a distinction the content already carries.

`StudioRunKept` is unchanged and a server's fills it: `command` is the `serve` line as it ran,
`duration_ms` is how long the instance was up, and `expect_exit_code` is written but not read,
because a server that exits on its own has failed whatever its code.

**This was written as 16.2 and is 16.3**, because `#1439` took 16.2 underneath it while the branch
was open — the third time this file records that collision, and caught at the merge again.

## Protocol 16.4: a Job dispatched from a Studio says so, and reaches it

`#1362`. Two `origin` values, `studio_dispatched` and `studio_helm_drafted`,
replacing the `manual` and `helm_drafted` 14.13 gave a Studio dispatch; and
`JobDetail.from_studio`, additive — `{ studio_id, name?, node_id }`, the Studio
that produced the Job and the node to select on landing.

**Minor, on `queued_reason`'s precedent.** Bridge types `origin` as `string`
and reads it through the generated vocabulary rather than matching on it, and
that map already answers `undefined` for a key it does not hold — so an older
Bridge meeting either value draws no provenance rather than the wrong one. The
values are Fleet's own writing: Bridge proposes `manual` and cannot send these,
so the strict direction is never exercised. **The condition is the same one**:
the moment either side branches on this value rather than rendering it,
widening the set is a major bump.

**Two values and not one, and that is the whole shape of the change.** One
column answers one question, and the row has to keep answering two — where the
Job came from, and who pressed. A single value meaning *a Studio* would have
put *dispatched by you* in a second place nothing checks, which is the argument
`sub_dispatched`'s own registry row already makes.

**`from_studio` is derived and no column is added.** The `produced` edge from
the Issue draft to the Job node is the record; `store::studio::tracing` reads it
backwards, the way `store::lineage` reads `redispatched_from`. So the field is
absent on a Job whose Studio was deleted — the nodes cascade with it — and
`origin` is what still says the Job came off one. Job detail draws that pair as
*that Studio is no longer there*, with no control, which is `job-board.md`'s *A
Board outlives its Workspace* one scope smaller.

## Protocol 16.5: a proposed Check names the runner that drives it

`#1456`. `ProposedCheck.runner`, additive and optional — `{ name, pkg? }`,
carried by the new `ProposedRunner` — naming the runner setup detected from what
a workspace's own script runs. Absent is every Check nothing detected one for,
which is every Check before this and every Check in a repository whose scripts
name no runner Armada ships a description of.

**Minor, and the one direction that matters is Fleet to Bridge.** Bridge reads a
proposal to draw it and to let a person edit lines by name; it does not compose
a `ProposedCheck`, so the strict direction is never exercised. An older Bridge
meeting the field ignores it and draws what it always drew — and what a person
approves is the proposal's own `text`, the edits applied through `config`'s one
writer, so the `runner:` block reaches them in the file whether or not the
surface knows the field exists.

**What travels is a name and a package, never a command.** Every way of running
less than a whole Check is written once in that runner's own description rather
than per Check — `docs/concepts/runner-adapter.md`. So this field does not grow
when a shape is added to that schema, and a Bridge rendering it needs to know
nothing about what any runner can do.

## Protocol 16.6: a redispatched Job names the one it replaced, readably

`#1474`. `JobDetail.replaces`, additive — `{ job_id, handle }`, the Job this one
replaced, beside `replaced_by` and shaped the same.

**The same column, followed the other way.** 16.2 read `redispatched_from` as a
predicate to answer *which Job replaced this one*; this follows it as a column
to answer *which Job this one replaced*. Still no second record, still nothing
written, and V82's index is the other direction's — this join is `job_id` at
both ends. `store::lineage` holds both reads, and Fleet composes both handles
with `core_model::handle_of` at the seam.

**A DTO of its own rather than `ReplacedBy` reused**, though the two fields
match today. They answer opposite questions of one record, and the direction is
the whole content of the answer.

**Why anything crossed at all.** The id already crossed as
`JobSummary.redispatched_from`, and the header drew it: a bare ULID, which is
the one fact on the screen about where the Job came from and the one fact a
person can neither read nor press. What was missing is the predecessor's number
and title, which live on a row Bridge does not hold.

**Additive, so a Fleet ahead of a Bridge sends a field it ignores** and the
header draws the ULID it already drew.
## Protocol 17.0: a runner's directory is called `dir`

`#1456`. `ProposedRunner.pkg` becomes `dir`, and `{pkg}` becomes `{dir}` in
every runner description's templates.

**The rename is the whole of why this is major**, on `PlanTask.detail`'s
precedent one major back: the old name silently stops arriving, which no
version of "additive" covers. Nothing else about the message moved.

`pkg` said package, and the value is a directory — the one a runner is invoked
in, which is what makes `vitest` mean one suite rather than the three others in
the same repository. It came from an illustrative example rather than from a
decision, and reached a real key without anyone asking what it meant.

**Taken at the cheapest moment there was.** The field shipped in 16.5 and no
Bridge outside this repository had yet been built against it, so the refusal a
major causes falls on a rebuild that was already owed. The longer a name that
says the wrong thing survives, the more it costs to move.

## Protocol 17.1: a Note says which server it was captured on

`#1294`. `StudioCapture.served`, optional — `{ run, name, address }`: the instance Fleet held, what
the Manifest calls it, and the origin the capture window was pinned to. Additive: one new optional
field and one new DTO.

**Absent is a Note captured on Bridge**, which is every Note written before this, so nothing an
older Bridge already reads changes and no stored row is rewritten. A Bridge that does not know the
field draws the Note exactly as it draws one taken on Bridge — which is what that build can
truthfully say about a page it has no record of.

**The origin, and `location` is the path within it.** Neither repeats the other, and the two
together are the page. The address is not an identity and the field does not claim to be one:
whatever bound that loopback port owns the origin, which is why capture ends with the Run —
`docs/practices/capture-window.md`, *What this does not claim*.

**Additive against 17.0, re-read as such rather than carried across the major.** This was written
on 16.x, and 17.0's rename is what makes it worth saying out loud: what it adds is one optional
field on `StudioCapture` and one new DTO beside it, neither named anywhere 17.0 touched.
`ProposedRunner` is the manifest-proposal surface and nothing here composes it. The two new
`Outcome` arms are Bridge's own — produced in main, read in its own renderer, sent in neither
direction — so they are not on this wire at all.

**This was written as 16.5, then 16.6, then 16.7, and is 17.1.** `#1456` took 16.5, `#1477` took
16.6 and `#1487` took the major, all three while this branch was open — the fourth, fifth and sixth
times this file records the collision, and all of them on one day.

**Re-reading the file is not the check, and a conflict is not the check either.** The 16.5
collision produced no conflict at all: both branches wrote `minor = 5`, so git had nothing to
disagree about and two additive changes agreed on a number meaning two different things. The 16.6
one did conflict, and the major conflicted loudest of the three — which is exactly the trap, because
the quiet one is the one that lands. A branch that reads 17.1 and a `main` that reads 17.1 look
identical and are not. The check is whether the bump is still the branch's own:

```
git log origin/main..HEAD -- protocol-version.toml
```

**Nothing printed, on a branch whose commits touched that file, means the bump was absorbed by a
merge and the number now belongs to somebody else.** Run it after bringing `main` in, before the
Checks — not once at the start, because `main` moves under an open branch and did three times under
this one.

## Protocol 17.2: Kit reads the setup a person already has

`#1491`. `get_kit_inventory` — one new query at `GET /kit/inventory`, and `KitInventory` with
`SetupKindRow`, `WhatWasRead`, `SetupItem` and `SetupUnreadable` beside it. Additive: a new route
and new DTOs, nothing renamed and nothing an older peer already reads changed.

**A Bridge that does not know the route draws Kit exactly as it draws it today** — the servers
table and the form — which is what that build can truthfully say about a read it cannot make.

**`harness` is the one vendor word on this wire**, and it crosses as data an adapter produced
rather than as a literal Bridge holds. A second harness is a second adapter and this DTO does not
move.

**Nothing on it can widen a Drone.** A row is a name, the item's own words for itself and where it
came from; a connected server carries the program's own file name or the host it is at, and never
the argument list, query string, userinfo or environment that follow either. What crosses could not
start the server it names, so one connected outside Armada is visible here and still reaches no
Drone. `add_kit_server` and `set_kit_server_reach` are the two separate acts they were.

**The field is the same shape either way.** `says` was optional and a string when this was written
and still is; what changed on the owner's word is what an adapter puts in it, which no peer parses
differently. 17.2 stands.

## Protocol 17.3: one press allows every read Armada's door offers

`#1518`. One variant added to `HelmCallAnswer` (`allow_every_read`) and one to `HelmCallSettled`
(`every_read_allowed`). Additive: nothing renamed, nothing retyped, and no field an older peer
already reads changed.

**A Bridge that does not know the answer draws the card with three buttons**, which is the
behaviour it had before this. `helmOfferedOf` already drops an offer this build has no words for
— the rule `offeredOf` set and the reason it was set — so the new one needs nothing of an older
Bridge but that it keep ignoring what it does not know.

**The settled variant is the direction that does not survive.** A Bridge ahead of Fleet is refused
on a minor mismatch anyway; a Bridge *behind* a Fleet that sends `every_read_allowed` reads it as a
string it has no wording for, in the record and nowhere a person is blocked by it.

**Fleet decides what the set is, and the wire never carries it.** The rules written are derived
from `operations.toml`'s own `kind = "query"` rows at the moment a person presses, so a Fleet whose
inventory grew allows more than one that had not, and neither one has to tell Bridge which.

## Protocol 17.4: Fleet decides what Helm is asked about

`#1525`. One variant added to `HelmCallSettled` (`ran_unasked`). Additive: nothing renamed, nothing
retyped, no field an older peer reads changed.

**The behaviour behind it is not additive and the wire is.** Fleet now classifies every call a
person's settings did not cover and only puts three classes of them to a person — destructive,
pushes code to a shared space, writes off this machine. A Bridge that does not know `ran_unasked`
sees fewer cards and one settled value it has no wording for, in a record; nothing it draws depends
on the value.

**It is published for a call no card was drawn for**, which is the whole reason it exists. Every
other member of that union answers a card. This one is the only account of what Helm did unasked,
so leaving it off the wire would have made the audit trail quieter exactly where it matters most —
`docs/concepts/helm.md`, *Audit trail*.

**`allow_every_read` is now unreachable and stays on the wire.** 17.3 added it for a card over a
read of Armada's own door; a door read no longer draws one. Removing a variant is a major bump and
a reversal of the owner's own decision of an hour earlier, so it is his call rather than this
change's.

## Protocol 18.0: the answer that could no longer appear is gone

`#1518` shipped `allow_every_read` in 17.3, on a card over a read of Armada's own door. `#1525`
made a door read run without anybody being asked an hour later, so no card is drawn over one and
the answer had nothing to appear on. **A variant Bridge matches on, removed, is a major bump**, and
this is that bump: `HelmCallAnswer` loses `allow_every_read` and `HelmCallSettled` loses
`every_read_allowed`.

**A major is refused in either direction**, so Fleet and Bridge come up together or not at all.
That is what `scripts/restart` does in one move, and it is why a dead variant was worth removing
rather than leaving: a union member nothing can produce is a promise the system does not keep.

**The owner decided this and it was not this branch's to decide.** Removing it reverses his own
approval of an hour earlier, which `CLAUDE.md` says is his call; he was asked and said remove it.

**Nothing else moved.** `ran_unasked`, added in 17.4, stays. Minor resets to 0.

## Protocol 18.1: a server says which checkout answers it

`#1577` and `#1564`. One field added to `ServerState` — `checkout`, carrying the path the server
runs in, the branch where there is one, the commit it came up on, and how many commits have landed
in that checkout since. Additive: nothing renamed, nothing retyped, no field an older peer reads
changed.

**A new required field rather than an optional one, and it is still a minor.** What minor promises
is that nothing an older peer already reads changes; a field it does not know is one it ignores.
`ServerState` travels Fleet to Bridge only, so there is no direction in which an older Fleet is
asked to produce it.

**The three `server.*` kinds did not grow a fourth.** Each already carries the whole `ServerState`
and a reader replaces a row rather than patching it, so a row that goes behind is published again
under the kind its phase is. A fourth kind would have been a second way to say what one already
says, and every reader of the stream would have had to learn it.

## Protocol 18.2: a server can be started on a checkout that is not the main one

`#1577`. One optional field added to `StartServer` — `checkout`, a path. Additive: a caller that
sends none gets the main checkout, which is what every caller got before.

**It changes which span `${port.NAME}` resolves against, and never the number.** The body still
names a server and never a port, which is the field it has always refused; what this adds is which
of a repository's checkouts the number is drawn for.

**Ignored where `job_id` names a Job**, rather than refused as a conflict. A Job's worktree is the
Job's, and the two fields naming the same thing twice is a caller confusing itself rather than a
disagreement Fleet has to arbitrate — the path route refuses a Job's worktree by name anyway, and
says to name the Job.

## Protocol 18.3: Fleet answers no page in a browser

`#1460`. One error code added — `api.from_a_page`, a 403 on any request or WebSocket upgrade
carrying an `Origin` header. Adding a code is minor by `docs/contracts/error-contract.md`: Bridge
looks one up or falls back, so a code it has never heard of renders either way.

**No DTO, route or event kind moved**, and no caller's behaviour changes. Bridge's main process
sets a content type and nothing else, the `armada` CLI writes its own request head, the `ws` client
sends no `Origin` unless told to, and the agent's door builds its inner request rather than
forwarding one — so nothing that reaches Fleet on purpose sends the header this refuses.

**The bump is here because the code is a new answer an older Bridge can meet**, not because the
shape changed. A Bridge built before this renders it through the fallback, which is exactly what
the error contract promises.

## Protocol 18.4: a Manifest says what its two policies are set to

Two optional fields added to `ManifestSummary` — `auto_merge` and `review_gate`, the word each key
holds as `armada.yml` writes it. Additive: a Bridge built before this ignores both, and a Fleet
built before this sends neither.

**A gate that defers had no way to say what it deferred to.** A step declaring
`manifest_rule:review_gate` draws as *the repository decides*, and a reader could not tell whether
a person would be asked or nobody would — the owner, 29 September 2026, reading that row. Fleet
already resolves both policies at every gate (`crates/fleet/src/policy.rs`), so this reports a
value it computes rather than computing a new one.

**Reported, not resolved, and that is the whole of what crosses.** One Manifest's word, not the
fold across a Job's several gating Manifests, and not the answer at any particular gate: both
settings are live, so the resolution has a lifetime shorter than a read. A Bridge that rendered
this as *what will happen* rather than *what the repository says today* would be claiming the
policy cannot move, which is the one thing it can do.

**Absence is the older Fleet and nothing else.** A file that declares neither key means each
policy's own default, and Fleet sends that word rather than leaving the field out — so a missing
field says the peer is older than 18.4, and Bridge draws the deference unresolved rather than
naming a default the repository never wrote. They are `#[serde(default)] String` on Fleet's side
and `?: string` on Bridge's, which is `WorkflowSummary.source`'s spelling for the same situation.

## Protocol 19.0: a dispatched request is a row

`#1159`. `JobStatus` gains `proposing` — the interval while the Job proposer is
reading a request — and **widening that set is a major bump by this document's
own table**, so the major moves and the minor resets. The table's row is *add a
new enum variant the other side is expected to `match` on*, and the caveat row
above it does not reach this set: `held_by` and `queued_reason` are open because
Bridge renders them through the generated vocabulary without branching, and this
paragraph's own *Minor vs. major* section already names `JobStatus` as the
counter-example — **Bridge picks a screen from it**. A Bridge built before this
looks `proposing` up in `packages/components/src/generated/vocabulary.ts`, finds
nothing, and has no screen to draw for a row that is now on every Board; the
Rust deserializer is stricter still and refuses the spelling outright
(`crates/ipc/src/enums.rs`'s `wire_enum!`). A minor bump would have promised that
an older peer parses every message the same way, and that is the promise this
breaks.

**Nothing else on the wire changed.** No DTO gains or loses a field, no route is
added, and `dispatched_by` (14.2) is already how a Job the proposer split names
the Job that dispatched it — so the extras need no second relation. `ProposalId`,
`ProposalInFlight`, `proposal.moved` and `stop_proposal` are untouched: a client
still watches the *call* through them and now has a Job's row to come back to as
well.

**And Fleet does not write it yet.** `domain/job-statuses.toml` carries the row
at `in_code = "Not yet"`, beside `awaiting_approval`, `awaiting_repair` and
`awaiting_attestation`; `propose_from_request` still answers with the Jobs the
request became. So the refusal a major buys is paid before anything can send the
word — which is the right way round, because the alternative is a Fleet that can
send a status and a Bridge that connects and cannot draw it.

**The event stream is neither better nor worse for this.** No kind is added, no
payload grows, and nothing here assumes delivery is complete: a `proposing` row
reaches a client through the `job.created` kind and the resync that already carry
every other row, and the bounded drop-oldest broadcast in
`crates/api/src/stream.rs` is unchanged. When Fleet does create a Job at
dispatch, several requests sent at once will publish one `job.created` each,
which is the same one-message-per-row the approval gate already published.

## Protocol 19.1: a proposal fills in as it is written

`ProposalInFlight.settled` and the two DTOs behind it, `ProposalSettled` and
`ProposalSettings` — additive on a message that already existed, so the minor
moves. A Bridge built before this ignores the field and draws the wait exactly as
it did at 19.0.

The owner, 30 Sep 2026, having looked at a proposing Job on screen: *"I would
really push for us to find a way to make the proposer not report the job whole.
Is there anyway for it to fill in as it goes?"* Four fields, in his order —
workflow, title, done-when, settings — and
`.claude/decisions/2026-09-30-a-proposal-fills-in-as-it-is-written.md` carries the
order's reasoning, the option he turned down and the cost he took.

**The settings are one field carrying two**, because both arrive on one line of
the answer: `urgency`, and `model` — which model a Drone on this Job is spawned
as. The owner took the model over the argument that a model choosing which model
runs the work is the dial every later call's cost hangs off. **It picks from
`list_models`' own set**, and a name that set does not hold refuses the request
through `fleet.proposer_model_not_held` — **its own code, and never
`fleet.no_workflow_fits`**. That refusal's advice is to say the request again
differently, which cannot fix a model name and is about a workflow that was not
wrong; `#334` and `#410`'s rule is that two causes wanting opposite responses
must not share a word. The refusal carries the model asked for and the set this
machine runs, and Bridge mirrors the code the way it mirrors
`fleet.proposer_stopped`.
**Absent stays absent**: a call that names no model reaches `ProposeJob.model`
null, which has always meant configuration decides. Land-as-one is deliberately
not here — `crates/fleet/src/proposal.rs`'s header carries the 3 Sep 2026 ruling
that how the work lands follows from having read the code, and the owner kept it
when it was put beside the model.

**Fields that are settled, never a transcript.** Fleet reads the answer's prefix
in one place (`crates/fleet/src/proposing.rs`, `Settled::of`), and a field
crosses only once its own line has ended — so a client either has a title or has
none, and never has `Say which of the two giv`. The raw text does not cross:
`answered_characters` is still a count, and `crates/ipc/src/proposing.rs` carries
the dated correction of the rule that used to make it the whole of what a surface
could say.

**What the correction turns on is the premise, not the reasoning.** *A channel
carrying the answer as it was written would be a second, earlier, worse copy of
the Jobs it minted* was right while nothing existed until the answer landed. A
dispatched request is a Job from the press since 19.0, so a field read early is
that row becoming more complete rather than a rival to it — and the objection
still stands for the text itself, which is why nothing beside the count carries
one.

**Bridge draws each field where the Job already draws it**, through one fold
(`filled`, in `packages/screens/src/proposal.ts`): the settled workflow is the
row's Workflow column, the settled title is the row's title, the done-when lines
are the Job's criteria and the settings are its urgency. The wait inside
Overview's lead says which of the four the call has got to, because that is the
one region on a proposing Job's page whose subject is the call.

**Fleet publishes it and nothing folds it yet.** `job-statuses.toml` still reads
`in_code = "Not yet"` for `proposing`, so no Fleet creates a Job at dispatch and
`proposal.moved` names no Job for `arrivals.ts` to fold onto. The mock is what
mints the row and applies the fold; when Fleet's half lands, `arrivals.ts` calls
`filled` on the Job the message names and nothing in `packages/` changes.

**It makes the back-pressure question neither better nor worse.** No new queue,
no new channel and no per-client state: `settled` rides on `proposal.moved`, which
is already on the one bounded drop-oldest broadcast, and a dropped message costs
a reading that the next one supersedes — the field is the whole prefix re-read
rather than a delta, so a client that lost one is not missing a field, it is a
beat behind. What it does add is messages: a field settling publishes
immediately rather than waiting for `TOKEN_TICK`, which is four more messages per
one-Job answer plus one per done-when line. That is bounded by the answer's own
shape and not by the frame rate, which is the property `TOKEN_TICK` exists to
hold.

## Protocol 19.2: a worktree's size says when it was walked

One optional field added to `WorktreeOnDisk` — `measured_at`, the instant the `du` that found
`bytes` ran. Additive: a Bridge built before this ignores it, and draws the size under the
reading's age as it always did.

**Fleet now keeps a worktree's size for 30 s** rather than walking it on every read, so the size
can be up to that much older than `read_at`. `read_at` used to say every figure was as of it; it
now says every figure but this one, and Pulse draws the size's own age under it.

**Present exactly where `bytes` is.** A walk that did not finish measured nothing, so there is no
instant to put on it; an absent `measured_at` beside a present `bytes` is a Fleet older than 19.2.

## Protocol 19.3: a pasted path is a File, and a pasted image a Picture

Two node kinds a person adds by hand: a File, decided with the owner on 1 Oct 2026, and a Picture,
decided with him on 28 Sep 2026.

**A File is the path and nothing else.** `StudioNodeContent` gains `file`,
`{ "kind": "file", "path": "…" }`, and `StudioNodeByHand` gains the same shape. Kept as pasted —
absolute, under `~` or relative to the repository — and trimmed on the way in; Fleet neither
resolves it nor checks that it exists. A blank one is refused as `fleet.studio_node_blank`, and a
body with no `path`, or one that is not text, does not decode.

**A Picture is the frame and nothing else, and it is read and written in two shapes.** Read, in
`StudioNodeContent`, it is the frame Fleet kept, in a Note's `capture.frame` field names:
`{ "kind": "picture", "frame": { "filename", "byte_size", "width", "height" } }`, fetched from
`get_studio_frame` by its node exactly as a Note's is. Written, in `add_node` and `StudioNodeByHand`,
it is the PNG Bridge's main staged, `{ "kind": "picture", "staged": { "staged_path", "width",
"height" } }` — `capture_note`'s `StagedFrame` — and Fleet copies it into the Studio's own keeping
through the same path a capture's frame takes, under the same 4 MiB cap. Over the cap is
`fleet.studio_frame_too_large`, a file Fleet cannot read is `fleet.studio_frame_unreadable`, and
from Helm a Picture is `fleet.studio_node_not_helms`, each before anything is written. Deleting the
node deletes the file.

**A write never names a kept frame.** A kept frame is a file name Fleet chose, and
`get_studio_frame` opens what a node names, so an `add_node` body with `"kind": "picture"` decodes
as the staged shape alone: one naming `frame`, beside `staged` or instead of it, or naming nothing
staged, does not decode. On Fleet's side the request is `ipc::StudioNodeAdded`, whose other variant
holds an `AddedContent` that cannot be a Picture; TypeScript's `AddStudioNode` says the same with
`Exclude`. Only Bridge's main builds the staged shape, from bytes it staged itself — the renderer
never names a path.

**Nor does a Note's `capture.frame`, which closed a hole older than the Picture.** A Note through
`add_node` could carry any file name there, and `get_studio_frame` joined it onto the Studio's
directory. Such a body no longer decodes; a captured Note's frame arrives staged, through
`capture_note`, as it always has, and no Bridge sent one the other way. Behind the record, Fleet
also refuses to open or delete a kept name that is not one plain path component — a row holding
`../x` reads as `fleet.studio_frame_unreadable` and is skipped by a delete.

**Additive by 14.7's and 14.18's reading**, which added node kinds the same way. Nothing an older
Bridge already parses changes; one meeting a File or a Picture draws no card for it, which is
`packages/screens`' `cardOf` default for a kind it does not know.

## Protocol 20.0: a Sketch is the pad's drawing

Decided with the owner on 1 Oct 2026: a Studio's Sketch and the dispatch composer's Sketch pad are
one drawing. `.claude/decisions/2026-10-01-a-sketch-is-the-pad.md`.

**Major, because a field an older Bridge reads is gone.** `StudioNodeContent`'s `sketch` was
`{ "kind": "sketch", "body": "…" }`, the diagram written as text, and is now
`{ "kind": "sketch", "drawing": { "boxes", "joins", "strokes", "pictures" } }`. A Bridge built
before this reads `body` off every Sketch on a Studio and finds nothing, which is the table's
*field removed* row. Keeping a derived `body` beside the drawing would have bought a minor at the
cost of a field nothing new reads, and Armada has no Bridge in the field to keep it for.

**The drawing is the pad's own four parts**, in the pad's coordinates as whole numbers: a box is
`{ id, x, y, body }`, a join `{ id, from, to }` naming a box or a picture, a stroke
`{ id, points: [{ x, y }] }`, and a picture `{ id, x, y, width, height, frame }`, where `width` and
`height` are the size it is drawn at and `frame` is the file Fleet kept, in a Note's
`capture.frame` shape. Every array is sent, empty or not. **It is checked on decode**, by
`core_model::Drawing::drawn`: an id on two parts, a blank id, a join to nothing or to itself, a
stroke of fewer than two points and a picture at no size do not decode, and nor does an unknown
field anywhere in it. Fleet's store holds a row to the same rule on read. Every Sketch written as
text is migrated by store V85 to one box, `b1` at the origin, holding the old body. **A Sketch
carries no `state`**, where it carried `frozen`: it is drawn on whenever its pad is opened, and V85
clears the state on every Sketch row. Finding's and Outline's `frozen` went in 21.0.

**Written, a picture is its staged file or nothing**, the way a Picture is written (19.3). `add_node`
takes `{ "kind": "sketch", "drawing": … }` with each picture carrying `staged`, decoded apart from
every other kind as a Picture is; Fleet copies each into the Studio's keeping under a name it mints,
`<node>-<ulid>.png`, under the 4 MiB cap. **`edit_studio_sketch`**, new and `Bridge only`,
`POST /studios/:studio_id/edit_sketch` with `{ node_id, drawing }`, replaces the whole drawing; a
picture carrying nothing keeps the frame the node already holds under that id, and one it holds
none under is `fleet.studio_sketch_picture_not_kept`. A picture naming `frame` in a write does not
decode, so no client names a file Fleet opens. Frames the new drawing no longer names are deleted
once it is written. A drawing with nothing on it is `fleet.studio_node_blank`, and a redraw of any
other kind `fleet.studio_not_a_sketch`.

**`get_studio_frame` takes `?picture=`**, the picture's id on a Sketch, read off that picture's own
`frame` — the record is still the allowlist. Deleting a Sketch deletes every frame it kept.

## Protocol 21.0: no Studio node is `frozen`

Decided with the owner on 1 Oct 2026, asked about a Finding's and an Outline's `frozen` after a
Sketch lost it in 20.0: *"I hate this frozen shit. Its overcomplicating it,"* and then **"Remove it
everywhere"**. `.claude/decisions/2026-10-01-no-studio-node-is-frozen.md`.

**Major, because a state value a client reads is gone.** `StudioNodeState` loses `frozen`, which is
the table's *variant the other side matches on* row: a body naming it no longer decodes, and a
Bridge built before this would draw a Finding with no state where it drew `frozen`.

**A Finding whose scout has ended carries no `state`.** How it ended is already `ended` on its
content, and `gathering` would say a scout still reads — Bridge pulses one, and a restart settles it
as failed. A Finding without `state` is always one with `ended`; Fleet's store refuses any other on
read. **An Outline is `draft`**, its only state. Store V86 moved every frozen Finding to no state and
every frozen Outline to `draft`, and narrowed the column's `CHECK` so neither comes back. Minor
resets to 0.

## Protocol 21.1: a workflow says what requests it is for

Decided with the owner on 1 Oct 2026: the approval screen shows the promise the proposer picked a
workflow on. One optional field added to `WorkflowSummary`, `for_requests` — the definition's own
`for_requests` line, the sentence saying what kind of request the workflow is for. Additive by
18.4's reading: a Bridge built before this ignores it, and a Fleet built before this sends none.

**Absent is a definition that declares no line**, and a Fleet older than 21.1; the two read the
same, because either way there is no promise to show. It is `Option<String>` left out when empty,
and `?: string` on Bridge's side. `crates/fleet/src/wire.rs`'s `workflow_summary` is the one
builder.

**It is served on the workflow list and nowhere else.** `JobDetail` carries the Job's frozen
workflow, which `core-model` holds without the line, so Bridge reads it off `GET /workflows` by the
Job's `workflow_id` and `manifest_id`. The list is the catalogue as it stands now, not the
definition the Job froze; a line edited since dispatch reads as the new one.

## Protocol 21.2: a Job's log files say what they weigh and whether they are being written

One optional field added to `JobResources`, `logs` — every log file the Job has: its own log,
each Drone's transcript, and each kept Judge brief, as a `LogFile` carrying `kind` (`job`,
`transcript`, `brief`), `path` relative to `records_root`, `bytes` and `being_written`. Additive
by 18.4's reading: a Bridge built before this ignores it, and a Fleet built before this sends
none, which reads the same as a Job with no file yet.

**`being_written` is an open file with write access, never an mtime** (#1648). Fleet asks `lsof`
about the Job's process tree and about Fleet's own pid, because Fleet is the writer: it appends a
Drone's transcript and the Job's log from its own process for as long as that Drone's stdout is
open. Asking the tree alone would read `false` on every live Drone.

**Absent is not zero, and not `false`.** `bytes` is left out for a file Fleet listed and could
not `stat`; `being_written` is left out where `lsof` did not answer. There is no owner on a row:
every one is the Job's own until a sub job exists to name.

## Protocol 21.3: a Job's Drones, the exited ones too

One route, `GET /jobs/:job_id/drones` (`list_job_drones`), answering `JobDrones`: every Drone the
Job has had, each with its step, its state (`running`, `done`, `failed`, `killed`), when it was
spawned and when it left, and its turns and cost. Additive: a new DTO on a new route.

**`GET /drones` is the roster and loses a Drone the moment it exits.** This reads the Job's own
history and the per-Drone spend rows instead, so a stopped Drone stays. The state rule, and why
`killed` is told apart from `failed`, is the operation's note in `crates/ipc/operations.toml`.

**`ended_at`, `turns` and `cost_micros` are left out where there is nothing**, never nought. A
running Drone has no `ended_at`, and one still in its first invocation has no terminating line yet,
so no turns and no cost. A running Drone's figures come off its transcript and trail it; a stopped
one's are the row the Job's spend is summed from.

There is no task on a row. Fleet runs one Drone per step and nothing joins a Drone to a plan task.

## Protocol 21.4: a row names its Drone, and says how much it thought

Two additions to a transcript row, on `observe_job` and on `get_drone`'s turns (#1662, #1664).

**`drone_id`, beside `step` and `by`**: the Drone whose transcript the row is in, the id
`list_job_drones` names. A Job with several Drones streams one interleaved history, and nothing
told their rows apart. A field rather than a socket per Drone, because the socket's point is one
order on one clock, and a viewer of one Drone filters. Fleet stamps it from the file's name,
`<drone-id>.jsonl`, so nothing is written per line and an older file stamps as fully as a new one.
Absent is a Fleet before 21.4, and a Helm thread's rows, which no Drone's transcript holds.

**`thinking`, a new `Saw` kind**, carrying `estimated_tokens`: the harness's estimate, cumulative
within one model call. It arrived before as `unrecognised` with the kind `system/thinking_tokens`
and the figure dropped. How much, never what: the reasoning text stays off the wire.

**Minor, by the 7.8 precedent, where `background_work` was named out of `unrecognised` the same
way.** Every Bridge reader of `Saw.event` has a fallback arm and nothing checks the union for
exhaustiveness, so an older Bridge reads the kind it has no case for rather than failing. That
Bridge does draw it wrong in one place: `story.ts`'s fallback is the unreadable row, so a
`thinking` row reads there as *a line the reader could not parse*, under the Fleet-ahead banner.
This Bridge reads `thinking` in the log and the working view exactly as it read the unrecognised
row before. Drawing the estimate on the Drones tab is still to do.

## Protocol 21.5: kill one process of a Job, or every one

Two routes, both answering the Job's `JobSummary`: `POST /jobs/:job_id/processes/:pid/kill`
(`kill_process`) and `POST /jobs/:job_id/processes/kill` (`kill_processes`). Additive: new routes,
no new DTO. Bridge sent both before Fleet served them, and `pending.ts` no longer lists them.

**The pid is a name, not a grant** (#1647). Fleet reads the Job's process tree again at the act and
refuses a pid outside it with `fleet.not_the_jobs_process`, a 409 carrying `pid`. A row that went
stale between the reading and the press meets the same refusal, and the answer is to read again.

**A child is not the Drone.** Killing one ends it and what it started and moves nothing on the
record; killing the Drone's own pid is `kill_drone`, and so is killing every process, plus the
descendants that left the Drone's process group. The operations' notes in
`crates/ipc/operations.toml` carry the whole rule.

## Protocol 21.6: a dispatched request is a Job, and the proposal names it

`ProposalMoved.job_id` names the Job at `proposing` the call is reading for. It is absent only from
an older Fleet, which created no Job until the call answered. Two escalation reasons are new on
`Reason.named`: `no_workflow_fits` and `proposer_failed`, both on `proposing -> escalated`. All of
it is additive.

**The row exists from the press** (#1714). `propose_from_request` creates the Job at `proposing`
before the call goes out, with the request as its title, `workflow_id` empty, no steps, and
`workflow_source` blank. The route still answers the plan or the refusal code. On a refusal the
Job is now escalated rather than never created: `fleet.no_workflow_fits` takes `no_workflow_fits`.
`fleet.proposer_unreachable`, `fleet.proposer_unreadable` and `fleet.proposer_model_not_held` take
`proposer_failed`, and the Job's log says which (#1716). A stop takes the Job to `killed`.

**The head of the plan is that Job, moved to the gate**, with the proposer's title replacing the
request. Its `job.state_changed` is the move, not a `job.created`. The other Jobs of a split arrive
as `job.created` with `dispatched_by` naming the head.

## Protocol 21.7: a merge refused because the base moved

`merge_pull_request` gains one refusal, `fleet.merge_base_moved`, a 409 carrying `refused:
base_moved`. Only a repository whose Manifest says `merge_by: push` meets it: Fleet makes the
merge commit itself and refuses a branch that does not hold the base it would land on, which the
forge would have merged. No shape moves.

**Minor because a refusal code added is additive**, for 14.14's reason.

## Protocol 21.8: a merge refused because its Checks went red on the moved base

`merge_pull_request` gains one refusal, `fleet.merge_gate_failed`, a 409 carrying `refused:
gate_failed`. Under `merge_by: push`, Fleet now answers a moved base by merging it into the Job's
branch and running the Job's Checks again before pushing, and this is that run going red. A
conflict on the way is the `fleet.merge_conflicted` that already existed, and `fleet.merge_base_moved`
now means the base kept moving through every round. No shape moves.

**Minor because a refusal code added is additive**, for 14.14's reason.

## Protocol 21.9: a step's run says what its gate resolved the policies to

One optional field on `StepAttempt`, `resolved`, holding `auto_merge` and `review_gate` as
`armada.yml` writes them. It is set on a run whose ruling read the advance gate (advanced,
finished, or held for review) and absent on every other. Additive, on 18.4's argument: an older
Bridge ignores the key, and an older Fleet never sends it.

**History, beside 18.4's live reading** (#1683). `ManifestSummary`'s two words say what the
repository says now. This says what one gate acted on, written to `job_step_policies` on that run
before the step moved. A Manifest edited afterwards changes the next run's value and leaves this
one as it was. That is what lets the Record say a step held because the repository said
`human_always` at the time.

**Both policies, on every gate that read them.** The owner decided this on 1 Oct 2026: not only the
one that gated. One object rather than two optional words, because they are written together and
neither can be there without the other.

**Absent means nothing was recorded, never a default.** Four causes give the same reading: a run
still going, a run stopped before the gate (a Check failed, the Judge refused, a gaming flag
stood), a run from before 21.9, or an older Fleet. None of them is a resolution, so Bridge draws
no policy rather than `never` or `human_always`. It is `#[serde(default, skip_serializing_if)]
Option` on Fleet's side and `?:` on Bridge's.

## Protocol 21.10: every run that reached a gate keeps its policies, and says whether they decided

`StepAttempt.resolved` gains `decided: boolean`, and is now set on every run a gate stopped as well
as on one that reached the advance gate. Additive, on 21.9's argument: an older Bridge ignores the
key, and an older Fleet never sends it.

**The owner's decision of 2 Oct 2026** (#1683): a run whose Checks failed, whose Judge refused, or
whose gaming check flagged it keeps what `auto_merge` and `review_gate` said at that moment too. His
reason: "Isn't it helpful to record them so we know what ran?" 21.9 kept them only where the
advance gate was read.

| Run | `resolved` | `decided` |
|---|---|---|
| Held for review, advanced or finished | set | `true` |
| Checks red (handed back or failed), Judge refused, Judge asking a person, gaming flagged, gate could not decide | set | `false` |
| Submission of the wrong kind, still going, from before 21.9, or an older Fleet | absent | — |

**`decided` is read off the ruling's variant in `fleet`, never carried**, so a run cannot be
recorded as decided when it stopped before the rule. The two words still say what the rules were
at the gate the run stopped at; `decided: false` says the rule never answered for it.

**A 21.9 payload has `resolved` without `decided`, and reads `true`.** 21.9 kept only runs that
reached the advance gate, which is also why V89's column defaults to `1` for every row V87 wrote.
Absent `resolved` still means nothing was recorded, never a default.

## Protocol 21.11: a kept brief is read back

One route and one DTO: `GET /jobs/:job_id/briefs/:name` (`get_brief`) answers `BriefContents`,
which is `path`, `lines`, `from_line`, `total_lines`, `bytes` and `whole`. Additive. It is what Pulse's log
panel reads when a person presses a brief row, so a Judge's or a gaming check's brief opens inside
Bridge the way a transcript does (the owner's decision, 2 Oct 2026).

**`:name` is the last part of a `brief_path`**, the same file `JobResources.logs` lists as kind
`brief`. Fleet resolves it inside that Job's briefs directory. A name that would leave it (`..`, a
separator, another Job's brief, a link out) gets the 422 a name it never kept gets. A missing Job
is a 404.

**The window is `get_check_output`'s**: the tail, 2,000 lines and 256 KiB. A brief ends on what
it asks, after the diff, so a cut brief keeps its question. `whole` is stated, never inferred.

## Protocol 22.0: a task in between, a task that failed, and the Record's two new signers

Spike 022, the wire lock for the new Job, signed off by the owner on 2 Oct 2026, slice 1a (#1760).

**Major, three times over, and bundled so the milestone takes one.** A major is the lifeboat for
a Fleet caught mid-Job, so the spike's *protocol changes, bundled* section puts every breaking
change of the milestone here rather than one per slice.

| Change | Why it breaks |
|---|---|
| `TaskState` gains `handed_in` | A strict set Bridge matches on: an older Bridge draws a handed-in task as `open` |
| `TaskState` gains `failed` | The same set, the same reason |
| `Actor` gains `judge` and `check` | A strict set stored on every recorded row; an older peer reads neither |
| `TaskCounts` gains optional `handed_in` and `failed` | Additive on its own, absent at zero; it rides the major |

**`handed_in` is the owner's answer 1**: a task's agent has handed its work in and the step's Checks
have not answered, so done arrives at green. **Nothing writes it or `failed` at 22.0**: slice 1b
writes the first at a task Drone's hand-in, slice 2 the second when a group's Checks go red, and
`update_task` refuses both, because both are Fleet's to mark.

**`judge` and `check` sign the rows their own answer wrote.** A refusal's step stop and escalation
are signed `judge`, as are a Judge asking a person and a gaming flag; a failed Check's stop and
hold, and a hand-back's `retrying`, are signed `check`. Fleet signs every move it decided on more
than one answer: an advance, a policy hold, a gate that could not decide. `fleet::Ruling::signed_by`
is the one place that says which. `human` stays `human`; spike 020's rename to `person` is not
needed, because Bridge says *you*.

**The figure.** Done over every count but `dropped`: a handed-in task and a failed one join the
total and neither joins `done`.

**Store V90 moves `KNOWN_SCHEMA_VERSION`**, so a Fleet built before this refuses the store at open
rather than failing to fold a Job signed `judge`. It also rebuilds the plan's `state` check to admit
the two new states. Minor resets to 0, and a Bridge and a Fleet must both be rebuilt from the same
commit.

## Protocol 22.1: the merge line, read off disk

One route, one event kind and four DTOs: `GET /merge_lines` (`get_merge_lines`) answers
`MergeLines`, and `merge_lines.changed` carries the same body whole. Each `MergeLine` is one served
repository's `root`, its `line` in place order and the newest three `off` it, as `MergeLineEntry`
rows with `state` a `LandState`. Additive. `docs/capabilities/merge-line.md`, *In Bridge*, has
the field-by-field table.

**A fleet-wide fact that persists, so a route and an event**, `manifest.reread`'s rule. What is
new is the writer: `armada land` is another process, so nothing tells Fleet a line moved. Fleet
reads every served repository's `armada-land/` every two seconds and publishes only when the answer
changed. Bridge reads the route once per connection and keeps no timer.

**`LandState` is a strict enum, not an open set.** Bridge branches on it: the two live states
pulse and each end state draws its own facts. A new state is a major move.

**On the unmeasured risk above: neutral.** It adds no queue. It publishes at most once a read and
only on a change, so a quiet line costs the shared backlog nothing. A busy turn costs a handful of
events a minute.

## Protocol 23.0: a node sits inside a Zone or a Cluster

Decided with the owner on 2 Oct 2026: everything a read-in brings back lands inside one Zone, and a
Cluster is a frame round its Notes. `.claude/decisions/2026-10-02-a-read-in-lands-in-a-zone.md`,
#1620.

**Major, because a position an older Bridge reads changed meaning.** `StudioNode` gains `within`,
the id of the frame it sits in, absent on the board; a node with one has its `position` measured
from that frame's corner. A Bridge built before this draws every such node at its offset from the
board's origin, which is the table's *field that parses the same and means something else*.
`StudioNodeContent` gains `{ "kind": "zone" }`, nothing beside the tag, which on its own would be
14.18's additive kind.

**`move_studio_node` takes `within`**, the frame a node was put down in, and absent is the board.
A frame that does not hold the kind is `fleet.studio_frame_cannot_hold`, and a Note moved out of
its Cluster `fleet.studio_note_stays_in_its_cluster`. **`add_studio_node` takes a Zone from
Bridge**, empty. **`group_studio_nodes` draws a Cluster round its Notes** and sets each Note's
`within`, wherever `position` said; a Note already in a Cluster is `fleet.studio_note_in_a_cluster`.

**A read-in's nodes arrive with `within` set**: its Zone, then its Finding inside it, and each
Cluster's Notes inside the Cluster. The source still produces every one of them, the Zone among
them, so `edges` is what it was plus one. Which of those edges Bridge draws is Bridge's
(`docs/concepts/studio.md`, *Edges*). Store V91 adds the column, and every node before it sits on
the board. Minor resets to 0.

## Protocol 23.1: a Drone per task, and the task a change moved

Spike 022, the wire lock for the new Job, slice 1b (#1762, carrying #1752).

**Four optional fields on three bodies, all additive.** `JobDrone` gains `task`, the plan task a
Drone was put on, absent on a Drone that worked its whole step. `JobPlanChanged` gains `task` and
`state`, the task a change moved and where it now stands, both absent on a whole recording. Spike
022's *why a transition rides `job.plan_changed`* is the reason the event carries them rather than
a new kind. `StepDetail` gains `drone_per_task`, absent at false, because the spike has Bridge
read the step key where it derived the step that works the tasks; the spike's wire row for 1b
names the first three and not this one.

**Fleet is a third author of `job.plan_changed`.** On a step declaring `drone_per_task`, Fleet
marks each task itself, `actor` `fleet`: `working` when its Drone is spawned, `handed_in` at that
Drone's `submit_evidence`, and `done` once the step's Checks pass. `update_task` still refuses
`handed_in`, and a task's Drone is not offered it at all. `docs/concepts/plan.md`, *A Drone per
task*.

**On the unmeasured risk above: worse, by a counted amount.** A task adds three
`job.plan_changed` and a `drone.spawned` and `drone.exited` pair, against one pair per step
before. Measured on a dev Fleet on 2 Oct 2026, with a scripted agent handing in two seconds after
each spawn: five events per task, and 43 in the minute that held a five-task Job's whole run,
against a `BACKLOG` of 256. A real task takes minutes, so the rate per Job-minute is five over
how long a task takes. #1759 has the line; `[broadcast-capacity]` stays open.

**Store V92** keeps which Drone was put on which task, and what each handed in, so `JobDrone.task`
and the step's one submission survive a Fleet restarting mid-step.

## Protocol 23.2: what landed, and what was sent back

Decided with the owner on 2 Oct 2026: the merge line's one list of what left it splits in two.
`MergeLine` gains `landed`, the newest `landed` outcomes up to `LANDED`, and `sent_back`, every
`red`, `conflict` or `stopped` outcome of a branch not in line written within `SENT_BACK_FOR`. Both
bounds are in `adapters::land_state::line`. Both lists are `MergeLineEntry` rows, newest first, with
the redaction `off` has. `docs/capabilities/merge-line.md`, *In Bridge*.

**Additive, so the minor moves.** `off` is still served as it was, so a 23.1 Bridge connects behind
the banner and draws what it drew. This Bridge does not read it. `off` could not carry the split by
itself: its newest few of either means a run of landings pushes every red out of it.

**The bound is the outcome file's own age, held against the instant Fleet's clock gives the read.**
A red that ages out changes the answer, so `merge_lines.changed` publishes it.

**`MergeLineEntry` gains `checks`**, each Check the turn runs as `MergeLineCheck { name, state }`,
`state` a strict `LandCheckState`: `waiting`, `running`, `passed`, `failed`, `timed_out`. The
runner writes the same list into the outcome file (`Outcome::checks`, through `OutcomePatch`) as
each Check starts and ends. Served for `gating`, `red` and `stopped` only, and while a Check
runs `doing` is left off: the list says it. Absent where empty, so additive like the rest.

**`MergeLinePullRequest` gains `settled`**, the Job's own `Settled`: `merged` where the forge
read the push as the merge, `closed_unmerged` where the runner closed it naming the merge or found
it closed. The runner records it as `Outcome::pr_settled` when the branch lands, from `gh pr view`'s
state; Fleet serves it for `landed` only. Absent is nothing known: a pull request still in line,
one left open because the remote held more than landed, or a forge that would not answer.

## Protocol 23.3: the files a fix holds off the Jobs that hit its test

#1673, the Fleet half.

**One optional field, additive.** `ClaimedBreakage` gains `held_off`, the files no Job on the claim
but the fix may change while it stands: those the reporting Drone named in `draft_fix`, then those
the fix has declared it will change. Absent is none, which is every claim from before 23.3 whose
fix has declared nothing yet. It reads the same from either side of the claim, like the rest of
the entry.

**Bridge reads `whole.breakages` for everything else.** Which Job is the fix, its title, and which
Check and test it is fixing were already there: `fix`, `fix_title`, `check`, `test`. A failed Check
row matches a breakage by `check`.

**A landed fix leaves `whole.breakages` at the merge, as before**, though its files stay held off
each Job until that Job's next catch-up brings the fix in. That hold is not on the wire; the Drone
is told, and `docs/concepts/fleet.md`, *A test another Job is fixing*, has why it outlives the merge.

**Not on the wire: `draft_fix` gains a required `files`.** It is an MCP tool, not this protocol, and
its own schema says so. **Store V94** keeps a claim's files and what a landed fix still holds.

## Protocol 23.4: groups, a task that failed, and two acts on them

Spike 022, the wire lock for the new Job, slice 2 (#1763, carrying #1652, #1656 and #1685).

**Optional fields, one new DTO pair and two routes, all additive.** `WorkPlan` gains `groups`,
each a `PlanGroup` naming its tasks, its `state` (`GroupState`, the registry's eight words) and
every run as a `PlanGroupRun`: its own number, the step's run it was filed under, its verdict and
the commit a green run made. `PlanTask` gains `group` and `failed_reason`, `reason` staying a
drop's alone. `CheckRun` and `Recorded` gain `group` and `group_attempt`, absent where no group's
gate made the row, which means exactly that and never "unknown" (#1652). `JobPlanChanged` gains
`group`, on the change a group's verdict made.

**Fleet writes `failed`, and only once a group's retries run out** (answer 9). A red group goes
round on its own, its tasks staying `handed_in`; the last red run fails every task in the group
with a reason naming the group and the run. A Judge refusal stops the group for a person, as it
stops a step (answer 3). `docs/concepts/plan.md`, *Groups*.

**Restart this task also answers a done task in a group the Judge refused** (2 Oct 2026), with
no change to the wire: Bridge reads the group's last run, `verdict.trigger` `gate_failure` over
tasks still `done`, as Fleet does.

| Route | Body | Answers | Refused |
|---|---|---|---|
| `POST /jobs/:job_id/tasks/:task_id/restart`, `restart_task` | `RestartTask`, an optional `note`; no body is valid | `JobSummary` | 409 `fleet.task_not_failed` on a task that has not failed, except a done one in a group the Judge refused |
| `POST /jobs/:job_id/plan/move`, `move_plan` | `MovePlan`: `group`, `task?`, `after?` | `WorkPlan` | 409 `fleet.task_in_flight` on a task, or a group holding one, still in its run; 422 `fleet.no_such_group`, `fleet.no_such_task` |

**The bodies are the lock's.** Bridge sent `to`, an index; the wire takes `after`, as `add_task`
places a task, because an index counted at the drag is stale the moment a Drone adds or drops one.
`MovePlan` moved from `pending.ts` to `work-plan.ts` with that shape. Bridge's two pending entries
go with the Bridge half, which draws the groups and answers both acts in the mock.

**`record_plan` takes an optional `group` per task**, a number in the order groups run; a task
naming none joins the group before it, so a plan recorded without groups is one group, which is
how every plan before this reads.

**Store V95** keeps each group's runs, the group and run in `job_step_checks`' key so two groups
gated on one run of a step keep both their rows, and a plan's two moves and a failed task's reason.

## Protocol 23.5: a pull request's title and comment count, after it merges

**Two optional fields on `JobDelivery`, additive.** `pull_request_title` and `pull_request_comments`
sit beside `pull_request` and are served only where it is. Unlike `pull_request_detail`, which is the
sweep's live reading and goes away when the pull request settles, both come off the Job's record, so
a merged Job still has them.

**`pull_request_title`** is written when Fleet opens the pull request, from the title it opened it
with, and again on every read of the forge, the settling read included, so a title edited on the
forge replaces it. Absent means no read has named it: a pull request opened before 23.5 and not read
since.

**`pull_request_comments`** is the count the sweep's read finds while the pull request is open:
conversation comments plus reviews that say something. A comment on one line of the diff is not
counted, because that is the second query only `get_remarks` asks. **Absent is unknown, never 0**:
the pull request settled before the rotation reached it open. Opening a new pull request clears it.

**No forge call on a Job read.** Both are written on reads Fleet already makes. **Store V96** keeps
them, in two `jobs` columns.

## Protocol 23.6: one Check's log on a merge line

The owner, 2 Oct 2026: a Check in the Checks strip opens its log, live, wherever the strip is
drawn. A plan group's Checks already had both halves, `observe_check_output` while the gate writes
a log and `get_check_output` once it has ruled. A merge line's had neither: the runner writes each
Check to `logs/<entry>/<turn>/<check>.log` under `armada-land/`, and `get_merge_lines` keeps the
outcome's `logs` off the wire.

**A route, and a socket of its own.** `observe_land_check` is `GET
/merge_lines/checks/observe?root=&branch=&check=`, an upgrade like `observe_check_output`'s,
answered before the socket opens. It reads the file the way that one does, a quarter-second behind,
and ends with `closed`. **An ended Check is served on it too**: it opens, sends what the file holds
and closes `finished`, so Bridge reads a merge line Check through one socket whether it is running
or not.

**The three names are the request, and never a path.** Fleet holds the root against the roots it
serves, reads the branch's outcome for its turn and the Checks that turn has started, and opens only
`<turn>/<check>.log` where the turn resolves under this line's own `logs/` and the log is not a
link. Everything else is one refusal. `adapters::land_state::line::check_log` is the rule;
`crates/fleet/src/tests/land_logs.rs` holds it to a path, `..`, a waiting Check, an unserved root,
a turn outside `logs/` and a link.

**Additive, so the minor moves.** `LandOutputMessage` is a new message family, `OutputMessage`'s
three with its own opening: `LandOutputOpened` names the root, the branch and the Check, and carries
no path, for `get_merge_lines`' redaction. `OutputLines` and `OutputClosed` are reused whole. A
23.5 Fleet has no such route, so a 23.6 Bridge behind it is refused, which is the skew rule's own
direction.

**Bridge holds it in main**, `land-following.ts` beside `following.ts`, published as
`BridgeState.landFollowed`, one at a time. The preload's `followLandCheck` takes the three names
and main reads only three strings off whatever the window sent.

## Open questions

Naming these rather than deciding them, per this document's brief:

- **[protocol-codegen]** What generates the TypeScript from `ipc`. Hand-rolled build script,
  `ts-rs`, `specta`, something else — not decided. Whatever it is, it must not
  reach `core-model` or `adapter-traits` (their `cargo tree` is a gate rule:
  no codegen framework belongs under either).
- **[verify-protocol-task]** What checks the rest of the generated half. The
  version pair is held by a `verify-foundations` rule, which is the part that
  shipped broken; the DTO types are generated by nothing, so there is no
  candidate output to compare the checked-in ones against, and no rule refuses
  a version literal spelled outside the generated file. Whether the remainder
  is a rule in `verify-foundations` or a `verify-protocol` task of its own
  follows from what `[protocol-codegen]` decides, and neither is decided.
- **[broadcast-capacity]** The bounded broadcast channel's capacity, and whether it's one number
  for all event types or tuned per event type.
- **[lifeboat-router]** Whether the lifeboat's four routes live inside the
  same `axum` `Router` as the main protocol or a separate one. Either can satisfy "no shared
  dependency with the versioned protocol"; which one hasn't been decided.
