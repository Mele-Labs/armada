# Protocol: the Fleet/Bridge seam

Armada has exactly one place where Rust stops and TypeScript starts: the wire
between Fleet (the daemon, Rust) and Bridge (the desktop app, Electron). Every
other boundary in the system is a function call or a file. This one is a
process boundary between two binaries with independent release cadence and
independent lifetimes, and everything in this document exists because that
combination has already gone wrong once, in v1, and cost real debugging time
figuring out which side was lying.

If your change touches `crates/ipc`, `packages/protocol`, the route table in
`crates/api/src/routes/`, anything under `apps/desktop/src/preload`, or the
WebSocket event stream, read this first.

## The protocol ID

**There is no version number to bump.** Every build carries a protocol ID, a
16-digit hash of the wire surface, taken at build time. Two branches that each
change the wire change different files, so they never conflict on a number, and
the ID follows whichever files they touched.

**Equal IDs connect. Anything else is refused.** Bridge opens no socket, and the
`/v0` lifeboat is the fallback. There is no order between two IDs: a mismatch
says the sides differ, not which one is older.

### What it hashes

| Source | Why it is wire |
|---|---|
| `crates/ipc/src/**/*.rs`, except `tests/` | The DTOs, and every other type Fleet serializes |
| `crates/ipc/operations/*.toml`, except `_header.toml` | Which operations and event kinds exist |
| `crates/api/src/routes/**/*.rs` | The method and path each operation is served on |
| `packages/protocol/src/**/*.ts`, except `*.test.ts` and `connection.ts` | Bridge's hand-written mirror of the DTOs |

Files are taken in path order. Within one, a line that is only a comment is
dropped and trailing whitespace is trimmed, so editing prose moves nothing. The
bytes then go through FNV-1a with 64 bits. The hash is the same on every
machine and every checkout, because it reads paths relative to the repository
root and never a commit hash.

### What it cannot see

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
- **It cannot tell that Bridge's mirror was updated.** The DTO types in
  `packages/protocol` are written by hand. Changing a Rust DTO moves the ID
  whether or not its mirror was touched, so equal IDs prove two builds read the
  same wire files, not that the mirror is right. That gap is `[protocol-codegen]`
  below.

### Where each side gets it

- **Rust.** `crates/ipc/build.rs` hashes the files on every build that touched
  them and embeds `ProtocolId::current()`. The hashing is
  `crates/ipc/wire_hash.rs`, a file `build.rs` includes.
- **TypeScript.** `apps/desktop/codegen/protocol-id.mjs` takes the same hash and
  `apps/desktop/electron.vite.config.ts` injects it into the main process, the
  preload and the renderer as `__PROTOCOL_ID__`. `PROTOCOL_ID` in
  `packages/protocol/src/connection.ts` reads it. **It is not a checked-in
  file**: a literal in git is a line every branch that touches the wire edits,
  and two of them conflict, which is the problem the ID exists to end. Under a
  test runner nothing injects it and `PROTOCOL_ID` reads `unbuilt`, which no Fleet
  speaks.
- **They are two implementations of one hash**, written twice because the build
  script and the Vite config cannot share code. `cargo xtask verify-foundations`
  runs both on the working tree and fails when they disagree
  (`xtask/src/rules_protocol/id.rs`), and the Rust half it runs is the file
  `build.rs` includes.

### The ID on the wire

It crosses as one string field, `protocol_id`, on the runtime file and on each
socket's opening message. The runtime file from a Fleet that predates IDs has no
such field and reads as an ID nothing equals, so an old Fleet reaches the
mismatch screen instead of reading as a file nothing wrote.

### Contributor workflow

1. Change the DTOs in `crates/ipc`, and mirror them in `packages/protocol` by
   hand.
2. Add or change the operation in `crates/ipc/operations/` and the route in
   `crates/api/src/routes/served/`.
3. Nothing else. There is no number to choose, no need to declare and no
   generated file to commit, so a branch that changes the wire never conflicts
   with another on account of it.
4. Run `cargo xtask verify-foundations` and read the line naming the protocol.
   The rest of its output is about other rules.

## DTOs, not domain types

`WireError` is a DTO like any other, and `docs/contracts/error-contract.md` is
what specifies it —
which fields are guaranteed, why `level` and `component` are not among them,
and why removing an error code moves the protocol ID like any wire change. The v0 lifeboat below is
deliberately outside that contract.

`ipc` speaks its own vocabulary. It does not re-export a `core_model` type and
put it on the wire. The conversion is explicit, one direction, and lives at
the Fleet boundary:

```rust
// crates/ipc/src/commanding.rs — the DTO. Only what Bridge is allowed to see.
pub struct AllowedCommandRow {
    pub run: String,
    pub allowed_at: Instant,
    pub by: Actor,
}

// The conversion, and the only place it happens.
impl From<&core_model::AllowedCommand> for AllowedCommandRow {
    fn from(allowed: &core_model::AllowedCommand) -> AllowedCommandRow {
        AllowedCommandRow {
            run: allowed.run.clone(),
            allowed_at: (&allowed.allowed_at).into(),
            by: allowed.by.into(),
        }
    }
}
```

The reason this conversion has to exist, rather than serializing the domain
type directly, isn't code cleanliness — it's that `From` is where someone has
to decide, field by field, what a Bridge (running on someone's laptop,
potentially screen-shared, potentially logged) is allowed to see. A domain type
will accrete fields as Fleet's needs grow: filesystem paths, adapter tokens,
internal retry state. If that type is `#[derive(Serialize)]` and put straight on
the wire, every new field is redacted or not by accident — whichever `serde`
does by default. A `From` impl forces a human to look at the new field and write
a line of code, one way or the other. A domain type on the wire is a redaction
decision nobody made.

This cuts the other way too: `ipc` types have no business back in
`core-model`. If Fleet-side code needs an `AllowedCommandRow` to build a
response, that's an argument for a thin builder function, not for teaching
`core-model` about the wire's shape.

## Changes that read like they break nothing

A refusal on any ID mismatch means the question "is this change safe for an
older peer" no longer decides anything: every wire change is refused by a peer
built before it. The mistakes below still matter, because they are the ones
that break a peer *after* the IDs agree, in a Bridge whose mirror was edited
by hand.

**An optional field is left out when it is empty, never sent as `null`.**
Bridge types it `field?: T` and compares against `undefined`, which `null`
passes. Every `Option` on a DTO Fleet writes carries `skip_serializing_if`, or
`deserialize_with = "stated"` where `null` is a value a person chose.
`xtask/src/rules_protocol/nulls.rs` is the rule.

The three people get wrong most often: widening an enum "because it's just
adding cases," making a field `Option<T>` "to be safe," and renaming a variant
"for clarity." All three feel non-breaking from inside the change and are not.
If you catch yourself writing "this shouldn't break anything, it's just
adding/loosening X" — that sentence is the tell. Check whether the mirror in
`packages/protocol` has an exhaustive match, a presence assumption, or a name
lookup anywhere near the thing you are touching.

**A closed set Bridge matches on is a `wire_enum!`, and one it only renders is
a string.** `StudioNodeState` — where a node on a Studio stands — is a strict
`wire_enum!`, because Bridge draws a different thing for each of its values. A
new one would mean the Studio grew a state, which is a change every reader has
to be told about. A `Requester.kind`'s set grows every time Fleet
learns of another asker, which is a change no reader needs to be told
about at all. Ask which of those two a new set is before making it open.

## What Bridge does with the ID it reads

Bridge is the side that decides. It reads Fleet's ID out of the runtime file
**before it opens a socket**, so a refusal is a screen rather than a malformed
first message, and it checks the same fact again on the resync — a client that
reached the socket without reading the file has had no check at all, and a Fleet
restarted under a live socket is not the Fleet the file described.

| Reading | What Bridge does |
|---|---|
| The IDs are equal | Connects. Nothing is said about protocols |
| The IDs differ | **Refuses.** Opens no socket. The notice says Fleet and Bridge do not match and offers Restart Fleet |
| The runtime file has no ID | **Refuses.** The same, and the notice says Fleet is out of date, since that is the one case where the side is known |

**The one action is Restart Fleet, on the notice.** It is `launchctl kickstart
-k` run by Bridge's main process (`apps/desktop/src/main/restart-fleet.ts`), not
a route, so a Fleet too old to have one is restarted as well as any other. What
it can and cannot do:

- It restarts the job onto the `armada` that is **installed**. Bridge cannot
  build or install one. If that binary is not this Bridge's build, Fleet comes
  back still apart and the window says so ("Still not matching"), and the one
  thing left is to update Armada and reopen Bridge.
- Working agents are adopted at boot, as with `scripts/restart --adopt`: they
  cannot be redirected until they finish. The tooltip says so, and pressing is
  the person's say-so.
- It refuses a Fleet whose pid launchd does not hold (one started by hand), since
  restarting the job would start a second Fleet beside it.
- A failure is `bridge.fleet.restart_failed`, a fault with its own notice and the
  usual copy-debug-info acts. A restart that takes but shows no new Fleet in 30
  seconds is the same code with `no_answer`.

Bridge cannot say which side is stale from the IDs alone, so the sentence does
not guess. `packages/shell/src/fleet.ts` carries the sentences, and the detail
row names the first eight digits of each ID so a person can read them to
someone else.

**The refusal is not a fault of the Jobs.** Fleet is alive and dispatching, and
the screen says so. Bridge declined to read it rather than failed to, which is
why it draws as degraded and not red.

## Why a mismatch is dangerous here specifically

Skew is usually a deploy-time annoyance: you restart the old thing, it's fine.
That's not what happens here, because **Fleet outlives Bridge by design.** Fleet
is a daemon; Bridge is a window someone closes to go to lunch. A Job runs
unattended, with agents spending real tokens against real API budgets, for
however long it takes — hours, sometimes. Fleet gets upgraded during that window
because that's when upgrades happen: nobody's watching.

So the window isn't "between deploys," it's "for the entire duration of
whatever Job happens to be running when someone updates one side." A mismatch
met mid-Session does not get a graceful restart: the connection that was streaming
its events goes bad while an agent is mid-tool-call, burning budget, with
nobody able to see what it is doing.

**Strict equality makes that more frequent, and it is accepted.** Before IDs, an
additive change was a banner and nothing else, and only a breaking one refused.
Now any change to the wire files refuses a Bridge relaunched after it until
Fleet is restarted, including a change that would have been additive. `scripts/restart --adopt` carries
working Jobs across the restart, so the cost is a restart rather than a lost
Job. What is gained is that no change is judged
"additive enough" by the person making it, which was the judgement a banner was
trusting.

**And the same lifetimes make one direction the likely one.** A running Fleet's
ID does not change when someone updates the app: the daemon started last week is
still speaking last week's protocol, and the Bridge relaunched after the update
is the newer of the two. The other direction is rarer, reached by restarting
Fleet without relaunching Bridge.

## The v0 lifeboat

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

## The first socket: the fleet's facts

`GET /events` is the stream Bridge draws the Cockpit from. Its shape is
`crates/ipc/src/event.rs`, which points here.

**A reconnection resyncs; it does not replay.** Every connection opens with a
`Resync` carrying the protocol ID and the cursor the stream is current as of.
Replay from the beginning was rejected on what it costs: a Bridge reopened
after lunch would fold hours of events before it could draw anything. **A
resync carries no state.** Each surface re-reads what it shows with the read
that serves it (`get_session`, `get_merge_lines`, `get_fleet_timeline`, ...),
and an event is only the wake-up for that read or the whole row it replaces.

| A resync rebuilds | A resync cannot |
|---|---|
| The cursor, and the ID Fleet is speaking | The path taken — no history of events |
| Nothing else: the reads do the rest | An instant for anything that already happened |

A surface drawing the whole Fleet's history reads `get_fleet_timeline`, below.

**What a resync cannot rebuild, Bridge reads back — every region together.** A
resync is a cursor, so nothing under an open Job comes with it, and the
receiving side is what makes the open screen whole again. Reading back some of
it is worse than reading back none, because the region left out is usually the
one drawing the failure.

**A resync arrives on two occasions, and they are not the same recovery.** The
first message on a connection is Fleet coming back, and every read attempted
while the socket was down failed — so a surface can be holding a failure nothing
else will clear. A resync following a `Missed` is a gap under a connection that
held, where HTTP answered throughout, so only what events keep current can be
stale. Bridge tells them apart by which resync it is on the socket.

| Taken again on a gap | Taken again only on a reconnection |
|---|---|
| Everything an event re-reads: the Job whole, what it holds, its thread | The reads only a press makes, and only where one is showing a failure |
| Either per-run socket that is down | |

**A reconnection is not a refresh.** Re-reading every open surface on every
resync spends the bytes the narrow reads exist to save, and a flapping Fleet
turns that into a fetch every retry. What comes back is what a screen is
showing and cannot repair itself.

**The stream is global, and a client subscribes to nothing.** Bridge holds
exactly one connection and the Cockpit renders every Job on it. A
per-subject subscription would put state on a connection whose whole value is
being cheap to drop and remake, and would need a subscribe message, an
unsubscribe message, and a rule for what a resync means when the set changes
mid-stream. The place a per-subject subscription *is* right is the run sockets,
below.

### A kind exists when something produces it

**An event kind that never fires reads as a stream that is working**, so a kind
`crates/ipc/operations/` names is not stubbed until a record exists for it
to carry. The kinds still waiting describe records this workspace has no type
for.

### A row appearing is a kind, not a state change

When a record is created while a client is connected, the event carries the
whole row and no `from`. A `StateChanged` with a `from` the edge table has no
row for would tell every client folding the stream to apply a move that did not
happen. **A creation is a row appearing, not a row moving**, and a deletion is
the opposite message with the opposite payload: only the id, because by the time
it is published there is no row left to carry.

### A fact about the fleet is a reading, not only an event

`manifest.reread` says what Fleet's last read of `armada.yml` came to. A
Manifest is Fleet's own, so nothing else on screen moves when it arrives.

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

Picture several Jobs running against a Bridge that's been minimized to the
tray, or is on a slow connection, or is just a slow renderer under load.
Fleet keeps producing tool-call and status events at agent speed. Bridge
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
silently believes its event log is complete will render a Cockpit that's
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

## The run sockets: one run, one server

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

## The Helm socket: retired

`GET /helm/observe`, `POST /helm/tune` and `GET /helm/file` are gone, with the socket's messages (`opened`, `asked`, `fresh`, `unanswered`, `sent`, `facts`, `missed`, `closed`) and the types that carried them. Helm is a hosted Job, and its thread is read the way a Job's is: *Helm is a Job*, below. 

## The Commander routes and the wave routes: gone

The Commander's socket, its words, its card and the owner's pick, its repair
Drone and its model choice are gone from Fleet, with the `commander.changed` and
`commander.card` events and the types that carried them. So are
`POST /jobs/:job_id/approve_wave` and the `dispatch_job` MCP tool, with the wave
a plan proposed. The Commander was deleted with the Jobs it steered
(`.claude/decisions/2026-10-15-sessions-absorb-jobs.md`). A Job's linked
Jobs and *after another* are not waves and stay.

## Changes before IDs

The sections headed `Protocol N.M` below were written when the wire carried a
version number, and they keep it. Each records what a change put on the wire
and why, which is still true. Nothing reads the numbers any more, and a bump
rule or a major/minor classification in one of them is history, not an
instruction.

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
would have without this design — since 2 Oct 2026 it sends the step back to a
Drone with the Judge's finding instead, by `restart_step`'s road, with no
change to the wire; either disagree advances it, and
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
(`crates/verification/src/forbidden.rs (deleted)`) refused every later step of the Job
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
or with no job, nothing is sent, because the next brief's THE PLAN is
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

## Protocol 13.37: the agent door answers about the repository a job stands in

`?manifest_id=`, optional and additive, on `list_jobs`, `list_job_board`, `list_reviews`, `get_activity_feed`, `list_alerts`, `list_drones`, `list_worktrees`, `list_servers` and `get_events_since`: absent is every repository, as before, so Bridge's All view is unchanged (#987). A named `get_events_since` counts events about that Manifest, about a Job it owns, and those naming neither, which are the machine's. `armada mcp` names the Manifest it walked to on every call to `/agent/mcp`, and the door names it on each of those routes and on the Manifest reads. Through the door, a `:job_id` another Manifest owns is refused as `fleet.job_in_another_repository`, a call naming another Manifest is refused, and `propose_job` takes its owner from the scope. Bridge's own routes are unscoped.

## Protocol 13.38: a workspace's Command runs where the root has no Manifest

`?repository=<root>` on `start_checkout_run`, `undo_checkout_run`, `list_checkout_runs`, `get_checkout_run_output` and `get_checkout_run_diff`, refused beside `?manifest_id=` as Verify's routes are (#986). `StartCheckoutRun.workspace`, optional, names a directory whose own `armada.yml` declares the Command. `CheckoutRunSheet.workspaces` lists each workspace's Commands, and `workspace` rides on `CheckoutRunUnderway` and `CheckoutRunRecord`. All additive.

## Protocol 13.40: how many of a step's Checks run at once

`LimitValues.checks_at_once` and `SaveLimits.checks_at_once`, additive: a fourth limit on `get_limits` and `save_limits`, from 1 to 8 (#284). Fleet runs a step's Checks up to it, and before starting each one after the first reads the machine against the memory and disk limits, so a short machine makes the next Check wait for a running one to finish.

## Protocol 13.43: a gate reuses a passing dry run

`CheckRun.reused_from_dry_run`, additive: when a Check's result came from the Drone's own `run_checks` instead of the gate running it again, rather than absent for a Check the gate ran itself (#1014).

## Protocol 13.44: Helm puts an approval card in front of the person

`ask_person_to_approve`, additive: a new command, `POST /jobs/:job_id/ask_person_to_approve`, answering `AskedApproval { job_id, handle }`. `agent_access = "Drafts only"`, the door offers it to a Helm session alone (#1041). It writes nothing — `Resolved` already turns a Job id that names nothing into the ordinary 404, and past that the route hands the id and handle back untouched. `approve_dispatch` stays `agent_access = "No"` for every agent; this only names which Job a card is for, and the person's own press on it is still what releases the Job. **Since Helm became a Session the card is drawn from the call's tool row** in the Helm Session's thread (`ask_person_to_approve <job id>`, the tool name prefixed by the server), and nothing new crosses the wire: the Job's facts and whether its gate still waits are the Board's, and the press is `approve_dispatch`.

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
Helm job, carrying `StudioHelmActed { studio_id, manifest_id, act, at }`
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
selected on its whiteboard, only beside its Studio. Fleet's line to the job names both by id,
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
deliberately absent from `operations/` and from `SERVED`, and a row added
for either would claim Bridge can call it. It also means the rule below does
not cover it: the address is written into a Drone's `mcp.json` from `api`'s own
constant, and that shared value is what stands between a typo and a Drone that
can never report.

**The rule that reads `operations/` runs both ways now.** One direction
fails on a route serving a name the inventory does not have; the other fails on
a name the inventory has and nothing serves. The second carries an allowance,
and every entry in it states a reason the gate prints — a list of exemptions
with no sentence each is the silent default the column was given reasons to
remove.

**A second route on the listener is not on this seam either, and it is this
seam spoken differently.** `/agent/mcp` is the agent's door: an MCP client
reaches it, and every tool on it is one row of `operations/` served at the
route `SERVED` already names, so there is no second implementation to drift.
What it adds over the HTTP surface is a cap — a tool answer over 64 KiB is cut,
says so, and names the route that serves it whole and the parameters that ask for less — and a scope: every answer
is inside one Manifest, and the handshake says which. It answers 405 to `GET`
and `DELETE` for `/mcp`'s reason, so it adds nothing to the risk above.

**Who opens it is the repository somebody is standing in.** `armada mcp` is the
relay a repository's own `.mcp.json` names: it reads `fleet.json` for the port,
refuses on `Stale { PidHeldByAnother }` rather than connecting to a port another
process now holds, and resolves its own working directory to a Manifest — never
a request field, for the reason a Job id is not one on a Drone's tools. A
job started below a repository root walks up to it, and says at its
handshake which root it settled on. The walk ends at a repository that has no
Manifest of its own, and at the home directory, so it can reach neither a parent
repository nor an `armada.yml` sitting above every project on the machine.
A job that resolves to no Manifest, or to one this Fleet is not serving, is
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

`#1373`. **Retired 13 Oct 2026 with Helm's checkout writes; see *Helm is a Job*.** `helm.changed_checkout`, a new event kind, additive: published as a Helm job's own
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
A Link is what no adapter recognised — a board, a page, a document, a job — and dispatches
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
carries `HelmAskingToRun { waiting: HelmCallInFlight }` when a Helm job reaches for something
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

## Protocol 15.1: a Helm job can be carried to whoever could fix it

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
than per Check — the retired Runner adapter concept. So this field does not grow
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
from `operations/`'s own `kind = "query"` rows at the moment a person presses, so a Fleet whose
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
already resolves both policies at every gate (the `policy` module (deleted since)), so this reports a
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
not here — the `proposal` module (deleted since)'s header carries the 3 Sep 2026 ruling
that how the work lands follows from having read the code, and the owner kept it
when it was put beside the model.

**Fields that are settled, never a transcript.** Fleet reads the answer's prefix
in one place (the `proposing` module (deleted since), `Settled::of`), and a field
crosses only once its own line has ended — so a client either has a title or has
none, and never has `Say which of the two giv`. The raw text does not cross:
`answered_characters` is still a count, and `crates/ipc/src/proposing.rs (deleted)` carries
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
(`filled`, in `packages/screens/src/proposal.ts (deleted)`): the settled workflow is the
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
`killed` is told apart from `failed`, is the operation's note in `crates/ipc/operations/`.

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
`crates/ipc/operations/` carry the whole rule.

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
new was the writer: `armada land` was another process, so nothing told Fleet a line moved. Fleet
read every served repository's `armada-land/` every two seconds and published only when the answer
changed. The files are no longer read; the loop folds the hub and publishes only when that moved. Bridge reads the route once per connection and keeps no timer.

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
`handed_in`, and a task's Drone is not offered it at all. the retired Plan concept, *A Drone per
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
bounds were in `adapters::land_state::line`, since removed. Both lists are `MergeLineEntry` rows, newest first, with
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
stops a step (answer 3). the retired Plan concept, *Groups*.

**Restart this task also answers a done task in a group the Judge refused** (2 Oct 2026), with
no change to the wire: Bridge reads the group's last run, `verdict.trigger` `gate_failure` over
tasks still `done`, with the step that run was filed under `stopped`, as Fleet does. **A group the
Judge only questioned is refused**: its step holds at `awaiting_human` until a person answers.

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

**`pull_request_comments`** is the count the sweep reads while the pull request is open, matching
what the forge shows: conversation comments, reviews that say something, and comments on lines of
the diff. The line comments cost one more forge call on a sweep turn that finds the pull request open,
the owner's call on 2 Oct 2026; the wire shape did not change, so there was no version bump. A turn
where either read goes unanswered keeps the last count. **Absent is unknown, never 0**: the pull
request settled before the rotation reached it open. Opening a new pull request clears it.

**No forge call on a Job read.** Both are written by the sweep and by opening the pull request. **Store V96** keeps
them, in two `jobs` columns.

## Protocol 23.6: a model per task, and a person's edit to a task

Spike 022, the wire lock for the new Job, slice 3 (#1764, carrying #1657).

**Optional fields, one new DTO pair and two routes, all additive.** `PlanTask` gains `tier`, a
strict `TaskTier` (`difficult`, `medium`, `easy`), and `model`, a person's pick. `JobDetail` gains
`tiers`, a `TierModels` of `difficult?`, `medium?` and `easy?`, and `JobDrone` gains `model`, the
model that Drone was spawned as. **An empty tier is a key left out, never `null`** (answer 8): a
tier the map leaves out is Armada picking, and the Drone's `model` says what it picked. Each is
absent where empty, so a 23.5 Bridge connects behind the banner and draws what it drew.

**Every spawn resolves its model in spike 022's order**: a person's pick on the task, then
`tiers` for the task's tier, then the step's model, then the Job's. `Job::model_spawned_for`
spells it once. A Drone on no task asks neither of the first two, and still records its model.

| Route | Body | Answers | Refused |
|---|---|---|---|
| `POST /jobs/:job_id/tasks/:task_id/edit`, `edit_task` | `EditTask`, only the changed fields | `WorkPlan` | 409 `fleet.task_in_flight` on a task working or handed in, `fleet.task_already_settled` on one done or dropped; 409 on a model `list_models` does not offer, as `set_model` refuses one; 422 on a body changing nothing or a blank title, and `fleet.no_such_task` |
| `POST /jobs/:job_id/set_tiers`, `set_tiers` | `SetTiers`: `tiers` | `JobSummary` | 409 on any model `list_models` does not offer, keeping nothing |

**The edit's body is the lock's, which is Bridge's.** `EditTask` moved from `pending.ts` to
`work-plan.ts` unchanged. It reaches an open or a failed task, so it works before the plan's gate
and after a failure alike, and it is appended as a person's change: `WorkPlan.recorded_by` stays
the planner, and the history reads the edit after the recording. Bridge's pending entry goes with
the Bridge half, which answers it in the mock.

**`set_tiers` is a choice the lock did not make.** The spike sources a Job's tiers from *dispatch,
then approval*, and both are slice 4's bodies; a Job needs a map before then for slice 3's claim
to be anything but a test, so the map is a Job setting `set_model`'s way, and slice 4 fills it from
those two as well.

**`record_plan` takes an optional `tier` per task.** It is an MCP tool, not this protocol, and its
own schema says so. **Store V97** keeps a recorded task's tier, a person's edit, a Job's map and
the model each Drone ran.

## Protocol 23.7: one Check's log on a merge line

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
link. Everything else is one refusal. `adapters::land_state::line::check_log` was the rule, held by
`land_logs.rs` to a path, `..`, a waiting Check, an unserved root, a turn outside `logs/` and a
link. Both are removed with the command; a branch's line has no log to open, and main's are served
as before.

**Additive, so the minor moves.** `LandOutputMessage` is a new message family, `OutputMessage`'s
three with its own opening: `LandOutputOpened` names the root, the branch and the Check, and carries
no path, for `get_merge_lines`' redaction. `OutputLines` and `OutputClosed` are reused whole. A
23.6 Fleet has no such route, so a 23.7 Bridge behind it is refused, which is the skew rule's own
direction.

**Bridge holds it in main**, `land-following.ts` beside `following.ts`, published as
`BridgeState.landFollowed`, one at a time. The preload's `followLandCheck` takes the three names
and main reads only three strings off whatever the window sent.

## Protocol 23.8: the proposal is what runs

Spike 022, slice 4 (#1765, carrying #1641, #1642, #1605, #1581 and #1699's route).

**A body, a route, a read and optional fields, all additive.** `approve_dispatch` takes
`ApproveDispatch`, the whole proposal as a person left it, and an empty body still approves it as
it stands, so a 23.7 Bridge approves exactly as before. `edit_job` saves `title?`, `facts?` and
`criteria?` without the press. `list_branches` reads the repository's branches through git.

| Route | Body | Answers | Refused |
|---|---|---|---|
| `POST /jobs/:job_id/approve_dispatch` | `ApproveDispatch`, or nothing | `JobSummary`, `queued` | 409 `fleet.proposal_frozen` past `awaiting_approval`; 422 `fleet.no_such_branch`, 422 `fleet.unacceptable_proposal` |
| `POST /jobs/:job_id/edit`, `edit_job` | `EditJob` | `JobSummary`, still `awaiting_approval` | As above, and 422 on a body changing nothing |
| `GET /manifest/branches?manifest_id=`, `list_branches` | — | `Branches` | 422 on a repository not served |

**Read on the wire:** `Criterion.origin`, `{kind: issue, ref, url}`, `{kind: prompt}` or `{kind:
person}`, absent where nothing says; `Criterion.origin_moved_at`, on an issue's line once the
issue was edited after Fleet read it; and on `JobDetail`, `landing` (`target?`, `from_ref?`,
`pr_mode`), `drone_cap`, `policy_overrides` (`auto_merge?`, `review_gate?`, the words
`armada.yml` writes) and `approved_at`. `JobRequest.settings` carries the dispatch form's
`workflow_id?`, `tiers?`, `drone_cap?` and `lands?`. Each is absent where empty.

**The lock's bodies, and what slice 4 chose inside them.** `edit_job` takes #1641's criteria
rather than Bridge's lines, the spike's bodies table. An overridden step keeps deferring on the
record and the override is laid over the repository's word after the fold, so it wins however the
rule moves (answer 4) and #1683's record shows the word each gate read; `auto_merge` overridden
without a person reads `checks-pass`. Only `delivered` completes a Job and only one branch per Job
is cut, so `branching: group` and the other three `complete_when` values are refused rather than
kept unread; `land_together` does not cross. `from_ref` cuts the worktree and is what its work is
measured from; `target` is where the pull request opens. The Drone cap is kept, and slice 5
enforces it.

**Store V98** keeps a Job's landing, Drone cap, policy overrides and the issue its request linked.
The issue rotation is a second cursor on the pull-request rotation's interval (answer 5), over
every Job in flight whose request linked an issue.

## Protocol 23.9: a node is added inside a Zone

The owner, 2 Oct 2026: pressing inside a Zone places the armed kind there and puts it in that Zone.
Bridge did it in two writes, an `add_node` on the board and a `move_studio_node` into the Zone, and
a refused move left the node on the board.

**`add_studio_node` takes `within`**, `move_studio_node`'s field: the frame it is added in, with
`position` measured from that frame's corner, and absent is the board. It is one store write, and
it refuses what the move refuses, with the move's codes: a frame that does not hold the kind is
`fleet.studio_frame_cannot_hold`, so a Zone in a Zone is, and so is anything added in a Cluster,
since a node just made is none of the Notes the Cluster was made of.

**Additive, so the minor moves.** A new optional field on a request. A 23.7 Fleet would drop
`within` and add the node at the board's origin plus the offset, so a 23.8 Bridge behind it is
refused, which is the skew rule's own direction.

## Protocol 23.10: several Drones at once, and one of them addressed

Spike 022, slice 5 (#1766, carrying #1666, #1651 and #1648's remaining half).

**Two routes and optional fields, all additive.** `PlanTask` gains `concurrent_with`, the other
tasks of its group it may run at the same time as, read both ways and less any pair Fleet ran
apart, and `touched_after_done`. `JobProcess` gains `drone_id`, and `recorded` now marks one row
per live Drone rather than one per Job, which is how *Drones running* is counted. Each is absent
where empty or false, so a 23.9 Bridge connects behind the banner and draws one Drone.

| Route | Body | Answers | Refused |
|---|---|---|---|
| `POST /jobs/:job_id/drones/:drone_id/kill`, `kill_one_drone` | — | `JobSummary` | 409 `fleet.drone_not_live`, with `drone_id` on its own field |
| `POST /jobs/:job_id/drones/:drone_id/redirect`, `redirect_one_drone` | `Redirection`, `redirect_drone`'s | `JobSummary` | 409 `fleet.drone_not_live`; 422 on a blank instruction |

**The kept Drone is the Job-wide act.** A Job keeps one Drone from admission to its end (answer
2); naming it is `kill_drone` or `redirect_drone`, and `kill_drone` now ends every Drone beside it
too, since the step they work stops. A Drone beside it is stopped alone, its task back to `open`
for the kept Drone, or told in its own session with the Job unmoved.

**The machine's cap counts Drones.** `get_capacity`'s `occupied` and the bound behind it count
each held Job's kept Drone and each Drone beside one; the Job's `drone_cap` from 23.8 sits inside
it, and a Drone beside a kept one gives way to a Job waiting to start.

**A Drone beside the kept one is announced, and is on no record of moves.** `drone.spawned` and
`drone.exited` carry its id with the Job as it stands, whose `assigned_drone` stays the kept one;
`list_job_drones` lists it off its task binding. **`observe_job` is one channel per Job**, made at
admission and dropped at the Job's end, so `closed` with `drone_ended` means the Job stopped
writing rather than one Drone exiting.

**`record_plan` takes an optional `concurrent_with` per task**, the places in the list of tasks
that may run beside it, refused where one is the task itself or in another group. It is an MCP
tool, not this protocol. **Store V99** keeps it, each Drone beside a kept one and how it left, each
task Drone's edit calls, and the pairs run apart. Overlap is read from edit calls only (answer
10): a shell write is not seen.

## Protocol 23.11: a Drone resting at the gate, and turns nobody counted

Job 3 on 3 Oct. Its last task's Drone submitted and ended its run, and `list_job_drones` read it
`running` for the seven minutes the step's Checks ran, so Bridge drew a Drone working while the
Checks were. Its three task Drones before it read `turns: 0` with no cost, each having made 6 to 12
calls: Fleet ended each within a quarter-second of its hand-in, before its terminating line.

**`JobDrone.at_rest_since`, optional and additive, so the minor moves.** When a `running` Drone's
last run ended, off its transcript's last terminating line with no run started or turn sent after
it; absent while it works and on a Drone that has left. `running` still means Fleet holds it — at
rest at the gate it is the Drone a red Check goes back to — so a fifth `DroneState` would have
been a major move for a Bridge matching on the states it knows, and the field says the same thing. A 23.10 Bridge
connects behind the banner and draws it working, as before.

**`turns` is absent on a stopped Drone whose row saw no terminating line**, where it read `0`. The
wire type is unchanged, and absent was already the field's word for none seen. Fleet now lets a
task's Drone that handed in come to rest before ending it, within `StepNorms::report_grace`, so
its row carries the harness's turns and cost; one that does not rest in that time reads absent.

## Protocol 23.12: a Job's retro, and who acted

The owner, 3 Oct 2026: every Job reflects when it ends. `../concepts/retro.md` is the concept.

**Two routes, one optional field, two enums, all additive.** `Recorded` gains `via`, the door the
request that made a move came through, absent on a move Fleet made on its own. Bridge sends
`x-armada-caller: bridge` on every request, and Fleet reads a request without it as not Bridge's.

| Route | Answers |
| --- | --- |
| `GET /jobs/:job_id/retro`, `get_job_retro` | `JobRetro`: `state`, `items`, `record`, `annotations` |
| `GET /lessons?manifest_id=&most=`, `list_lessons` | `Lessons`: items across Jobs, newest retro first |

**A person's act from any door but Bridge is signed `helm`.** `actor` keeps its set: an agent made
the request and Fleet cannot see the person behind it. A Bridge before 23.12 sends no header, so
behind a 23.12 Fleet its presses read `helm` with `via: http` until it is rebuilt.

**`submit_evidence` takes an optional `in_the_way`.** It is an MCP tool, not this protocol, and its
own schema says so. **Store V100** keeps each move's door, a Drone's notes and the retro, and marks
every Job already ended `skipped`.

## Protocol 23.13: an Epic's wave, released in one press

Spike 022, slice 6 (#1767, carrying #1694, #1699's rest and #1692).

**A route, a body and optional fields, all additive.** An Epic's plan proposes its wave as real
Jobs at `awaiting_approval`, each `dispatched_by` the Epic, and one press releases them.
`JobSummary` gains `dispatched_pass`, the pass of the parent's plan that proposed it, counted from
one, and `merged_at`, when its pull request merged; `JobDelivery` gains `merged_at`; and
`LandingRule` gains `complete_when`. Each is absent where empty, so a 23.12 Bridge connects behind
the banner and reads a wave as Jobs at their gate.

`POST /jobs/:job_id/approve_wave` is retired; see *The wave routes: retired*.

**One act releases the wave.** Each member takes `awaiting_approval -> queued` as a person's act,
as `edit_job` left it, and the parent's plan step advances without the step after it being
entered: the parent waits on its members as it waited on a dispatch before. A member dropped
before the press (`kill_job`) is not in the wave. Three acts that would cut across the press are
refused with the same code: `approve_review` on a plan holding a wave, `approve_dispatch` on one
member, and the last approval of a parent with `complete_when: all_members_landed` while a member
has not merged.

**`all_members_landed` is honoured**, so the approval keeps it rather than refusing it. Under it
the parent's wait after its plan also counts a finished member whose pull request has not merged,
and its roll-up is written about what landed. **`merged_at`** is the forge's own instant, read on
the call that said it merged, or Fleet's clock where Fleet pressed the merge; absent beside a
`merged` noticed before 23.13.

**The shipped epic changes shape, and the parser holds it.** `may_dispatch_jobs` moves from
`dispatch` to `plan`, the step a person answers, and `dispatch` goes: a step that creates Jobs
must be `human_always` now, since what it creates waits for that gate. A plan that runs again
withdraws, as Fleet, what its last attempt proposed and nobody released. A Job frozen on the old
three-step epic runs as it was frozen.

**Store V101** keeps the pass on the Job as `DispatchOrigin`'s third column, when a pull request
merged, and `complete_when` with the landing. A member with a pass rebuilds at
`awaiting_approval`; a child with none is one made before V101, which entered `queued`.

## Protocol 23.14: each wave's line, and the order a wave waits in

Spike 022, slice 6's last two pieces (#1692): `WaveRoundView`'s per-pass line, and the waits-on
edges a wave's graph is drawn from.

**Two optional fields, both additive.** `JobDetail` gains `wave_rounds`, and `JobSummary` gains
`waits_on`. Each is absent where empty, so a 23.13 Bridge reads neither key and draws each wave by
its number alone, in one column, as it did.

| Field | On | Carries | Absent |
|---|---|---|---|
| `wave_rounds` | `JobDetail` | Retired: Fleet no longer fills it | Always |
| `waits_on` | `JobSummary` | The ids of the Jobs this one waits on: its `depends_on` edges, which `JobDetail.dependencies` serves whole | A Job that waits on nothing |

**A pass is counted as `dispatched_pass` counts it**: one, plus every return the roll-up sent onto
the step that proposes. A recording belongs to the pass the returns before it opened, so a Judge's
rerun inside a pass replaces that pass's line, and a return starts the next. **The approach is
served whole**, a paragraph; Bridge's strip reads its first sentence, because a tab does not
truncate.

**The edges ride the Board row rather than a read per child.** The row is built from the Job record,
which already holds its edges, so serving them costs no store read; the alternative was one
`get_job` per member to draw one graph. Only `depends_on` crosses: a `blocks` edge is the same
fact read from the other Job, whose own row carries it.

**No store change.** The lines are read off the plan's history and the step moves already kept, and
only for a Job whose frozen workflow has a step that proposes Jobs.

## Protocol 23.15: where a retro item's fix lands

The owner, 3 Oct 2026: every retro item says where its fix lands, apart from whose way it got in.
`../concepts/retro.md`, *Where the fix lands*, is the concept.

**One optional field, one query parameter, one enum, all additive.** `RetroItem` and `Lesson`
gain `lands_in`, exactly one of `armada`, `kit` or `manifest`. `GET /lessons` takes
`?lands_in=` to narrow to one, and absent is all three; any other value is a 400.

| Field | On | Absent |
| --- | --- | --- |
| `lands_in` | `RetroItem`, `Lesson` | An item kept before 23.15, which no `?lands_in=` matches |

**Never defaulted.** The retro call is told the three places and must name one per item; an item
naming none, or one that is not one of the three, is dropped the way an item citing nothing the
record holds already was. A fix that spans two places is two items. **An old item is not
migrated**: a place chosen after the fact would be a guess, so it reads with the field absent and
Bridge lists it under All alone.

**Store V102** adds the nullable `lands_in` column to `job_retro_items`.

## Protocol 23.17: a person reshapes the slot pool

The owner, 4 Oct 2026: from Cleanup's bay grid, add a slot, remove one, or close one for a while.
`../concepts/fleet.md`, *Worktree slots*, is the concept.

**One field and one command, both additive.** `WorktreeSlot` gains `closed`. `change_slot_pool`
is `POST /worktrees/slots?manifest_id=` with `ChangeSlotPool` (`act`, and `slot` for all but
`add`), answered by `SlotPoolChanged`.

| Field | On | Absent |
| --- | --- | --- |
| `closed` | `WorktreeSlot` | A Fleet before 23.17, which closes nothing: read as open |

**Each refusal has its own code**, so Bridge says on the slot why it cannot go:
`fleet.slot_held`, `fleet.slot_stranded`, `fleet.slot_busy`, `fleet.slot_not_a_checkout`,
`fleet.slot_dirty` and `fleet.slot_last` are 409s, and `fleet.no_such_slot` a 422.

**No store change.** The pool's shape is written beside its slots at `.armada/slots/pool`, which
the CLI reads too.

## Protocol 23.18: walk notes, kept on the Job, and a server started for review

What a person points at while walking a Prototype's served mock in a Bridge window, kept by Fleet
on the Job so it outlives the throwaway worktree, and handed to the next Drone when the Job is sent
back.

**Additive only.** Two new routes, four new DTOs and three optional fields; a 23.17 Bridge sends no
`with_walk_notes` and reads no `walk_notes`, and sees exactly what it did.

| Change | Where | Carries | Absent |
|---|---|---|---|
| `capture_walk_note` | `POST /jobs/:job_id/walk_notes` | `CaptureWalkNote { said, capture, frame? }` in, `WalkNotes { notes }` out | — |
| `remove_walk_note` | `POST /jobs/:job_id/walk_notes/remove` | `RemoveWalkNote { id }` in, `WalkNotes { notes }` out | — |
| `walk_notes` | `JobDetail` | `WalkNote { id, said, at, element, selector, location, served?, frame?, sent }` per note, oldest first, sent ones included | A Job with no walk notes |
| `for_review` | `ServerState` | Fleet started it because a step with `evidence.walked` stopped for a person, on the Manifest's `walk` server; Bridge opens it when that Job is opened | Started by anything else |
| `with_walk_notes` | `ChangesRequested` | Append every unsent walk note to the note delivered, and mark them sent | False |

**The frame is a path under the machine directory**, `<machine>/walks/<job_id>/<note_id>.png`,
never the worktree. **A blank `note` is taken with `with_walk_notes`** where at least one unsent
note exists, because the notes are then what the Drone is told; otherwise blank is refused as it
was. The notes are marked sent only once the send lands, so a refused one leaves them unsent.

**Store V103**, `job_walk_notes`: one row per note, keyed to the Job so `forget_job` takes it. A
trigger refuses any update but `sent_at` moving once from null.

## Protocol 23.19: a step's phase, and the grants a canvas draws

The owner, 4 Oct 2026: the approval canvas lays its lanes by phase. #1768 waits on it too.

**Optional fields and one enum, all additive.** `WorkflowStep` gains `phase`, `may_dispatch_jobs`
and `drone_per_task`; `StepDetail` gains `phase` and `may_dispatch_jobs`, `drone_per_task` being
there since 23.1. `StepPhase` is `setup`, `work` or `delivery`, a plain closed set Bridge lays a
lane per value of rather than matching on.

**Fleet serves the phase resolved.** A step's own `phase:` where it declares one, else `delivery`
where it `delivers` and `work` otherwise: `core_model::StepPhase::of` spells it once. Design Plan's
`present` and Epic's `roll_up` declare `delivery`, since neither sends anything out. On
`StepDetail` it is absent where Fleet cannot say, as `delivers` is. The two booleans are absent at
false.

**The workflow file gains `phase:`**, a word refused outside the three. **No store migration**:
the frozen workflow column carries `phase` beside `model`, null where the step declared none, and
a row frozen before it reads as none. The event stream is untouched.

## Protocol 23.20: a step tuned at the press

The owner, 4 Oct 2026: the approval canvas tunes each step, and lands only once Fleet takes what
it draws.

**One optional field, one DTO and one enum, all additive.** `ApproveDispatch` gains `tuning`, a
`StepTuning` per step a person tuned, `gates`' shape: `step_id`, and `model?`, `effort?`
(`Effort`: `low`, `medium`, `high`), `context?`, `judges?` and `checks_off?`. A field left out is
the step as declared, so a 23.19 Bridge approves as before.

**Frozen into the step, after its gate**, as `gates` is, so the Job keeps it for its life and each
reader finds it where it already reads the step. `model` is the step's model in
`Job::model_spawned_for`'s order, under a task's pick, a tier and `set_model`. `effort` goes to the
harness at the spawn; absent sends nothing, the harness's own default. `context` is the Drone's
FOR THIS PART block (`../contracts/agent-prompt.md`). `judges` sets every judge check's
`panel_size`. `checks_off` drops Manifest Checks by name and refuses Fleet's own looks, as the gate
box does.

**Refused, keeping nothing**, with 422 `fleet.unacceptable_proposal`: a step the workflow lacks,
one tuned twice, a blank model, `judges: 0`, `judges` on a step with no Judge, a Check the step
does not run, and a built-in. A model `list_models` does not offer is refused as `tiers`' is.
**`StepTuning` refuses an unknown field**, so the canvas's `harness` is refused rather than
dropped: one harness runs, and a per-step one is not built.

**No store migration**: the frozen workflow column carries `effort` and `context` beside
`model`. `get_job` reads `checks` and `judge_checks` off the frozen step as before; the effort and
the words are not served back. The event stream is untouched.

## Protocol 23.21: a branch that does not exist yet

The owner, 4 Oct 2026: a Job may start from a branch a person names on the canvas before it exists.

**One optional field, additive.** `LandingChoice` gains `start_point`, a branch the repository
holds. Where `from_ref` names a branch the repository does not hold, Fleet makes it at
`start_point`'s commit at the press, a local branch, and the worktree is cut from it as from any
`from_ref`. `target` may name the same new branch. **Read only then**: beside a `from_ref` the
repository holds, or none, it is not used. A 23.20 Bridge sends none and is refused an unknown
`from_ref` as before.

**Refused before anything is kept**: a `from_ref` the repository lacks with no `start_point`, and
a `start_point` it lacks, are 422 `fleet.no_such_branch` naming the branch. git refusing to make
the branch is a `fleet.fault` naming both. The branch is made before the proposal is written, so
an approval refused after it, by the machine, leaves the branch at its start point.

**`Vcs` gains `create_branch`**, which refuses a name already a branch. No store change: the
landing row keeps `from_ref` as before, and `start_point` is not kept. The event stream is
untouched.

## Protocol 23.22: where an approved Job lands, set once

The owner, 4 Oct 2026: a Job approved landing in the base may be aimed at another branch after
the press.

**A route and a body, additive.** `POST /jobs/:job_id/set_landing_target`, `set_landing_target`,
takes `SetLandingTarget { target }` and answers the Job's `JobSummary`. `JobDetail.landing.target`
reads it back, and the delivering step opens its pull request against it, as against a target the
approval set. A 23.21 Fleet has no such route, so a 23.22 Bridge behind it is refused, which is the
skew rule's own direction.

| Refused | Code |
|---|---|
| At `awaiting_approval`, where the approval sets it; ended; already landing where a person chose; its work committed or opened for review | 409 `fleet.landing_target_settled` |
| A blank branch | 422 `fleet.landing_target_blank` |
| A branch the repository does not hold | 422 `fleet.no_such_branch` |

**Once, and before the work goes out**, because the pull request opens against it: a target moved
after that is a record the forge disagrees with. **The worktree keeps what it was cut from.** No
store migration: the landing row takes the target. The event stream is untouched.

## Protocol 23.23: the harness a step runs under

The owner, 4 Oct 2026: the card shows the one harness Fleet runs, and any other is refused.

**Two optional fields, additive.** `ModelChoices` gains `harnesses`, the harness names a Drone may
run under, one today and absent from a Fleet before 23.23. `StepTuning` gains `harness`. A name
`harnesses` does not list is 422 `fleet.unacceptable_proposal`, and the message names the ones that
do. The one name is spelled in `adapters` (`HeadlessAgent::harness_name`). The tuning keeps no
harness, since there is one to run, so nothing about it is frozen or served back.

## Protocol 23.24: how the work leaves the worktree, as approved

The owner, 4 Oct 2026: the canvas offers stop at the branch, and a pull request that merges itself.

**Two optional booleans on two shapes, additive.** `LandingChoice` and `LandingRule` gain `local`
and `auto_merge`, each absent at false.

| Setting | What Fleet does |
|---|---|
| `local` | **Stop at the branch.** On entering the delivering step the work is committed on the Job's own branch, with no push, no pull request and no merge. The step then runs as any other, and the branch outlives the Job: reclaiming keeps unmerged work. |
| `auto_merge` | When the pull request opens, Fleet runs `gh pr merge --auto --merge` on it, so the forge merges once its required checks pass. A forge that will not, usually a repository that does not allow auto-merge, is said in the Job's log as a warning carrying the forge's own sentence, and the pull request stays open for a person. |

**One source of truth with `pr_mode`.** `local` wins: `pr_mode` is stored `ready` while it holds, so
`LandingRule` never reads `draft` beside `local`. `local` with `auto_merge` is 422
`fleet.unacceptable_proposal`, since a branch with no pull request has nothing to merge.
`policy_overrides.auto_merge` is untouched: it answers a `manifest_rule:auto_merge` gate, which is
the repository deciding, and this is one Job's forge setting.

**Store V104** adds `local` and `auto_merge` to `job_landing`, false on every row kept before. `Delivery`
gains `enable_auto_merge`. The refusal is a log line and not a wire error, because the approval
cannot know the forge's answer until the pull request exists.

## Protocol 23.25: a note sent back to the proposer

The owner, 4 Oct 2026: from the approval gate a person can send the proposal back with a note, and
the proposer rewrites it.

**A route, a body and the one new edge, additive.** `POST /jobs/:job_id/to_proposer`, `to_proposer`,
takes `ToProposer { note, tuning?, landing? }` and answers a `ProposedPlan`. The transition registry
gains `awaiting_approval -> proposing`, a person's act, crossed by `Job::sent_back_to_the_proposer`
alone; `proposing -> awaiting_approval` is still `Job::answered`'s. The first `job.state_changed`
carries the move out, and the answer's the move back, so a Bridge that reads the stream sees the Job
leave its gate and return. A 23.24 Bridge has no such route, and a 23.25 Bridge behind it is
refused, which is the skew rule's own direction.

| What | How |
|---|---|
| The proposer is asked | The Job's request, the proposal's title, and the note |
| What the answer replaces | Title, workflow, steps, criteria, urgency and the rest `Job::answered` freezes, whole |
| The person's tuning | Carried to every step whose id the new workflow has; the rest is dropped and logged |
| The person's landing | Carried as it stands; one the repository will not take is dropped and logged |
| A split | **The whole split goes back together**, from any one of its Jobs. The proposer answers once and its answer replaces the group (below); each Job comes to the gate on its own with the matching tuning |
| A call that fails | The proposal is put back as it was, and the refusal is the proposer's |

**A split goes back whole, so no sibling is stranded.** The group is the head and every extra
naming it, by number. The answer is laid over it in order: the head and the old extras are kept as
the same Jobs and rewritten whole, an answer with more Jobs makes the rest as new extras of the
head, and an answer with fewer ends the old extras it has no place for, killed with a line in their
logs saying the revised proposal replaced them. Every rewrite is read before any is written. A call
that cannot be read puts every Job of the group back as it was.

**Refused, moving nothing:** 409 `fleet.proposal_frozen` off the gate; 422
`fleet.unacceptable_proposal` on a blank note, on a Job of the group that is not at its gate (the
message names it and its status, and nothing goes back), on a Job dispatched by an Epic's plan step,
which the Epic's plan owns, and on a tuning or landing the proposal in front of the person cannot
take. The tuning and landing in the body are laid on every Job the answer makes. **Store**: no migration. `record_answered` clears the step and write-target rows a revised
answer replaces, and the rebuild replays `awaiting_approval -> proposing` and the answer back
through the machine, from the columns the last answer wrote.
## Protocol 23.27: a retro item says where it stands

The owner, 5 Oct 2026. The Retro sheet in Bridge offered Agree and Disagree on an item already
answered from the Lessons list, because `get_job_retro` carried no answer state.

**Additive.** Two optional fields, with the names and types `Lesson` already has.

| Change | Where | Absent or older |
| --- | --- | --- |
| `state` | `RetroItem` | Only on an item whose row has no answer record, read as before |
| `job_proposed` | `RetroItem` | Absent until agreeing proposed a Job for the item |

Both are read from the rows `list_lessons` reads. A 23.26 Bridge ignores them.

## Protocol 23.26: a retro item has parts, and a person answers it

The owner, 4 Oct 2026, after Job 3's retro was rejected as prose nobody could act on and wrong
about whose fault it was. `../concepts/retro.md`, *Items* and *Agree and disagree*, is the concept.

**All additive.** Two routes, one query parameter and a handful of optional fields.

| Change | Where | Absent or older |
| --- | --- | --- |
| `id` | `RetroItem`, `Lesson` | Always present: the Job's id, a hyphen and the item's place in its retro |
| `title`, `what`, `fix` | `RetroItem`, `Lesson` | An item kept before 23.26, which has `statement` alone |
| `state` | `Lesson` | Always present: `open`, `agreed`, `accepted` or `discarded`. Every item kept before starts `open` |
| `job_proposed` | `Lesson` | Absent until agreeing proposed a Job for the item |
| `paths` | `RecordCheck` | A failure that names no file, or one the Drone ran itself |
| `GET /lessons?state=` | `list_lessons` | Absent is `open`; any other word than the four is a 400 |
| `POST /lessons/:lesson_id/agree` | `agree_lesson` | New. Answers the `Lesson`. `agent_access` is `Helm only` |
| `POST /lessons/:lesson_id/disagree` | `disagree_lesson` | New. Answers the `Lesson`. `agent_access` is `Helm only` |

**`statement` stays**, and on an item written since 23.26 it repeats `what`, so a Bridge that
predates the parts still has a sentence to draw. **The listing's default changed in meaning, not in
shape**: before 23.26 it listed every item, and it now lists the `open` ones. A 23.22 Bridge
never answers an item, so every item it reads is open and it sees no difference.

**Agreeing**, by where the fix lands. `manifest` proposes a Job at the approval gate on the
repository the item's own Job worked on, and `armada` on Armada's own repository, the Manifest
named `armada`. Both go through `propose_from_request`, with the item as the request. `kit`
proposes nothing and the state becomes `accepted`. An item that is not `open` answers with the
state it stands in and proposes nothing, so agreeing twice makes one Job.

| Refused | Code |
|---|---|
| No item has that id | 404 `fleet.no_such_lesson` |
| An item kept before 23.15, which names no place its fix lands | 422 `fleet.lesson_names_no_place` |
| A fix in Armada, where this Fleet does not serve the Manifest `armada` | 422 `fleet.lesson_armada_not_served` |

A refusal from the proposer itself is returned as it is, and the item is open again.
**Disagreeing** keeps the row as `discarded` and never takes back a Job already proposed.

**Store V105** adds `title`, `what`, `fix`, `state` and `job_proposed` to `job_retro_items`. Existing
rows read `open`.

**The record gains a fact.** A gate failure that names a file carries `paths`, each file with
`named_in_drone_calls`: whether a tool call of the step's Drones names it. `false` says the failure
did not come from the Drone's own calls. It is handed to the retro call and drawn nowhere.

**The retro is written on its own model.** `ARMADA_RETRO_MODEL` overrides it and the default is
`sonnet`, from `crates/config/settings.toml`'s `retro-model`. The Judge's dial is not moved.

## Protocol 23.28: a workflow definition saved, and the folders read again

The owner's ask of 5 Oct 2026: Helm helps author a workflow and ends by creating it, in the repository or in Kit.

**A route and its types, additive.** `POST /workflows/save`, `save_workflow`, takes `SaveWorkflow { scope, definition, overwrite? }` and answers `WorkflowSaved { workflow_id, scope, file, replaced, runs_from? }`, `scope` being `repository` or `kit`. `LeftOutWorkflow.source` may now read `repository`, where it read only `armada` or `kit`: a Fleet that was already running set a repository's own file aside. A 23.27 Bridge has no such route, and a 23.28 Bridge behind it is refused, which is the skew rule's own direction.

| What | How |
|---|---|
| The definition is checked | `config::fit`: parsed against the machine's models, resolved against the Manifest |
| A definition that does not fit | 422 `fleet.workflow_unfit` with the loader's sentence; nothing is written |
| An id that cannot be a file's name | 422 `fleet.workflow_id_not_a_name` |
| An id the scope already holds, no `overwrite` | 422 `fleet.workflow_exists`, naming the file |
| A file that cannot be written | 500 `fleet.workflow_unwritable` |
| Held | Before the answer, and again after any hand edit to either folder, by a watch on both folders, with `armada.yml`'s settle window |

**No migration, no store change, and no event.** What a reader sees move is `list_workflows` and `list_left_out_workflows`, which are asked again. The generated TypeScript moves by the version constant only, since the DTO types are mirrored by hand and no Bridge reads these yet.

## Protocol 23.29: a Scout reads a stranded slot

The owner, 4 Oct 2026: when a worktree slot is stranded, send an agent to look at the work, then
choose to scrap it or stash it. `../concepts/scout.md`, *Starting from a stranded slot*, and
`../concepts/fleet.md`, *Rescuing a stranded slot*.

**Additive only.** One command, six DTOs and two optional fields on `WorktreeSlot`; a 23.18 Bridge
reads neither field and sees exactly what it did.

| Change | Where | Carries | Absent |
| --- | --- | --- | --- |
| `rescue_slot` | `POST /worktrees/slots/rescue?manifest_id=` | `RescueSlot { act, slot }` in, `SlotRescued { manifest_id, slot, branch?, branch_kept, committed? }` out. `act` is `start`, `stop`, `scrap` or `stash` | — |
| `stranded` | `WorktreeSlot` | `SlotStranded { uncommitted, commits, unpushed }`: what a Scrap would lose. Each commit is `{ sha, subject, home }`, `home` being `only_here` (on no remote branch and not on the local base), `on_remote` or `on_main` | A slot that is not stranded |
| `rescue` | `WorktreeSlot` | `SlotFinding { state, commit, uncommitted, cut, read, searched, verdict?, items?, summary?, why?, cost_micros? }`, `state` being `reading`, `answered`, `stopped` or `failed`. `verdict` is `unfinished` or `scraps`; `items` is what is left to do under the first and one line of leftovers under the second. `summary` is the Scout's own words, kept only where its answer was not that shape | No Scout has read it, or it has moved off the commit read |

**A Job's slot the Job could not give back is on the wire as kept.** `SlotHolding::Job` gains
`job_status` (where the Job ended), `kept` (why its release was refused, so its work is still in
the slot) and `completed`, all optional or defaulted. A kept slot carries `stranded` too, and
`rescue_slot` acts on it as on a stranded one; each act ends the Job's claim, so the slot is free
after.

**A bay a hosted Session holds is on the wire as that Session.** A Session's lease is written as a Job of
its own id, so `WorktreeSlot.held` was `job` with a Session id and Bridge drew the id. Fleet now serves it as
`SlotHolding::HostedSession { session_id, session_title? }`, and gives a process holder
(`SlotHolding::Session`) the optional `session_id` and `session_title` of the hosted Session whose agent started
it, found by walking down from each Session's agent (or its `session-keep`) to the holder's pid. Both are
additive or a new state; a Bridge that matches `held.state` without a default would miss the new one, which is
the major-bump rule above. A pid that no hosted Session started is served as before.

**Each refusal has its own code**, all 409s: `fleet.slot_not_stranded`, `fleet.slot_busy`,
`fleet.rescue_reading`, `fleet.rescue_not_running`, `fleet.rescue_on_no_branch`,
`fleet.rescue_on_the_base` and `fleet.rescue_no_remote`. `fleet.no_such_slot` is a 422.

**Bridge re-reads `GET /worktrees` while a Finding is `reading`.** No event carries it.

**Store V104 and V105**, `slot_rescues` (V105 adds `verdict` and `items`): one row per slot, keyed by Manifest and slot number, replaced as
the Scout reads and deleted by a Scrap or a Stash. A row left `reading` by a restart is set to
`failed` when Fleet starts.

## Protocol 23.33: where a workflow came from, one read whole, and a start that leaves out

The Workflow creator needs to draw each definition's place, the ones a more specific place replaced, and the one it is editing. A workflow file that does not fit stops refusing Fleet's start.

**Additive.** `WorkflowSummary` gains `file` and `overrides` (each an `OverriddenWorkflow { source, file }`, one per replaced place, absent where none). A new route, `GET /workflows/definition?workflow_id=&source=&manifest_id=`, `get_workflow`, answers a `WorkflowDefinition { workflow_id, source, file, definition, overridden_by? }`, where `definition` is the file's text in the shape `save_workflow` takes back. `source` is optional and absent means the one that runs; 422 `fleet.no_such_workflow_definition` where Fleet holds none by that id and place. `FleetHealth` gains `workflows_left_out`, a boolean and never a count, true where any served repository left a definition out. A 23.28 Bridge ignores all of it, and a 23.33 Bridge behind it is refused, which is the skew rule's own direction.

| What | How |
|---|---|
| A repository's own file that does not fit | Left out with its reason at start, as Kit's is; `list_left_out_workflows` carries it with `source: repository`. It no longer answers 422 `fleet.repository_refused` |
| Two files in one place with one id | Both left out, named together, at start and on a re-read alike |
| A definition a more specific place replaced | Not in `list_workflows`' rows; named on the winner's `overrides`, and read with `get_workflow` and its `source` |
| A step that routes back and names no `iteration_cap` | Takes five. Nothing else declares a back edge, so the passes cap is what bounds one |

**Not a protocol change, and the same release.** The workflow definition has no `structure` key any more; a file still writing it is left out as an unknown key. `verdict_routing` on any step is the only declaration of a back edge, and must name an earlier step. A Job's frozen copy never held the key, so no frozen Job moves.

**No migration, no store change, and no event.**

## Protocol 23.34: Helm is told when a person is on Workflows

The Workflow creator hands its draft to Helm, and Helm has to know which screen the person is on to read it as a workflow under edit.

**Additive.** `HelmScreen` gains `workflows`, sent in `AskHelm.context.screen` while the Workflow creator is the screen. Fleet's screen phrase for it is *Workflows*. A 23.33 Fleet refuses the value as an unknown variant, and a 23.34 Bridge behind it is refused by the skew rule's own direction, so no Bridge sends it to a Fleet that cannot read it.

**No migration, no store change, and no event.**

## Protocol 23.30: a Scout's Finding picked up

The owner, 5 Oct 2026: a stranded slot's Finding gets a third act beside Scrap and Stash, which
proposes a Job that continues the work from the slot's branch. `../concepts/fleet.md`,
*Rescuing a stranded slot*.

**Additive only.** One variant and one optional field; a 23.29 Bridge never sends the variant and
never sends the field.

| Change | Where | Carries | Absent |
| --- | --- | --- | --- |
| `pick_up` | `RescueAct` | Stash, then a proposal. The answer is `SlotRescued`, as a stash's | A 23.29 peer sends `start`, `stop`, `scrap` or `stash` |
| `continue_from` | `ProposeJob` | A branch the Job's worktree is cut from in place of the base. Kept as the proposal's landing `from_ref`, so the approval shows it and refuses it unless the repository holds it | Cut from the base, as every proposal before it |

**The seam is the landing the approval already has.** `from_ref` was set only at the approval; a
proposal can now arrive with it set, and the approval's own body still replaces it. A blank
`continue_from` is none.

**Pick up uses the proposer, not a workflow of its own.** Fleet assigns no workflow by default
(the `proposing` module (deleted since)), so the request, the branch and the Finding's items, goes to the
proposer as a dispatched request does, and the head of its plan is cut from the branch. A plan's
other Jobs wait on the head and are cut from the base.

**Two refusals, both 409s:** `fleet.rescue_unread` (no Finding of the slot has items or words to
carry) and `fleet.rescue_nothing_left` (its verdict is `scraps`). A stash's refusals apply too.

**No migration, no store change and no event.** The landing is the existing `job_landing` row.

## Protocol 23.31: what a step does, in words

The owner, 5 Oct 2026, at the approval gate on a workflow he had not used: *I have no idea what that
means.*

**One optional field, additive.** `WorkflowStep` and `StepDetail` gain `about`: a line or two on what
the step does for the Job and what it hands on. A workflow step declares it as `about:` beside
`label:`. Absent where the step wrote none, and a blank one is read as absent. `StepDetail` reads it
off the frozen workflow (`about` is written beside `label` in the Job's frozen steps), so a Job
frozen before 23.31 shows nothing there.

## Protocol 23.35: Accept changes the Kit

The owner, 5 Oct 2026: Accept on a Kit item in Retros should change the Kit, starting with allowed
commands. `../concepts/kit.md`, *Kit's allowlist*; `../concepts/retro.md`, *Items*.

**Additive only.** Optional fields, one DTO set and one route; a 23.32 Bridge sends none of the new
fields and reads past the ones it is sent.

| Change | Where | Carries | Absent |
| --- | --- | --- | --- |
| `scope` | `AnswerCommand` | `repository` or `kit`, read by an Always allow only. `kit` keeps the command in `~/.armada/allowed-commands` for every Job on this machine | The repository, which is what Always allow has meant since `#836` |
| `change` | `RetroItem`, `Lesson` | `{ "kind": "allow_command", "command": "…" }`, on a Kit item whose command Fleet copied off a refusal in the record | An item with no change, and every item kept before 23.35 |
| `applied` | `RetroItem`, `Lesson` | The change `agree_lesson` applied, on every later read | Nothing applied |
| `change` | `RetroAnswered` | What the retro model may write: `{ "kind", "refusal", "command"? }`. Read leniently, so one that will not read costs the change and not the item | Not asked for by a Fleet before 23.35 |
| `allowlist` row | `KitInventory` | Each command as `name` and where it came from as `source` (`retro item <id>`, `always allow`, `written by hand`), in place of *not read* | `not_read` with why, where the file will not read |
| `remove_kit_allowed_command` | `POST /kit/allowed_commands/remove` | Body `{ "run" }`; answers `KitAllowedCommands`, each row `{ "run", "source", "lesson_id"? }`. Helm only | A 409 `fleet.kit_allowlist_refused` where no line is spelled `run` |

**The Always allow value is a field and not a fourth `CommandAnswer`.** `CommandAnswer` is one Bridge
matches on, so a variant added there is a major bump by the table above; a `scope` on the answer is
read by Fleet alone. `offers` is unchanged, and a Fleet at 23.35 or later takes both scopes whenever
it offers Always allow.

**One migration, V110**, on `job_retro_items`: `change_kind`, `change_command` and `applied`, all
null or false on every item kept before. **No event.** Agreeing is the one place a retro acts and its
answer is the `Lesson`.

**One Fleet fitting, `Host::kit_home`**, which is not on the wire.

**Bridge's half**, mirrored by hand in `packages/protocol`: `RetroChange` and the `change` and `applied` fields on `RetroItem` and `Lesson` (`retro.ts`), `AlwaysAllowScope` and `AnswerCommand.scope` (`commanding.ts`), and `KitAllowedCommand`, `KitAllowedCommands` and `RemoveKitAllowedCommand` (`kit.ts`). Bridge sends `scope: kit` only with a `rule`, and the allowlist rows it draws are the `allowlist` kind of `get_kit_inventory`, read in place of *not read*.

## Protocol 23.36: a merge held behind a need

`merge_pull_request` gains one refusal, `fleet.merge_waiting_behind`, a 409 carrying `refused:
waiting_behind`. A Job that declared a need on a file (#1059) is refused its merge while a need ahead
of it on the same file stands, and the message names what it waits behind: what each declarer said it
needed, its branch, and what it took. It is Fleet's own press that is held, so a repository that says
`merge_by: forge` meets it as one that says `push` does; a person pressing the forge's button goes
around it. The Job stays at `awaiting_review`, and a press made again once the need has landed or been
given back goes through. No shape moves.

**Minor because a refusal code added is additive**, for 14.14's reason. The call a Drone declares a
need through, `declare_scope` and the tasks of `record_plan` and `add_task`, is the Drone seam and not
this one: it takes an optional `needs`, which is why it moves no number here.

## Protocol 23.37: Clear commits uncommitted files

The owner, 5 Oct 2026: freeing a worktree loses nothing once its work is committed on a branch.
`../../.claude/decisions/2026-10-05-a-branch-commit-is-enough-to-free-a-slot.md`; `../concepts/fleet.md`,
*Worktree slots*.

**Additive only.** One optional field; a Bridge before 23.37 reads past it and says nothing about
what was committed.

| Change | Where | Carries | Absent |
| --- | --- | --- | --- |
| `saved` | `WorktreeReclaimed` | `{ "commit", "files" }`: the WIP commit a Clear made on `branch.branch`, and the files it took | No uncommitted files, and every answer before 23.37 |
| `release` | `SlotAct`, in `ChangeSlotPool` | Commit a session-held slot's uncommitted files to its branch and free it | An older Fleet refuses the unknown act |
| `holder` | `ChangeSlotPool` | For `release`: the holder the person was shown, `claude (pid 44698)`. Required by it | Every other act |
| `stranded` | `WorktreeSlot` | Also on a session-held slot with uncommitted files, as `uncommitted` alone, with no commits | Every slot a job holds clean, and every read before 23.37 |
| `released` | `SlotPoolChanged` | `{ "branch", "saved"? }`, `saved` as above | Every act but `release` |
| `fleet.slot_holder_changed`, `fleet.slot_not_parkable` | 409 | The slot has another holder now, or git would not commit to a branch | |
| `fleet.slot_holder_unnamed` | 422 | A release naming no holder | |
| `fleet.slot_session_working` | 409 | A release of a bay a hosted Job holds, while its turn runs. `holder` is the Job's id, as `HostedSession` shows it. Not a new field or act | |

A release hands a slot an agent job holds back by the same park, and only for the holder named, so a slot re-leased since is refused. The job's process is not touched.

A bay a hosted Job holds is released the same way with the Job's id as `holder`, and is refused with `fleet.slot_session_working` while its turn runs, since the park ends every process in the slot.

`reclaim_worktree` no longer refuses a slot for uncommitted files. It commits them to the Job's
branch, never pushes, and frees the slot; a Job's own worktree outside the pool is committed the same
way before it is removed.

**Bridge's half**, mirrored by hand: `ReclaimedSaved` and `WorktreeReclaimed.saved` in `reclaimed.ts`; `SlotAct`'s `release`, `ChangeSlotPool.holder`, `SlotPoolChanged.released` and `SlotReleased` in `holding.ts`.

## Protocol 23.39: pause and resume a Job

`park_job` and `resume_job` are Fleet's pause and resume as commands, `POST /jobs/:job_id/park_job` and
`/resume_job`, no body, each answering the Job's `JobSummary`. `../concepts/job.md`, *Pausing a Job*.

**The names are `park_job` and `resume_job`**, because `pause_job` is a retired row for a different act
(a hold on a healthy Drone) that `xtask`'s unserved-operation allowance keeps on purpose.

**Additive only.** A new field, a new queued reason, two commands, two events and six refusal codes; a
Bridge before 23.39 reads past the field and the event, and draws a paused Job by its status.

| Change | Where | Carries | Absent |
| --- | --- | --- | --- |
| `paused` | `JobSummary` | `{ "by", "at", "resuming" }`: `person` or `fleet`, when, and whether a resume waits for a slot | A Job that is not paused, and every row before 23.39 |
| `paused` | `queued_reason` | A running Job that was paused reads `queued` with this reason | Every other queued Job |
| `park_job`, `resume_job` | commands | The Job's row, `paused` set or lifted | |
| `job.paused`, `job.resumed` | events | `{ "job", "actor", "at" }`, the row whole; a resume that waits carries `paused.resuming` | |
| `fleet.not_pausable` | 409 | A status that cannot hold a pause, or no slot to park | |
| `fleet.already_paused`, `fleet.not_paused` | 409 | A pause on a paused Job, a resume on one that is not | |
| `fleet.checks_running` | 409 | Checks are running again on the worktree; wait | |
| `fleet.pause_refused` | 409 | The pool or git would not park the work, or take it back, and why | |
| `fleet.paused` | 409 | A person's act on a paused Job: resume it first | |

**The marker rides beside the status.** A paused Job at a review gate reads `awaiting_review` with
`paused` set, so `queued_reason` is no place to look for it; it is only the reading of a running Job
that went back to `queued`. `job.state_changed` is silent for a gate Job, which is why a pause has its
own event.

`fleet.checks_running` was `fleet.not_resumable` for a re-run of the Checks and a merge gating a moved
base before this, and is now its own code for every act that meets it. No Bridge code read the old one.

## Protocol 23.40: who asked for a run, and a Drone's asked runs

The owner, 6 Oct 2026: a Drone's `run_checks` left no record, and no Check run shown anywhere said who asked for it. `../concepts/manifest.md`, *Who asked for a run*.

**Additive only.** One new DTO set, and optional or defaulted fields; a 23.36 Bridge reads past all of it.

| Field | Lands on | Carries | Absent |
| --- | --- | --- | --- |
| `Requester` | new, `ipc::Requester` | `kind` (an opaque string: `gate`, `drone_task`, `drone_step`, `merge_line`, `outside`) and `job_id`, `step`, `task_id`, `drone_id`, `branch` where the kind needs them | Never on the wire from 23.40; a reader treats none as `outside` |
| `requester` | `CheckRun` (every `StepDetail.check_runs` row, stamped `gate`) | who asked | A Fleet before 23.40 |
| `requester` | `ChecksUnderway`: `gate` on `checking`, the Drone on `dry_run` | who asked | As above |
| `requester` | `RunUnderway`, `RunRecord`, `CheckoutRunUnderway`, `CheckoutRunRecord`, `CheckoutVerify` | `outside`; on `RunUnderway` and `RunRecord`, the Drone (`drone_task` or `drone_step`) where a Drone started the run through the agent's door | A record written before 23.40 reads `outside` |
| `requester` | `MergeLineCheck` | `merge_line`, with the entry's `branch` | As above |
| `started_at` | `MergeLineCheck` | When the runner began the Check, an `Instant`, written by `armada land` into the line's state as the Check goes `running` | A Check still waiting, and a line state from before the field |
| `handle` | `Requester` | What a person calls the Job; a Drone's handle | Where Fleet does not know it |
| `asked_runs` | `StepDetail`, `RunList` | `AskedRun`: `id`, `requester`, `attempt`, `started_at`, `finished_at`, `state` (`running`, `passed`, `failed`, `stopped`, `lost`), `checks`, `narrowed`, `only_check`, `logs` | No run was asked |

**One read across Jobs, `list_manifest_checks`.** `GET /manifest/checks?manifest_id=` answers `ManifestChecks { rows, total, truncated? }`: every gate row and every asked run of the repository's Jobs (every served repository's where none is named), newest first, at most 200. A `ManifestCheckRow` is `source` (`gate` or `asked_run`), `requester`, `job_id`, `job_handle`, `job_title`, `step`, `attempt`, `group?`, `name`, `state`, `started_at?`, `ended_at?`, `took_ms?`, `logs[]` (`{ check, kept }`) and `asked_run_id?`. A gate row has no start or duration, only the write of its ruling as `ended_at`. A log opens through `get_check_output` on the row's `job_id` and `kept`, which now also resolves an asked run's logs. Merge-line Checks and checkout runs are not in it. `requester.handle` is the Job's handle, which is the Drone's handle: a Drone has no other name than its id.

**No version bump for the Drone stamp.** A `start_run` a Drone makes through the agent's door carries `drone_task` or `drone_step` where it carried `outside`: the same field and the same kinds, so a Bridge that follows a Drone requester already draws it. Fleet places the caller by its connection, and a caller it cannot place stays `outside`.

**An asked run is never a `CheckRun`**, so a dry result cannot be read as a gate's pass. It is its own row in `asked_runs`; a `running` row begun by a Fleet that is gone reads `lost`.

**One migration, V113**, `asked_runs`, pointing at `jobs`. **Bridge's half**, mirrored by hand in `packages/protocol`: `requester.ts` holds `Requester`, `AskedRun`, `requesterOf` and the kind spellings; `requester` and `asked_runs` are optional there so a fixture and a Fleet before 23.40 still type.

## Protocol 23.41: the hub on the merge line

`ipc::MergeLineHub`, additive on `MergeLine` as `hub`: main's CI as Fleet reads it off the forge and every open pull request with its `ci`. A line from a Fleet before 23.41 has none and a Bridge before it ignores one. `hub.main.state` and `hub.pull_requests[].ci` are strict sets Bridge picks a mark from, so a new value is a major.

**A repository with a hub and no queue is now in `get_merge_lines`**, with an empty line. That is the one thing an older Bridge sees: an empty panel where it drew none, which is a rendering and not a parse.

**Main's failed job log is not a new route.** `observe_land_check` takes the branch `main` and reads the job's log off the forge, so the lifeboat and the route table are untouched and Bridge still never reaches the forge.

**Event stream: neither better nor worse.** It adds no queue and no event kind. The two-second loop already published `merge_lines.changed` whole on a change; the hub makes that message larger (a hundred pull requests at most) and changes it when main or a pull request does, which is rarer than a queue turn.


## Protocol 23.42: acting on main's red, and what merged

Additive. `fix_main` (`POST /merge_lines/fix`, body `FixMain {root, job?, brief?}`) dispatches a Job for a red main or sends the work back to one, and answers with that Job's `JobSummary`. `JobSummary.fixes_main` (`FixesMain {state, check, test?, merge?, fixed_in?}`) is a Job's part in a red; `hub.fixing`, which 23.41 declared and never set, now names the Job on it; `hub.merged` lists the newest five pull requests merged into the base, read from the forge on the visit that already lists the open ones.

**Older peers:** a Bridge before 23.42 ignores every field and draws no buttons, so nothing breaks; it draws the line's own `landed` list. A Bridge at 23.42 against an older Fleet is refused as any minor ahead is, so the buttons never reach a Fleet without the route. `FixesMain.state` is read as working for a value it does not know, so a new one is not a major.

**One store table, additive** (`main_fix.main_ci_fixes`): which Job took each red. No other record changed.

## Protocol 23.43: the job ledger

`docs/concepts/session.md`. **Additive only**: three operations, one event kind and the DTOs of `ipc::sessions`, and nothing an older Bridge reads changes.

| Operation | Carries | Notes |
| --- | --- | --- |
| `report_session` (`POST /sessions/report`) | `SessionReport`: `harness`, `session_id` and one `fact`: `started`, `titled`, `moved`, `attached`, `settled`, `measured`, `tuned` (since 23.53), `turn_started` (since 23.86), `turn_completed` (with `said?` since 23.86), `touched`, `read`, `ran`, `wrote`, `permission_asked`, `permission_settled` or `ended` | Answers the whole `SessionRecord`. `agent_access` `No`: a harness reports, never the agent inside it |
| `list_sessions` (`GET /sessions?manifest_id=&q=&state=`) | `SessionList` | `q` finds a job by title, branch, pull request number or slot |
| `who_owns` (`GET /sessions/owner?kind=&target=&manifest_id=`) | `Owners`: every holder, standing ones first | `kind` is `branch`, `pr`, `job` or `slot` |
| `session.changed` (event) | `SessionRecord`, whole | Published on a fact that changed something; a repeat publishes nothing |

**`kind` on an attachment is open text, and `state` is not.** A kind Fleet has not met is kept as it arrived; every kind's state is `standing`, `spent` or `given_back`, so a released slot and a spent need are read the same way. A holder is `{ kind: session | job, id }` and no foreign key to either.

**One migration**, `session_ledger.tables`: `sessions` and `ledger_attachments`. Bridge's half is `packages/protocol/src/sessions.ts`, written by hand like the rest.

## Protocol 23.44: a held red, and each merge's run on main

Additive. `hub.main.checking` (`MainChecking {commit, pull_request?}`, newest first) names the newer commits whose CI is still running while main is red, and `hub.main.red_commit` the commit the red was read at. Main's state is the newest commit whose run has finished, so a green on an intermediate commit clears a red; `checking` on a `green` is only checks running. On a `red`, non-empty `checking` is a held red: `fix_main` refuses with `fleet.main_checks_running` and Fleet's own pickup waits. `hub.merged[].main_run` (`MainRun {state, failed?}`, `state` passed, running or failed) is the CI run on that merge commit on main itself. A failed job's log opens on `observe_land_check` under the branch `main@<commit>`, a new spelling of an existing free-text field.

**Older peers:** a Bridge before 23.44 ignores the three fields and draws the red band with its buttons, which Fleet then refuses while held. `state` stays `red` rather than gaining a variant, because Bridge branches on it and a new variant would be a major. A Bridge at 23.44 against an older Fleet is refused as any minor ahead is.

**Store columns, additive** (`main_ci.red_commit`, `main_ci.decided_commit`, `main_ci.newer_running`). Forge cost is in `docs/concepts/fleet.md`, *What Fleet knows about main's CI*.

## Protocol 23.46: needs on the job ledger

*Removed 17 Oct 2026, see "Needs are deleted".* **Additive only**: two operations and the DTOs of `ipc::needs`. A need is a row in `ledger_attachments`, which 23.43 made, so there is no migration, and nothing an older Bridge reads changes.

| Operation | Carries | Notes |
| --- | --- | --- |
| `act_on_need` (`POST /needs`) | `NeedCall`: `act` (`declare`, `took` or `release`), `branch`, `path`, and `what` or `value` | Answers `NeedAnswer`: the holder's need, whether it was `already` declared, who is `ahead`, and whether a `release` `gave_back`. `agent_access` `No`: a Drone runs `armada need` |
| `list_needs` (`GET /needs?manifest_id=`) | `NeedList`: every standing need, by path and in order | Each line names its holder as a person would: the branch, else the job's title |

**Fleet finds the holder.** `armada need` names a branch and Fleet resolves it to the Job whose branch it is, else the session standing on it, else the branch alone (a `session` holder with id `branch:<name>`). A `took` for a need nobody declared is `fleet.need_not_declared`, and a call that names nothing is `fleet.need_unnamed`, both 422.

**The ledger gains rows, not columns.** A Job's own slot and branch are written as `slot` and `branch` rows held by the Job, exclusive as a session's are, and `who_owns` answers them. `kind=need` is not exclusive. Bridge's half was a file of `packages/protocol`, written by hand like the rest, and is deleted.

## Protocol 23.48: acts on a pull request

`docs/concepts/session.md`, *Acts on a pull request*. **Additive only**: five operations, the DTOs of `ipc::pull_requests`, and one `origin` value. A pull request is named by its repository (a manifest id) and its number, and nothing here is a Job's. 23.49 is the session host's.

| Operation | Carries | Notes |
| --- | --- | --- |
| `get_pull_request` (`GET /pull_requests/:repository/:number`) | `PullRequestState`: `state` (`draft`, `open`, `merged` or `closed`), `auto_merge`, `queued` (in the base branch's merge queue now), `checks` (`pending`, `passed`, or `failed` with `failing` names), `title`, `branch`, `address`, `body` (the description as Markdown, left out where there is none) | `agent_access` `Yes`. Refreshes every Job's `pr` row for it, and the pull watch does each interval for pull requests nobody opened; a merged one is settled `spent`, a closed one `given_back`, in the same write as its detail |
| `ready_pull_request` (`POST /pull_requests/:repository/:number/ready`) | nothing | Answers `PullRequestState`. `agent_access` `Helm only`, asked as `pushes to shared` |
| `merge_pull_request_by_number` (`POST /pull_requests/:repository/:number/merge`) | nothing | Answers `PullRequestState`. Refused with `fleet.merge_checks_not_passed` unless the checks have passed. `Helm only`, `pushes to shared` |
| `enable_auto_merge` (`POST /pull_requests/:repository/:number/auto_merge`) | nothing | Answers `PullRequestState`. While the checks run, and once they have passed, when the forge queues it. `Helm only`, `pushes to shared` |
| `review_pull_request` (`POST /pull_request_reviews/:repository`) | `ReviewPullRequest`: `pull_request` (a number or an address), `session_id?` | Answers `ReviewDispatched {job_id, address, session_id?}`, a 201. `Helm only` |

**`merge_pull_request` was taken**, and is a Job's press at its gate, so the by-number sibling is `merge_pull_request_by_number` and the two are never one route. **`origin` gains `session_dispatched`** (*From a Session, by you*); a Bridge before 23.48 shows the raw spelling on such a row, since `origin` is a string on the wire. Which Session is the ledger's: the Session holds a `job` row with `detail.origin` reading *dispatched from Session <id>*.

**Refusal codes**, all 409 unless noted: `fleet.merge_checks_not_passed`, `fleet.merge_not_open`, `fleet.pull_request_is_a_draft`, `fleet.pull_request_not_a_draft`, `fleet.ready_refused` and `fleet.auto_merge_refused` (the forge's own sentence), and the forge's merge kinds `fleet.merge_branch_protected`, `fleet.merge_conflicted`; 500 for `fleet.pull_request_unreadable`, `fleet.merge_no_tool` and `fleet.merge_refused`; 422 for `fleet.pull_request_unnamed` and `fleet.session_unknown`. No migration. Bridge's half is `packages/protocol/src/pull-requests.ts`, written by hand like the rest.

## Protocol 23.49: a job Fleet hosts

`docs/concepts/session.md`, *A job Fleet hosts*. **Additive only**: eight operations, one event kind, one optional field on `SessionRecord`, and the DTOs of `ipc::hosted_sessions`. Nothing an older Bridge reads changes, and Helm's operations are untouched.

| Operation | Carries | Notes |
| --- | --- | --- |
| `start_session` (`POST /sessions/start`) | `StartSession`: `manifest_id`, and `title`, `model`, `effort`, `mode` | Answers the `SessionRecord`, `origin` `bridge`, `hosted` set, holding nothing |
| `send_session_message` (`POST /sessions/message`) | `SendSessionMessage`: `text`, `attachments` (`name`, `media_type`, base64 `data`), `mentions` (`SessionTag`) | **202**. Answers the `SessionRecord`; the reply is the thread's |
| `answer_session_ask` (`POST /sessions/ask/answer`) | `AnswerSessionAsk`: `session_id`, the ask's `call`, an `answer` it offered, a `note` | 409 where nothing is waiting under that call |
| `tune_session` (`POST /sessions/tune`) | `TuneSession`: `model`, `effort`, `mode` | A model or effort left out is the machine's own |
| `close_session` (`POST /sessions/close`) | `CloseSession` | Parks the slot, ends the row |
| `stop_session` (`POST /sessions/stop`) | `StopSession` | Ends the turn that is running, keeps the row live and the slot held. Since 23.84 |
| `get_session` (`GET /sessions/one?session_id=`) | `SessionThread`: the record and its `rows` | |
| `get_session_subagent` (`GET /sessions/subagent?session_id=&subagent_id=`) | `SessionSubagent`: the subagent's own `rows`, `finished`, and its `report` once it has; `model`, `context_tokens` and `context_window` from its last answer, each absent until it has answered | Read again while it runs; 422 where the job has no such subagent transcript |
| `get_session_file` (`GET /sessions/file?session_id=&file=`) | The bytes, under the media type they were sent as | `file` is a `SentFile.id` |
| `gate_session_call` (`POST /sessions/gate`) | `SessionGate`, answered in the harness's own hook shape | **Reached by the harness, never by a client** |
| `ask_from_terminal` (`POST /sessions/ask/terminal`) | `TerminalAsk`: `asks` with the question's input, or a permission's with its `tool`, answered at once with `TerminalAsked` `asked` and the call's id; `wait` with that call, held up to 25 seconds and answered with `answered` (the input and its `answers`), `refused`, `gone`, or `waiting` where it ran out; `settled` where the terminal's own prompt ended first | **Reached by the job's mod, never by a client.** Writes the same `ask` row a hosted job's question does, and none for a permission; the question and a Bridge answer are kept in the store until the mod collects them |
| `take_held_messages` (`POST /sessions/held`) | `TakeHeld`, answered with `MessagesHeld`: what a person sent a terminal job, once | **Reached by the job's mod, never by a client.** Since 23.53 |
| `session.row` (event) | `SessionRowChanged`: `session_id`, `row` | Appends, or replaces the row with that `id` |

**A row is `message`, `tool`, `lease` or `ask`**, tagged by `kind`. A message's `from` is `you`, `agent` or `session`, with the other job named. An ask carries the whole `HelmCallInFlight` and a `state`, and is replaced by id as it is answered. `session.changed` still carries the row whole, now with `hosted`: the `turn` (`idle`, or `working` with `woken_by` where another job started it), what the agent is `asked`, `model`, `effort`, `mode` and whether a process is `running`.

**`SessionRecord.hosted` is optional**, so a record a terminal reports reads as before. **One migration**, `session_host.tables`: `hosted_sessions` and `session_rows`. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest.

**Two changes that are not wire.** The agent door offers a hosted job what any agent of the person's is offered, and `ask_the_person`, so its permission tool is answerable; its acts are not recorded as Helm's. And `report_session` keeps `origin: bridge` when the mod in a hosted job reports `started`, and ignores its `ended`.

## Protocol 23.50: taking a Job over

the retired Pilot concept. *`take_over` and `get_handoff` are removed, see "Sessions absorb Jobs, group 1".* **Additive only**: five operations, the DTOs of `ipc::piloting`, one optional field on `JobSummary` and one migration. 23.48 is the pull-request acts and 23.49 is the session host's.

| Operation | Carries | Notes |
| --- | --- | --- |
| `take_over` (`POST /jobs/:job_id/take_over`) | `TakeOver`: `outcome` (`take_over` or `restart_step`), `session_id?`; or no body | Answers the Job's `JobSummary`, `piloted`. `Helm only`, asked as `destructive` |
| `get_handoff` (`GET /jobs/:job_id/handoff`) | nothing | Answers `HandoffBundle`: the Job's detail, its history, its evidence, the declared plans and the worktree's changed files marked against them, the worktree path and branch, where it stopped and the Drone's `narrative?`. `agent_access` `Yes` |
| `submit_for_verification` (`POST /jobs/:job_id/submit_for_verification`) | nothing | Held for the whole run. `Helm only`, `pushes to shared` |
| `attest_complete` (`POST /jobs/:job_id/attest_complete`) | `PilotNote`: `note?`; or no body | `Helm only`, `pushes to shared` |
| `close_as_superseded` (`POST /jobs/:job_id/close_as_superseded`) | `PilotNote`: `note?`; or no body | `Helm only`, `destructive` |

**`JobSummary.piloted`** is `Piloted`: `reason`, `session_id?`, `since`, and `exit?`, `ended_at?`, `note?` once the pilot ended. Absent on a Job nobody piloted, and on every row from a Fleet before 23.49. **`exit` reads `attested` on a Job a person completed**, and a Bridge draws that apart from a Job that passed its gates. Assist is not an `outcome`: a body naming it does not decode.

**Refusal codes**, all 409: `fleet.not_pilotable`, `fleet.already_piloted`, `fleet.not_piloted`, `fleet.pilot_busy`, `fleet.no_step_to_restart`, `fleet.nothing_to_verify`, `fleet.nothing_to_submit` and `fleet.steps_not_advanced`; and `fleet.paused` where the Job's worktree is parked. The migration adds `job_pilots`. Bridge's half is `packages/protocol/src/piloting.ts (deleted)`, written by hand like the rest.

## Protocol 23.51: a session started on a Job, and the commands an agent names

`docs/concepts/session.md`, *Started on a piloted Job*. *`StartSession.pilot` and the `handoff` row are removed, see "Sessions absorb Jobs, group 1".* **Additive only**: two optional fields, one row kind, and no operation. 23.50 is the take over and its exits.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_session` | `StartSession.pilot?`: `job_id`, `outcome` (`take_over` or `restart_step`) | Takes the Job over and starts the session on its worktree in one call. **`manifest_id` is not read** where `pilot` is set: the repository is the Job's. Refused with `take_over`'s own 409s and no session is written |
| `session.row`, `get_session` | `SessionRow` kind `handoff`: `job_id`, `number`, `title`, `reason`, `slot?`, `branch`, `step?`, `attempts`, `refusals`, `plan` (`declared`, `outside`, `unwritten`), `narrative?` | First in a piloted session's thread, once. **`plan` is Fleet's comparison and not two lists to compare**, since a declared path covers everything beneath it |
| `SessionRecord.hosted` | `commands?`: names, slash commands then skills | Off the stream's `init` line. Empty before an agent has started anywhere, and after a Fleet restart |

**A bundle that is the agent's first context is not a row.** Fleet renders it as prose and puts it ahead of the person's first message. A session closed or released from a pilot clears its own lease, so its next write leases a slot as any session's does. Bridge's half is in `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest.

## Protocol 23.52: a job's name

`docs/concepts/session.md`. **Additive only**: one operation and one optional field. 23.51 is a session started on a Job.

| Where | Carries | Notes |
| --- | --- | --- |
| `rename_session` (`POST /sessions/rename`) | `RenameSession`: `session_id`, `title` | Answers the `SessionRecord` and publishes `session.changed` when the title changed. Refused with 422, nothing written, for a blank title or a job Fleet has no row for. Not offered to agents |
| `report_session` | `SessionFact` `titled.named?` | `true` is a person's name from the terminal's `/rename`: it replaces the title. Absent, it is the first prompt's line, kept only where the job has no title |

**A name from Bridge stands until the next terminal `/rename`.** Both are the same column, so the later one is the one shown. Bridge's half is in `packages/protocol/src/sessions.ts`, written by hand like the rest.

## Protocol 23.56: a command and a compaction in a terminal job's thread

`docs/concepts/session.md`, *A terminal job's thread*. **Additive only**: two row kinds and nothing else. 23.53 is the terminal job.

| Where | Carries | Notes |
| --- | --- | --- |
| `session.row`, `get_session` | `SessionRow` kind `command`: `text` | A slash command or `!` shell line as the person typed it, `/reload-plugins` or `/model opus`. **No markup and no output**: the command's own output is not drawn |
| `session.row`, `get_session` | `SessionRow` kind `compaction`: `text` | The summary the CLI wrote in place of a conversation it compacted. It is neither the person's nor the agent's words |

Bridge's half is in `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest.


## Protocol 23.58: Triggers on the wire

`docs/concepts/trigger.md`. **Additive only**: four operations, one event, the DTOs of `ipc::triggers` and one optional field on `JobDetail`. 23.52 is a job's name.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_triggers` (`GET /triggers`) | nothing, or `?manifest_id=` | `TriggerList`: `triggers` (`name`, `when`, `workflow?`, `step?`, `runs`, `block`, `repair`, `level`, `file`, `skipped?`, `overrides`) and `left_out` (`level`, `file`, `said`). One object, because it starts as one. `Yes` |
| `get_trigger` (`GET /triggers/definition`) | `?when=&step=&name=&source=` | `TriggerDefinition`: the YAML text beside the identity, `level` and `overridden_by?`. 422 `fleet.no_such_trigger`. `Yes` |
| `save_trigger` (`POST /triggers/save`) | `SaveTrigger`: `scope` (`repository` or `machine`), `definition`, `overwrite?` | `TriggerSaved`, with `runs_from?`, `waits_for_main` and `skipped?`. 422 `fleet.trigger_unfit`, `fleet.trigger_name_not_a_name`, `fleet.trigger_exists`; 500 `fleet.trigger_unwritable`. `Bridge only` |
| `remove_trigger` (`POST /triggers/remove`) | `RemoveTrigger`: `scope`, `when`, `step?`, `name` | `TriggerRemoved`. 422 `fleet.no_such_trigger`. `Bridge only` |
| `job.trigger_changed` (event) | `JobTriggerChanged`: `job_id`, `trigger`, `at` | The row whole, on each state a firing reaches. Bridge re-reads the open Job |
| `JobDetail.triggers` | `JobTrigger`: `name`, `when`, `step`, `level`, `state`, `skipped?`, `exit_code?`, `started_at?`, `ended_at?`, `log_at?` | `pending` is a frozen Trigger no moment has reached. Absent from a Fleet before 23.58 and where empty |

**The event stream: slightly worse, and bounded.** At most two messages a firing, to open and to end, on the one drop-oldest channel. A Bridge that missed some re-reads `get_job`. The rate is `[broadcast-capacity]`'s to measure.

**`log_at` is an instant and not a line number.** The Job's log has no numbers, so the log line for a firing is stamped with the firing's own end, and `get_job_log` finds it by that `at` and its `trigger` field. Bridge's half is `packages/protocol/src/triggers.ts`, written by hand like the rest.

## Protocol 23.69: a job forked from an ended one

`docs/concepts/session.md`, *A forked job*; `docs/spikes/028-can-a-session-be-forked-from-a-dead-one.md`. **Additive only**: two optional fields, one migration and no operation. 23.54 is another branch's.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_session` | `StartSession.fork?`: `session_id` | Starts a job as a copy of an ended or dead one's conversation. 409 `fleet.session_fork_live` for a live one, 422 `fleet.no_such_session` for an unknown one. `manifest_id` is read as for any start |
| `SessionRecord.terminal` | `listening?`: `true` while the job's mod has asked within ten seconds | Absent is not listening. A live terminal job that is not listening is dead for Fork. Published as `session.changed` when it flips, while a window has the thread open |
| `SessionRecord.attachments` | kinds `forked_to` and `forked_from`, `target` the other job's id, `spent` | Open text as every kind is. **One migration**, `session_fork.hosted_fork_of`: `hosted_sessions.fork_of` |

## Protocol 23.62: a mod that is out of date

`docs/concepts/session.md`, *A mod that is out of date*. **Additive only**: one optional field on two facts and one on the record.

| Where | Carries | Notes |
| --- | --- | --- |
| `report_session` | `SessionFact` `started.mod_version?`, `tuned.mod_version?` | The version of the `armada` mod reporting. Kept on the job's row, and a fact without one leaves it |
| `session.changed`, `list_sessions`, `get_session` | `SessionRecord.mod_out_of_date?` | `true` for a terminal job whose mod is older than its repository's `plugins/armada/.claude-plugin/plugin.json`, or reported none. Absent otherwise |

**An older Bridge ignores the field, and an older mod sends none**, so a job running one is marked as soon as Fleet and the repository carry a version.

## Protocol 23.64: Checks reported while they wait

`docs/concepts/manifest.md`, *How many Checks run at once*. **Additive only**: no field and no operation, two new values and one id that was absent.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_manifest_checks` | A `gate` row with `state` `waiting` or `running` | One per Check of a running gate that has not finished, with no `ended_at` and, while waiting, no `started_at`. **Waiting covers every wait**: an earlier Check at the one-at-a-time gate, a prerequisite, a place, a machine slot. A finished Check is the stored ruling's row, as before |
| `list_manifest_checks` | An `asked_run` row with `state` `waiting` | A Drone's run that has started none of its Checks. Reads `running` once one starts |
| `get_merge_lines` | `MergeLineCheck.requester.job_id` and `handle` | On a branch some Job owns. Absent on one none does. A Bridge from before 23.64 ignores them |
| `get_merge_lines` | `MergeLineEntry.checks` on a queued branch | A branch in `line` with state `waiting` that a Job owns lists the Manifest's declared Checks, each `waiting` and naming the Job, until the line gates it and writes its own. Absent on a branch no Job owns |

**A known cost of calling this minor.** A Bridge from before 23.64 has no `waiting` in its gate vocabulary and draws such a row failed, so a Fleet ahead shows a waiting Check on that Bridge's Checks page as red. The banner for a Fleet ahead does not say so. Bridge's half is `packages/screens/src/manifest-checks.ts`.
## Protocol 23.68: steps added to one Job, and the repository's Draft default

`docs/concepts/trigger.md`, *Steps added to one Job*. **Additive only**: two operations, one event, the DTOs of `ipc::added_steps`, one optional field each on `ApproveDispatch`, `JobDetail`, `SaveTrigger` and `ManifestDeclared`, and one edit. The repair, the hold and the Draft default below are the same minor, landed together; `armada need` gave the number after the branches ahead of it.

| Where | Carries | Notes |
| --- | --- | --- |
| `add_job_step` (`POST /jobs/:job_id/add_job_step`) | `AddStep`: `runs` (`script` and `command`, `skill` and `skill`, or `drone` and `brief`), `when`, `step`, `block?`, `repair?` | `AddedStep`. 409 `fleet.added_step_behind` (`reason`: `started`, `passed` or `ended`), 409 `fleet.added_step_before_approval`, 422 `fleet.unacceptable_addition`. `Bridge only` |
| `remove_job_step` (`POST /jobs/:job_id/remove_job_step`) | `RemoveAddedStep`: `id` | `AddedStepRemoved`. 409 `fleet.added_step_fired`, 422 `fleet.no_such_addition`. `Bridge only` |
| `approve_dispatch` | `additions?`: a list of `AddStep` | Placed at the press. Left out keeps what was placed, `[]` clears it. 422 `fleet.unacceptable_proposal` |
| `save_trigger` | `kept_from?`: `{ job_id, addition_id }` | **Keeps an addition for every Job** with no new operation: the definition is the Trigger the editor drew, and Fleet records where it went. A Script or a Skill only |
| `job.addition_changed` (event) | `JobAdditionChanged`: `job_id`, `addition`, `removed?`, `at` | The row whole, on each add, removal, keep and state a firing reaches |
| `JobDetail.additions` | `AddedStep`: `id`, `runs`, `when`, `step`, `block`, `repair`, `placed`, `added_at`, `state`, `skipped?`, `exit_code?`, `started_at?`, `ended_at?`, `log_at?`, `kept?` | `state` reuses `TriggerFiringState`: `pending` until the moment has come. Absent from a Fleet before 23.68 and where empty |
| `edit_manifest` | the edit `set_pr_mode`: `{ "edit": "set_pr_mode", "pr_mode": "draft" \| "ready" \| null }` | `null` removes the key |
| `ManifestDeclared` | `pr_mode?` | Absent where the file defers to this machine's default |

**A place is a moment and a step**, a Trigger's. Before a step is its `step_starts`, after it is its `step_passes`, the gap before the pull request opens is the delivering step's `step_starts` and the gap after is `pr_opened`. A gap behind the Job's current step is refused, which is what stops a step landing where it can never fire.

**A Drone step is recorded `skipped`, and says so.** A step a Drone works needs a gate and Fleet has the frozen workflow's step rows only. A Skill is skipped as a skill Trigger is. Both run on a side Drone from 23.73, below.

**The event stream: slightly worse, and bounded.** At most two messages a firing and one per act a person makes, on the one drop-oldest channel, with nothing a Drone produces on it. A Bridge that missed some re-reads `get_job`. The rate is `[broadcast-capacity]`'s to measure.

**Skew.** A Fleet before 23.68 sends no `additions` and no `pr_mode`, which Bridge reads as none. A Bridge before it sends no `additions` and no `kept_from`, which Fleet reads as every call before. Nothing a 23.58 peer reads changes. Bridge's half is `packages/protocol/src/added-steps.ts (deleted)`, written by hand like the rest.

## Protocol 23.68: a failed Trigger's repair on the wire

`docs/concepts/trigger.md`, *A failed Trigger with `repair` on*. **Additive only**: one operation, three states and one optional field.

| Where | Carries | Notes |
| --- | --- | --- |
| `JobTrigger.state`, `JobTriggerChanged.trigger.state` | `repairing`, `rerunning`, `fix_ready` | A failure with `repair` on stops at `repairing`, not `failed`. A Bridge that does not know them draws an unknown state |
| `JobTrigger.repair?` | `TriggerRepair`: `attempt`, `branch?`, `files`, `choice?`, `pull_request?` (`url`, `number?`) | Present from the first repair Drone. `attempt` is 1 or 2 and no client draws it as a count. `files` is what the fix changes, set when it is held |
| `choose_trigger_fix` (`POST /jobs/:job_id/choose_trigger_fix`) | `ChooseTriggerFix`: `trigger`, `choice` (`this_branch` or `new_pr`) | `TriggerFixChosen`: `state`, `pull_request?`. 409 `fleet.no_fix_waiting`, `fleet.fix_not_placed`, `fleet.fix_conflicts`, `fleet.fix_waiting`. `Bridge only` |

**A fix waiting on the owner is an alert already**: `list_alerts` carries it, and a Trigger that failed after both tries. Nothing new is read for it. Bridge's half is `packages/protocol/src/triggers.ts`, written by hand like the rest.

## Protocol 23.68: a Trigger that blocks holds the Job

`docs/concepts/trigger.md`, *A failed Trigger with `block` on*. **Additive only**: two operations, one state, one skip reason, one escalation reason and two optional fields.

| Where | Carries | Notes |
| --- | --- | --- |
| `JobTrigger.state`, `AddedStep.state` | `held` | A failure with `block` on and no repair, or a repair that did not fix it. A Bridge that does not know it draws an unknown state, and the Job's own status says it is waiting |
| `JobTrigger.blocks?` | `true` where the Trigger blocks | Left out where it does not. It tells `repairing`, `rerunning` and `fix_ready` on a Trigger that holds the Job from one that does not |
| `TriggerSkipReason`, `AddedSkipReason` | `by_owner` | The owner skipped it while it held the Job |
| `escalation_reason` (`JobSummary.reason`) | `trigger_held` | A Job stopped before a Drone by a hold at `step_starts` or `step_passes`. Job-level, so no step is named |
| `rerun_trigger` (`POST /jobs/:job_id/rerun_trigger`) | `HoldAct`: `trigger` or `addition`, one | `HoldSettled`: `state`, `released`. 409 `fleet.no_hold`, `fleet.hold_repairing`, `fleet.hold_has_a_fix`, `fleet.hold_nothing_to_run`, `fleet.hold_no_worktree`, `fleet.hold_job_working`, 422 `fleet.no_hold_named`. `Bridge only`. It may take as long as the Command |
| `skip_trigger` (`POST /jobs/:job_id/skip_trigger`) | `HoldAct` | `HoldSettled`. 409 `fleet.no_hold`, `fleet.hold_repairing`. `Bridge only` |
| `approve_review`, `merge_pull_request` | | **A new 409**, `fleet.trigger_holds`, at a gate a `pr_opened` hold stands in front of. Nothing moves |
| `JobSummary.alert?` | `JobAlert`: `kind` (`held`, `fix_ready`, `failed`), `trigger`, `when`, `step` | **The Board row's bell**: a hold, a repair fix waiting on the owner's choice, a Trigger that failed after its repair tries. Today the bell shows only on the open Job. Absent is none |

**`list_alerts` says which Trigger**: a Job a hold stopped is already in `blocked`, and its `why` names the Trigger. A `pr_opened` hold on a Job at its gate is in `waiting` with the same words.

**The event stream: no change.** Every state a hold passes through is a `job.trigger_changed` or `job.addition_changed` that already existed, and a rerun adds one transient `rerunning` that is not stored.

**Skew.** A Fleet before 23.68 holds nothing and sends no `blocks`, no `alert` and no `held`, which Bridge reads as every row before. A Bridge before it draws `held` as an unknown state and has no Rerun or Skip, and the Job it cannot answer is `escalated`, whose acts it already offers. Bridge's half is `packages/protocol/src/trigger-holds.ts (deleted)`, written by hand like the rest.

## Protocol 23.68: a draft default for pull requests

the retired Landing concept, *What the landing rule carries*. **Additive only**: two optional fields and no operation.

| Where | Carries | Notes |
| --- | --- | --- |
| `get_job` | `JobDetail.pr_mode_default?`: `ready` or `draft` | What a Job still at its approval gate opens as when its approval says nothing: the workflow's delivering step, the repository, this machine, then ready. Absent once `landing` is there, and from a Fleet before 23.68 |
| `get_preferences`, `save_preferences` | `Preferences.draft_pull_requests?`, and the name `draft_pull_requests` for a save | This machine's default, off until set. Absent is `false` |

**Bridge starts the draft choice on `pr_mode_default` where `landing` is absent.** A Job approved with no `landing.pr_mode` takes the same answer, so a Bridge that never learned the field still gets the default. **The default Fleet serves is for the workflow the Job was proposed on**: a person who picks another workflow in the proposal sees the first one's until the approval, and what is frozen is the picked workflow's. Bridge's half is in `packages/protocol/src/detail.ts (deleted)` and `preferences.ts`, written by hand like the rest.

## A saved Trigger runs a Drone

`docs/concepts/trigger.md`, *A Skill or a Drone step*. **Additive only**: one variant of `TriggerRuns` and no new operation, state or error. A saved Trigger can name a prompt, and a Drone is sent with it on a side Drone as a Skill is.

| Where | Carries | Notes |
| --- | --- | --- |
| `TriggerRuns` | `{ kind: "drone", brief }` | The prompt, named as an added Drone step's is. A Trigger file spells it `brief:`, and an empty one is refused naming `brief`. `TriggerSummary.runs` serves it |
| `JobTrigger.drone` | `true` | Now also where the Trigger names a prompt |
| `save_trigger` `kept_from` | | A Drone step can be kept for every Job. 422 `fleet.unacceptable_addition` is no longer raised for one |

**`repair` is ignored**, as for a Skill: a file with `brief:` and `repair: true` loads with it off.

## Protocol 23.78: a Job's retro in its thread

`docs/concepts/retro.md`, *A Job's retro*. **Additive only**: one thread row, no operation, state or error.

| Where | Carries | Notes |
| --- | --- | --- |
| `SessionRow` kind `retro` | `id`, `at`, `stage`, `stage_since`, `characters`, `items`, `retro?`, `why?` | Added when a press is accepted, replaced by id as the retro is written, kept when done. `stage` is `asking_agent`, `reading_record`, `writing`, `written` or `failed`. `characters` is how much of the model's answer has arrived. `items` is a list of `PartialRetroItem` (`title?`, `what?`, `fix?`, `who?`, `lands_in?`), each field present once its value has ended, and is empty outside `writing`. `retro` is the kept retro's number on `written`. `why` is Fleet's words on `failed` |
| `session.row`, `get_session` | The row above | A hosted Job only. `write_session_retro` still answers when the retro is written |

**A press that is refused, and one answered with the last retro, adds no row.** A row a restart left going is returned by `get_session` as `failed` with `why` "Fleet restarted while it was written". Replacements are at most four a second; a stage change and the last state are always sent.

**Skew.** A Bridge before 23.78 reads a `retro` row as a row kind it does not know and draws nothing for it, or draws it badly; it still reads the retro from `write_session_retro`. A Fleet before it sends no such row. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand.

## Protocol 23.77: Start Job on a retro item

`docs/concepts/retro.md`, *Agree and disagree*. **Additive only**: one operation, `Helm only` like `agree_lesson`, and one optional field on two types.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_lesson_session` (`POST /lessons/:lesson_id/session`) | No body. Answers `LessonSessionStarted`: `lesson`, `session_id`, `brief`, `send_failed?` | `agree_lesson`'s sibling: a hosted Job on the item's repository, started as `start_session` starts one, and the brief sent to it as its first message at once, so the Job runs. `brief` is the text that was sent. `send_failed` is Fleet's words for why it was not, where the send was refused, and is absent otherwise |
| `Lesson.session_proposed`, `RetroItem.session_proposed` | The Job id, optional | State `agreed`. At most one of it and `job_proposed` is set. Absent from the wire when none |

**Refusals.** As `agree_lesson`: 404 `fleet.no_such_lesson`; 422 `fleet.lesson_names_no_place`, `fleet.lesson_armada_not_served`, `fleet.lesson_session_names_no_repository`. 422 `fleet.lesson_session_nothing_to_work_on` for a `kit` item. 409 `fleet.lesson_not_open` for an item answered with no Job to hand back. **A Job that started and whose first message was refused is a 200 with `send_failed` set**: the item is `agreed` with the Job on it, so Bridge can open it and the person can send by hand. A second press answers 200 with that Job, sets `send_failed` again, and sends nothing.

**One press makes one Job and one message.** The write that moves the item off `open` is the claim. An item already agreed with a Job answers 200 with it.

**Skew.** A Fleet before 23.77 has no route and answers 404, and never sends `session_proposed`, which Bridge reads as absent. A Bridge before it never calls the route and ignores the field. Bridge's half is `packages/protocol/src/retro-session.ts`, written by hand.

## Protocol 23.78: a slash command run in a terminal Job

`docs/concepts/session.md`, *Running a block*. **Additive only**: one operation, `agent_access: No`, and its type. No type that exists changes.

| Where | Carries | Notes |
| --- | --- | --- |
| `run_session_command` (`POST /sessions/command`) | `RunSessionCommand`: `session_id`, `command`. Answers 200 `SessionRecord` | Holds the command for the Job's own mod as a `HeldCommand` (name without the slash, the rest as `args`), the path `tune_session` takes for `/model`; the mod runs it as typed on its next ask and nothing comes back. 422 `fleet.session_command_refused` where the Job is unknown or not a terminal one, or the text is not one slash command on one line (`/name` or `/name args`, a name of lower-case letters, digits, `:` and `-`); 409 `fleet.session_closed` where it ended, and `fleet.terminal_session_unreachable` where its mod is not listening |

**Skew.** A Fleet before 23.78 answers 404 and Bridge shows the refusal as for any act on a Job. A Bridge before it never calls the route. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest.

## Protocol 23.89: a run's end is a row in the thread

`docs/concepts/session.md`, *Running a block*. **Additive only**: one thread row, and no operation changes.

| Where | Carries | Notes |
| --- | --- | --- |
| `session.row`, `get_session` | `SessionRow` kind `ran`: `id`, `at`, `command`, `code?`, `tail` | A hosted Job's thread keeps one when a run ends, in place of the message from Fleet it kept before. `code` is absent where the run was stopped or a signal ended it. `tail` is the last 40 lines as a person would have read them, escapes and carriage-return overwrites resolved: the text the agent is sent, and empty where the run printed nothing |

The agent is sent the same words as before (the command, `Exit <code>.` or `Stopped.`, the last lines, and the note that Fleet took a waiting item off), **once**, and the thread shows no message from Fleet beside the row. A terminal Job's thread is the transcript its CLI keeps and Fleet cannot write to it, so it has no `ran` row and its mod is still handed the words.

**Skew.** A Bridge before 23.89 reads a `ran` row as a kind it does not know and draws nothing for it, or draws it badly. A Fleet before sends the message. The IDs differ, so the two are refused regardless, and the lifeboat serves. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest.

## Protocol 23.88: a Job's run is on a terminal

`docs/concepts/session.md`, *Running a block*. **Two things change that exist, two operations are new.** Both operations are `agent_access: No`: running a block and typing into it are the person's own acts.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_session_run` | `StartSessionRun` gains optional `cols` and `rows` | The command runs on a pseudo-terminal of that size (80 by 24 where absent) with `TERM=xterm-256color`, as the job leader with the terminal as its controlling terminal. 422 for a `cols` or `rows` of 0 |
| `observe_session_run` | `SessionRunMessage` loses `lines` and gains `output` (`text`) | `text` is the terminal's bytes as UTF-8 with the escapes intact, sent as they arrive. A character a read ends inside waits for the next message, so none is split. A late viewer's first `output` is the held tail, the last 256 KiB, replacing the last 2000 lines |
| `write_session_run` (`POST /sessions/run/write`) | `WriteSessionRun`: `run_id`, `data`. Answers 200 `SessionRunUnderway` | `data` is written to the terminal as it is. Ctrl-C is the byte 3 and reaches the command as SIGINT. 422 on a run that has ended or Fleet does not hold, the shape a stop has |
| `resize_session_run` (`POST /sessions/run/resize`) | `ResizeSessionRun`: `run_id`, `cols`, `rows`. Answers 200 `SessionRunUnderway` | 422 as `write_session_run`'s, and for a size of 0 |

**The Job is told when the run ends**, as Fleet tells a Job anything: the command, `Exit <code>.` or `Stopped.`, and the last 40 lines with escapes and carriage-return overwrites resolved. A `run` item in the Job's `waiting_for` whose target is the command, trimmed, is taken off the list in the same moment when the exit is 0. Bridge no longer answers it. A Job that has ended is not told. *Protocol 23.89 keeps a hosted thread's telling as a `ran` row.* A slash command sent to a terminal Job is no run, and Bridge still settles its item.

**Skew.** A Fleet before 23.88 sends `lines`, which a newer Bridge ignores, and answers 404 to the two routes. A Bridge before it ignores `output`. The IDs differ, so the two are refused regardless. Bridge's half is `packages/protocol/src/sessions.ts`, written by hand like the rest. There is no number to take: `armada need --status` showed nothing standing on these paths.

## Protocol 23.76: a shell block run from a Job's thread

`docs/concepts/session.md`. **Additive only**: three operations, all `agent_access: No`, and their types. No type that exists changes.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_session_run` (`POST /sessions/run/start`) | `StartSessionRun`: `session_id`, `command`. Answers 202 `SessionRunUnderway`: `run_id` | Runs `$SHELL -lc <command>` (`/bin/zsh` where `SHELL` is unset or empty) in its own process group, in the worktree of the slot the Job holds, else the Job's directory. Output and errors are one stream. 422 `fleet.session_run_refused` for a blank command, an unknown Job, an ended one, or a directory the command cannot start in |
| `stop_session_run` (`POST /sessions/run/stop`) | `StopSessionRun`: `run_id`. Answers 200 `SessionRunUnderway` | SIGTERM to the group, SIGKILL after 3 seconds. 422 on a run not in flight |
| `observe_session_run` (`GET /sessions/run/:run_id/observe`, WebSocket) | `SessionRunMessage`, tagged by `kind`: `opened` (`protocol_id`, `run_id`), `lines` (`lines[]`), `missed` (`dropped`), `exited` (`code`: integer, or `null` where the run was stopped or signalled) | `exited` is sent once, last, then the socket closes. 422 on a run Fleet does not hold |

**Nothing is written down.** A run's output is held in memory while it lives and for 90 seconds after it exits, so an observer that opens late replays it and then `exited`. After that the run is unknown. *Protocol 23.88 moved the run onto a terminal and replaced `lines` with `output`.*

**The event stream.** None of this rides `/events`. Each observer has a bounded broadcast of 256 messages and is told `missed` with how many it lost, so a slow viewer does not slow the run and does not believe a gap is whole; on `missed` it should reopen the socket, which replays the held tail. This is the bounded-with-a-count shape the open WebSocket risk asks for, one subject over.

**Skew.** A Fleet before 23.76 has neither route and answers 404, so a Bridge that offers the block has nothing to call. A Bridge before it never asks. Bridge's half is `packages/protocol/src/sessions.ts`, written by hand like the rest.

## Protocol 23.75: a Job's Checks

`docs/concepts/session.md`, *A Session's Checks*. **Additive, with one loosening**: two operations, a requester kind and a thread row, and `ManifestCheckRow`'s Job fields become optional.

| Where | Carries | Notes |
| --- | --- | --- |
| `report_session_check` (`POST /sessions/checks`) | `SessionCheckCall`: `act` (`start` or `end`), `cwd`, `name`; on `end` also `run`, `outcome`, `took_ms`, `log`. Answers `SessionCheckAnswer`: `run?` | `armada check` is the caller. No `run` where no Job holds the slot |
| `get_session_check_output` (`GET /sessions/checks/:run/output`) | Answers `CheckOutput` | Keyed by run id, since a Session's run has no Job |
| `Requester` kind `session` | `session_id`, `slot` | |
| `SessionRow` kind `check` | `name`, `run`, `state`, `target?` | Replaced by id as the run ends. `target` is what the run was narrowed to, absent where it ran whole; optional on `SessionCheckCall` too, where a `--changed` run tells it on `end` |
| `ManifestCheckRow.job_id`, `job_handle`, `job_title`, `step` | Now optional | Absent on a Job's run, which carries `session_id` and its run id in `asked_run_id` |

**Skew.** A Bridge before 23.75 reads a row with no `job_id` as a Job row with a missing id, so it draws that row badly; it never asks for the new routes. A Fleet before it sends neither the rows nor the thread row.

## Protocol 23.74: a guided review of the open retro items

`docs/concepts/retro.md`, *Reviewing*. **Additive only**: two operations, both `Helm only` like `agree_lesson`, and no change to a type that exists. Neither stores anything.

| Where | Carries | Notes |
| --- | --- | --- |
| `review_lessons` (`POST /lessons/review`) | `ReviewLessons`: `manifest_id?`; the body may be left out. Answers `LessonReview`: `model`, `entries[]`, `set_aside[]` | One model call over the open items, read as `list_lessons` reads them (up to 200). `entries` are best first and carry no rank. `ReviewEntry`: `lesson_id`, `merged_ids[]`, `reason`. `SetAside`: `lesson_id`, `why`. No open item answers empty with `model` empty and makes no call. 500 `fleet.lesson_review_failed` |
| `ask_lesson` (`POST /lessons/:lesson_id/ask`) | `LessonAsk`: `question`, `history[]` of `AskTurn` `role` (`person` or `fleet`), `text`. Answers `AskLessonAnswer`: `answer` | One model call, answered from the item and the record rows it cites. 404 `fleet.no_such_lesson`; 422 `fleet.lesson_question_refused` for an empty question, one over 2000 characters, more than 20 turns or a turn over 4000 characters; 500 `fleet.lesson_ask_failed` |

**The answer of a review is held to the open items.** An id the model invented is dropped. An id that leads an entry is kept once, so it is taken out of every `merged_ids` and out of `set_aside`. An item set aside needs a `why`. **An open item the model named nowhere is appended to `entries` with the reason `not ranked`**, so nothing a person has not answered goes missing from the queue. Bridge keeps the conversation, because Fleet holds no thread.

**Skew.** A Fleet before 23.74 has neither route and answers 404, so a Bridge that offers the review has nothing to call and shows the plain list. A Bridge before it never calls them. Bridge's half is `packages/protocol/src/retro-review.ts`, written by hand like the rest.

## Protocol 23.87: one file of a pull request, read when it is opened

`docs/concepts/session.md`, *Acts on a pull request*. **Additive only**: one operation and the DTOs `PullRequestFileOf` and `PullRequestFilePatch`. `PullRequestPage` is unchanged, so no patch rides it.

| Operation | Carries | Notes |
| --- | --- | --- |
| `get_pull_request_file` (`GET /pull_request_files/:repository/:number?path=`) | `PullRequestFilePatch`: `path`, and `patch` or `said` | `patch` is the forge's hunks for the file with no `diff --git` header. A file the forge sent none for (binary, or too large) is a 200 with `said` in Fleet's words. A path the pull request does not list is a 422 `fleet.pull_request_file_unknown`; a forge that would not answer is a 500 `fleet.pull_request_unreadable`. `agent_access` `Bridge only` |

**Read from the forge with `gh api repos/{owner}/{repo}/pulls/N/files`**, the one read that carries a patch per file and leaves it out for a file it will not show. Fleet reads the pages and keeps the one file, so a person opening a file pays for the pull request's files and not for a patch on every page read.

**Skew.** A Fleet before 23.87 has no route and answers 404, and the IDs differ, so the two are refused regardless. Bridge's half is `packages/protocol/src/pull-requests.ts`, written by hand like the rest. There is no number to take: `armada need --status` shows nothing standing, and the sections above carry no ordering to follow.

## Protocol 23.86: a Job's standing, and the replied and done items

`docs/concepts/session.md`. **Additive only**: one optional field, one enum, two `WaitingSource` values, one new `SessionFact` and one optional field on another. No new operation. **Nothing the owner sees may depend on an agent making a tool call**: Fleet reads what a Job is doing from the turn, the thread and `done_since`, and the agent's own `waiting_for` and `all_done` only add detail.

| Where | Carries | Notes |
| --- | --- | --- |
| `SessionRecord.standing?` | `SessionStanding`: `working`, `stuck`, `replied`, `done`, `idle` | The one answer the Jobs list, the Cockpit's tiles, lanes and calls all draw. Absent from a Fleet that does not say, and Bridge then reads the turn alone |
| `SessionFact::TurnStarted` (`report_session`) | nothing | The terminal mod tells a prompt went in. Fleet has the Job working and forgets the reply the turn before left. Never sent for a slash command. A Fleet before 23.86 refuses the fact, which the IDs make moot |
| `SessionFact::TurnCompleted.said?` | The first line of what the turn ended on, at most 200 characters | Left out when the turn said nothing, never sent as `null`. Fleet keeps it with the turn's end in `session_turn_end` (store migration `20261014T1000Z`) and uses it only to tell a reply from a turn that said nothing |
| `WaitingItem.source` | `replied`, `done` | `replied` is derived on every read, never kept: id `replied:<turn ended at>`, text the sentence of the reply that asks, no act. It is raised only for a reply whose close asks the person something and that the agent did not ask through a tool; a reply that only reports is the `replied` standing and no item. It is dismissable, dismissing sends the agent nothing, and a dismissed id never returns. `done` is derived on every read, never kept: id `done:<done_since>`, `since` that instant, text `Done`; Bridge offers End job (`close_session`) and Keep going (`dismiss_waiting`). It is dismissable, and a dismissed id never returns |
| `DismissWaiting.reason?` | Why the person removed the item | Left out where he gave none, never sent as `null`. An item the agent stated (`waiting_for`) tells it so: the Job is sent `Owner removed "<item text>"`, with `: <reason>` after it where the reason is not blank. A derived item is only dropped. |

**Skew.** A Fleet before 23.86 serves neither, and Bridge reads a Job as before. The IDs differ, so the two are refused regardless, and the lifeboat serves. Bridge's half is `packages/protocol/src/sessions.ts`, written by hand like the rest. **Served.** Fleet derives `standing` and the `done` item from observed facts on every read (`session_host/standing.rs`), for hosted and terminal Jobs, and none of it needs the agent to call a tool. A running turn nothing is heard from for ten minutes reads `stuck`, which no event says, so the notice loop's tick republishes the Job when that moves: a `session.changed` per move, in both directions, and no other new traffic. `Timeline::session_turn` no longer marks its entry `live`, since a finished turn is not moving. A Helm Job has no `done` item, never replies and is never stuck. A terminal turn's running state is kept in memory, so a Fleet restart reads it as idle until the next `turn_started`; a mod older than `0.3.20` reports no `turn_started` or `said`, so its Job is never read as working or stuck and has no reply, and its `done` still stands.

## Protocol 23.85: the model a hosted Job resolved

`docs/concepts/session.md`, *Which model, and a fresh start*. **Additive only**: one optional field on `HostedFacts`. No operation, event or error is new.

| Where | Carries | Notes |
| --- | --- | --- |
| `HostedFacts.resolved_model?` (`session.changed`, `list_sessions`, `get_session`) | The model the CLI resolved for the running process, as its stream's `init` line names it: `claude-opus-5-5` | Absent until the first `init` of a process, and again once the process is let go (a stop, a tune, a quiet spell). **Not `model`**, which is the alias a person picked and is absent when none was: a job with no pick still runs on something. Left out when empty, never sent as `null` |

**Skew.** A Fleet before 23.85 sends none, so a Bridge names the picked alias and draws no chip where none was picked. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest, and `resolvedModel` on the Job the window draws (`packages/screens/src/sessions-wire.ts`).

## Protocol 23.83: a pull request's page and the writes on it

`docs/concepts/session.md`, *Acts on a pull request*. **Additive only**: two operations and the DTOs of `ipc::pull_requests` that go with them. The type is `PullRequestPage` and not `PullRequestDetail`, which a Job's live pull request snapshot already is.

| Where | Carries | Notes |
| --- | --- | --- |
| `get_pull_request_detail` (`GET /pull_request_details/:repository/:number`) | `PullRequestPage`: `base`, `author?`, `created_at?`, `additions`, `deletions`, `mergeable`, `review`, `approvals`, and `conversation[]`, `checks[]`, `files[]`, `commits[]` | One forge read. Comments and reviews merge in time order. Capped at the first 300 files, the first 250 commits and the latest 200 things said, with the count left out kept in the reader and not yet shown anywhere. `agent_access` `Yes` |
| `write_pull_request` (`POST /pull_request_writes/:repository`) | `WritePullRequest`: `pull_request`, `kind` (`comment`, `approve` or `request_changes`), `body?`, `session_id?` | Answers `PullRequestPage`, read again after the write, and every Job's `pr` row follows the forge. A comment or a request for changes with no body is a 422 `fleet.pull_request_write_blank`; a `session_id` the ledger does not know is a 422 `fleet.session_unknown`; the forge's refusal is a 409 `fleet.pull_request_write_refused`. `Helm only`, and Helm asks first (`pushes to shared`) |

**Skew.** A Fleet before 23.83 answers 404 to both, so a Bridge that opens the panel draws its tabs empty and cannot write.

The page's `head`, `body` and `threads`, the write's `comments` and `commit`, and the two operations the Reviews tab adds are in *The Reviews tab* below.

## Protocol 23.81: write clashes between agents

`docs/concepts/fleet.md`, *Write clashes*. **Additive only**: one operation, one fact a terminal reports, and one new reach for `leave_note`.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_write_clashes` (`GET /clashes?manifest_id=&holder=`) | `WriteClashes`: `clashes[]` of `WriteClash`: `one`, `other`, `paths[]`, `since` | `one` and `other` are `ClashHolder`: `kind` (`job`, `session` or `main`), `id`, `label`. `other` is `main` for a holder against the base, with the base's name as `id`. `holder` is a Job's or a Session's id and keeps the clashes it is in. In memory in Fleet: a restart answers none until the holders write again. `Bridge only` |
| `SessionFact` `touched` (`POST /sessions/report`) | `paths[]`, relative to the job's `cwd` | Sent by the `armada` mod from 0.3.13 for each write tool call. Paths only; one that climbs out of the directory is dropped. Fleet reads it for a Job standing in a pool slot and ignores it elsewhere |
| `leave_note` (a Drone's tool) | `to` may be a Session's address, `s-` and the first eight characters of its id | Refused where Fleet has found no clash between that Session and the Job, and once per two minutes per Session |

**Skew.** A Fleet before 23.81 answers 404 to the read, so a Bridge that marks rows takes the 404 as no clashes and draws none, without a message. It also rejects a `touched` report as an unknown fact, and the mod drops it and goes on. A mod before 0.3.13 reports no writes, so its Job is never a holder in a clash. Bridge's half is `packages/protocol/src/clashes.ts` and `sessions.ts`, written by hand like the rest. Bridge reads `list_write_clashes` through one preload entry, `getWriteClashes`, which names no Manifest: main asks once per served repository and merges the answers (`apps/desktop/src/main/clashes.ts`).

## Protocol 23.80: a Job an agent started

`docs/concepts/session.md`, *A Job an agent started*. **Additive**: two operations on the agent's door, one thread row, two enum values and two attachment kinds. No existing field changes.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_another_session` (`POST /sessions/start_another`) | `StartAnotherSession`: `manifest_id`, `title`, `message`, `group?` (a plan group; see *A Job follows the Jobs its plan was handed to*). Answers `SessionHeld`: `session_id`, `title`, `fork` | `agent_access` Yes. The Job is created and listed at once with **no process and no turn**. 422 `fleet.agent_start_unplaced` where the connection places no hosted Job, 422 `fleet.agent_start_empty` for a blank `title` or `message` |
| `fork_this_session` (`POST /sessions/fork_this`) | `ForkThisSession`: `title`, `message`, `at?`. Answers `SessionHeld` with `fork` true | `agent_access` Yes. **Of the caller's own conversation only**: the caller is placed by its connection and no id is read. `at` is a thread row id as `fork.at` is; left out, Fleet takes the caller's latest message of its own that the transcript holds. 422 `fleet.fork_point_unwritten` as for `fork.at`; nothing is written |
| `SessionRow.started` (`session.row`, `get_session`) | `id`, `at`, `session` (`SessionLink`: `id`, `title`), `fork` | Written in the **caller's** thread. Bridge draws it as the press that opens `session` |
| `Attachment.kind` `unsent` | `target` is the first message's words, `standing` until the person's ordinary send, then `spent` | On the new Job. Bridge reads it as the text to put in the box once. Ending the Job gives it back. Kinds are open text, so no type changes |
| `Attachment.kind` `started_by`, `started_to` | `target` is the other Job's id, `spent` | A start's links. A fork keeps the `forked_from` and `forked_to` it already had |
| `WaitingSource` | `first_message` | Fleet's. Id `unsent:<session id>`, text `First message`, no options |
| `WaitingActKind` | `message` | `target` is the Job's id; a press opens it. An agent may also state it in `waiting_for` |
| `answer_waiting` | | Refused 422 `fleet.session_waiting_empty` for a `first_message` item: sending the first message is the answer |

**`send_session_message` is the one call that sends it.** What the box holds is sent, so an edit before the send changes nothing on the wire. It starts the first turn and settles the attachment and the item together.

**Skew.** A Fleet before 23.80 has neither route, so the tools are not offered to an agent, and sends no `started` row, `first_message` item or `unsent` attachment. A Bridge before it meets `first_message` and `message` as values it does not know. Bridge's half is `packages/protocol/src/sessions.ts` and `hosted-sessions.ts`, written by hand like the rest. **The event stream is unchanged**: one more `session.row` and `session.changed` per call, on the same sink.

## Protocol 23.84: a Job's running turn stopped

`docs/concepts/session.md`, *Stopping a turn*. **Additive only**: one operation and one type, `StopSession`, which carries a `session_id`.

| Where | Carries | Notes |
| --- | --- | --- |
| `stop_session` (`POST /sessions/stop`) | `StopSession`: `session_id` | Answers the `SessionRecord` with `hosted.turn` `idle`. The process is ended as after a quiet spell and the next message resumes it; a question it was held on is withdrawn; the slot is kept as it stands, nothing parked or committed. An ended job answers its row. 422 `fleet.session_command_refused` for a terminal job, whose process Fleet does not host |

**Skew.** A Fleet before 23.84 has no route and answers 404, so a Bridge that draws Stop shows the refusal. A Bridge before it never calls it. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest; the part that offers the act is `stopSession` on the preload.

## Protocol 23.92: a Job paused and resumed

`docs/concepts/session.md`, *Pausing a Job*. **Additive only**: two operations, three types and one optional field. There is no number to bump: the protocol ID moves with the files below.

| Where | Carries | Notes |
| --- | --- | --- |
| `pause_session` (`POST /sessions/pause`) | `PauseSession`: `session_id` | Answers the `SessionRecord` with `hosted.paused` set. Ends the turn, commits the slot's work to the branch (WIP, no push), frees the slot. `Helm only` |
| `resume_session` (`POST /sessions/resume`) | `ResumeSession`: `session_id` | Answers the record; `hosted.paused` is absent once a slot is held, and `resuming` while it waits for one. Starts no turn. `Helm only` |
| `HostedFacts.paused` | `SessionPaused`: `at`, `resuming` | Absent unless paused. Carried on `session.changed`, which is published on every change; no new event kind |
| 409 `fleet.session_not_pausable`, `fleet.session_already_paused`, `fleet.session_not_paused`, `fleet.session_pause_refused` | | A terminal Job is 422 `fleet.session_command_refused`, as for a stop |

**Skew.** A Fleet before 23.92 has no route and answers 404, and sends no `paused`. A Bridge before it ignores the field and never calls the routes. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest.

## Protocol 23.79: a Job forked at a message

`docs/concepts/session.md`, *A fork at a message*. **Additive only**: one optional field on `ForkFrom`, one 422 and one key in an existing detail. No new operation.

| Where | Carries | Notes |
| --- | --- | --- |
| `ForkFrom.at?` (`start_session` `fork`) | The id of a thread row | The copy ends at that message, included. Left out, the fork is the whole conversation and is made of an ended or dead job only, as since 23.69 |
| `start_session` with `fork.at` | | **The old job may be live** and is left as it was: no 409 `fleet.session_fork_live`. 422 `fleet.fork_point_unwritten` where no message of the thread has that id or the agent's conversation does not hold it; nothing is written |
| `Attachment.detail` of `forked_from` | `at`, `cut` | `at` is the row id as sent, which Bridge cuts the folded old rows at. `cut` is the transcript line the agent is told to end at and Bridge does not read it. Both absent for a whole-conversation fork |

**Skew.** A Fleet before 23.79 reads a `fork` with an `at` as it reads one without and refuses a live job 409, so a Bridge that sends `at` to it shows the refusal. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest; the part that offers the act is `forkSessionAt` on the preload.

## Protocol 23.73: Skill and Drone steps run

`docs/concepts/trigger.md`, *A Skill or a Drone step*. **Additive only**: one optional field on `JobTrigger`. A Skill Trigger, a Skill added step and a Drone added step now run on a side Drone, on a branch cut from the Job's, and no state, operation or error is new.

| Where | Carries | Notes |
| --- | --- | --- |
| `JobTrigger.drone?` | `true` where the Trigger names a skill | Left out for a Command. Lets Bridge name a Skill's branch apart from a repair's. An added step says the same through `AddedStep.runs.kind` |
| `JobTrigger.state`, `AddedStep.state` | `running` with `repair` / `repair_record` present | **A Drone is on it.** A Command that is running has no repair record. It ends `fix_ready` (the Drone committed), `passed` (it changed nothing) or `failed`, `held` where it blocks |
| `choose_trigger_fix`, `rerun_trigger`, `skip_trigger` | | Unchanged. A Skill's or Drone step's fix is placed as a repair's is, with no Command to run again, so `this_branch` ends `passed` once merged. Rerun on a held one puts the Drone on again and answers `running` |
| `TriggerSkipReason.skill_not_run`, `AddedSkipReason.skill_not_run`, `drone_step_not_run` | | **No longer produced.** The values stay, so a row an earlier build wrote still reads and removing them is not a major bump |

**`repair` is ignored for a Skill or a Drone step.** A Drone already fixes its own failures, so Fleet reads it as off: a Trigger file naming a skill loads with `repair` false, and `add_job_step` and the approval store `repair` false for a Skill or a Drone step. `block` stands.

**Skew.** A Fleet before 23.73 records a Skill `skipped` and sends no `drone`; Bridge draws it as it did. A Bridge before it sees a Skill run as a `running` Trigger that then ends `fix_ready`, which it draws as it draws a repair's fix. Bridge's half is `packages/protocol/src/triggers.ts`, written by hand like the rest.

## Protocol 23.72: repair on an added step

`docs/concepts/trigger.md`, *Steps added to one Job*. **Additive only**: one optional field on `AddedStep`, one on `ChooseTriggerFix`, one 422 and no new operation. A failed added Script with `repair` on is repaired as a Trigger is, and the repair branch is deleted once its fix is placed.

| Where | Carries | Notes |
| --- | --- | --- |
| `AddedStep.repair_record?` | `TriggerRepair`, as `JobTrigger.repair` | Present from the first repair Drone. Named apart from `AddedStep.repair`, which is whether the step asks for one. `AddedStep.block` already tells a hold from a plain repair |
| `AddedStep.state` | `repairing`, `rerunning`, `fix_ready` | The states 23.68 added to `JobTrigger.state`, now reached by an added step too. A step that blocks holds the Job through each |
| `choose_trigger_fix` | `ChooseTriggerFix`: `trigger?`, `addition?`, `choice` | **Exactly one of `trigger` and `addition`**, as `HoldAct`. `trigger` was required and is now optional, which only relaxes what Fleet reads: a Bridge before 23.72 still sends it and Fleet still takes it. 422 `fleet.no_fix_named` where both or neither is sent; 409 `fleet.no_fix_waiting` where the step holds no fix |
| `list_alerts`, `JobSummary.alert` | | An added step's fix waiting on a choice, and one that failed after its repair tries, are alerts as a Trigger's are. Nothing new is read |

**A repair branch is deleted** after `this_branch` has merged it and when the repair ends `failed` or `held`, for a Trigger as for an added step. It is a Fleet act with no wire: `branch` on `TriggerRepair` names a branch that may no longer exist once the state is `passed` with `this_branch`, `failed` or `held`. A `new_pr` branch is the pull request's head and stays.

**Skew.** A Fleet before 23.72 sends no `repair_record` and acts on no added step's `repair`, which Bridge reads as every row before. A Bridge before it never sends `addition`, so a fix it cannot see is one it cannot place. Bridge's half is `packages/protocol/src/added-steps.ts (deleted)` and `triggers.ts`, written by hand like the rest.

## Protocol 23.70: the merge queue on the hub

Additive only. The repository moved to the forge's merge queue, so `hub.pull_requests` entries say where the queue holds each one, rather than a second list beside them.

| Field | Shape | Notes |
|---|---|---|
| `HubPullRequest.queue` | `HubQueue`: `state`, `position?` | Absent from a Fleet before 23.70, and where the pull request is not in the queue and no auto-merge is waiting on its checks |
| `HubQueue.state` | `waiting_for_ci`, `in_queue` (a state Fleet does not know), `queued`, `awaiting_checks`, `mergeable`, `unmergeable` | **Strict**: Bridge picks a mark from it. `waiting_for_ci` is a pull request with auto-merge on whose own `ci` is still running, so it has no entry yet |
| `HubQueue.position` | 1-based, 1 is next to merge | Absent for `waiting_for_ci` |

**One more forge call a visit**, beside the open pull requests' listing. A forge that will not answer keeps the last reading. Bridge's half is `packages/protocol/src/merge-lines.ts`.

## Protocol 23.71: a Job's question answered

`docs/concepts/session.md`, *A job Fleet hosts*. **Additive only**: optional fields on two existing DTOs.

| Where | Carries | Notes |
| --- | --- | --- |
| `HelmCallInFlight.questions` (`hosted.asked`, `session.row` ask) | `AskedQuestion`: `question`, `header`, `multi_select`, `options` (`label`, `description`, and `preview` where the agent sent one) | Present where the call is the agent's `AskUserQuestion`; absent for every other call. `offers` is `allow_once`, `refuse` and nothing else |
| `answer_session_ask` (`POST /sessions/ask/answer`) | `answers?`: `QuestionAnswer` `question`, `chosen[]` | One entry per question. `allow_once` without every question answered is a 422 `fleet.session_answer_incomplete`. Free text is an entry in `chosen` |

Bridge's half is in `packages/protocol/src/helm-calls.ts` and `hosted-sessions.ts`, written by hand like the rest.

## Merge pressed while the forge's checks run

`docs/concepts/fleet.md`, *A person presses to merge*. **Additive only**: one operation, one optional field on Bridge's `Outcome`, and a store column.

| Where | Carries | Notes |
| --- | --- | --- |
| `enable_job_auto_merge` (`POST /jobs/:job_id/auto_merge`) | no body | Answers the `PullRequestState` the forge shows afterwards. Records the ask on the Job; the Job stays at `awaiting_review`. 409 off the gate, once the checks have failed (`fleet.merge_checks_not_passed`), and with the forge's words where it refused. `Helm only` |
| the sweep | nothing on the wire | A Job at its gate whose pull request is found merged, and whose ask is recorded, is taken as `approve_review` takes it, actor `fleet`. Any other merge is recorded and left |

`Outcome.pullRequest` is Bridge's own and not on the wire: the `PullRequestState` main carries back from `enable_job_auto_merge`, which the gate draws as Auto-merge on.

## Mods: a theme or a layout a job can make

`docs/concepts/mods.md`, and `docs/concepts/layout-mods.md` for the second kind. Five operations, one event and two optional fields on the preferences, and a layout mod adds one field to each of three of them.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_mods` (`GET /mods`) | `ModList`: `ModSummary` `name`, `kind?` (`theme` or `layout`), `version?`, `description?`, `enabled`, `valid`, `reason?`, `changed_at?` | A broken mod is a row with `valid` false; a field that could not be read is left out |
| `scaffold_mod` (`POST /mods/scaffold`) | `ScaffoldMod`: `name`, `description?`, `kind?` (`theme` or `layout`, absent is a theme); answers `ModScaffolded`: `name`, `path` | 422 `fleet.unacceptable_mod` for a name that is not a slug, or a mod that exists. 500 `fleet.mod_not_written` |
| `set_mod_enabled` (`POST /mods/enable`) | `SetModEnabled`: `name`, `enabled`; answers the `ModSummary` | 404 `fleet.no_such_mod`. `agent_access` is `Helm only` |
| `validate_mod` (`GET /mods/validate?name=`) | `ModChecked`: `name`, `valid`, `problems[]`, `css?`, `layout?` | `css` is a theme's checked text and `layout` a layout mod's, each present only when `valid` and never both |
| `promote_mod` (`POST /mods/promote`) | `PromoteMod`: `name`, `manifest_id?`; answers `ModPromoted`: `name`, `branch`, `commit` | 409 `fleet.mod_not_promotable`. `agent_access` is `No` |
| `mods.changed` (event) | `ModList`, whole | Names no Job. A resync does not carry it |
| `get_preferences` | `Preferences.theme?`, `Preferences.layout_choices?`, `Preferences.key_bindings?` | `theme` is left out while `dark`, and absent is `dark`. `layout_choices` is the text of the owner's `layout.json`, left out while he has made none. `key_bindings` is the owner's own keys, left out while he has changed none |
| `save_preferences` | `SavePreference.text?`, read for the names `theme`, `layout_choices` and `key_bindings` | 422 `fleet.unacceptable_theme`, 422 `fleet.unacceptable_layout` for text `layout.json`'s rules refuse, and 422 `fleet.unacceptable_key_bindings`. An empty `layout_choices` or `key_bindings` takes the choices back |

**The event stream gets no new queue.** `mods.changed` is one message per change, found by a rescan every two seconds, and goes through the shared drop-oldest backlog; a Bridge that missed it reads `list_mods` after the resync. It does not touch the unbounded-sink risk above.

## settings.json: every Machine setting, described and saved

`docs/concepts/machine.md`, *settings.json*. **Additive only**: two operations and one event; `get_limits`, `save_limits`, `get_preferences` and `save_preferences` keep their shape and read and write the same file.

| Where | Carries | Notes |
| --- | --- | --- |
| `get_settings` (`GET /settings`) | `SettingsList`: `path`, `refused?` (`key?`, `reason`), `settings[]` | Each `Setting`: `key`, `group`, `section`, `title`, `description`, `kind` (tagged on `kind`: `integer` `min` `max` `unit`, `seconds` `min` `max`, `boolean`, `choice` `options`, `text`, `prompt`, `text_list`, `json`), `default`, `value`, `saved` (`null` where absent), `applies` (`live` or `at_restart`), `pending_restart`, `overridden_by_env` (`null` where none) |
| `save_settings` (`POST /settings/save`) | `SaveSettings`: `changes`, a key to a value or `null`, which removes it | 422 `fleet.unacceptable_settings` naming the key, and nothing written; 500 `fleet.settings_not_written`. `agent_access` is `Helm only` |
| `settings.changed` (event) | `SettingsList`, whole | After a save, a hand edit, or a hand edit refused. A resync does not carry it |

Bridge's half is `packages/protocol/src/settings.ts`, written by hand like the rest.

## A call's arguments are no longer served

`#1774`. **A removal**: `get_call` (`GET /jobs/:job_id/calls/:call_id`) and `CallArguments` are gone, from `crates/ipc` and from `packages/protocol`. Bridge stopped reading it when the activity log's sheet went (`#1761`, `#1773`), so no Bridge on this side of the ID asks for it.

| Where | Carries | Notes |
| --- | --- | --- |
| `explain_command` (`GET /jobs/:job_id/calls/:call_id/explain`) | unchanged | Still under the call. Fleet reads a cut command whole from the transcript for it, and that read no longer crosses the wire |
| `Saw.called`, `Refusal`, `CommandInFlight` | unchanged | `truncated` and `length` still say how much was cut. Nothing serves the rest |

## Autopilot: a repository works its own backlog

`#2092`, `docs/concepts/autopilot.md`. **Additive only**: nine operations, one event kind, the DTOs of `ipc::autopilot`, and `origin` gains `autopilot_picked` (*Picked by Autopilot*), a string on the wire like the rest. Bridge's half is `packages/protocol/src/autopilot.ts`, written by hand.

| Operation | Carries | Notes |
| --- | --- | --- |
| `list_autopilots` (`GET /autopilots`) | nothing | Answers `AutopilotList`, every repository's, stopped ones included |
| `get_autopilot` (`GET /autopilots/:repository`) | nothing | Answers `Autopilot`: the settings, `state`, `capped_by`, the figures `spent_micros`, `jobs_started` and `at_work` read now, and `picks`. 409 `fleet.no_autopilot` |
| `propose_autopilot` (`POST /autopilots/:repository/proposal`) | `ProposeAutopilot`: `sentence`, `local_now?` | Answers `AutopilotSettings`. One model call; **starts and keeps nothing**. 500 `fleet.autopilot_not_read` |
| `start_autopilot` (`POST /autopilots/:repository`) | `AutopilotSettings`: `instruction`, `at_once`, `until?`, `spend_cap_micros?`, `job_cap?` | A 201 with `Autopilot`. 409 `fleet.autopilot_running`, 422 `fleet.autopilot_unsettable` |
| `pause_autopilot`, `resume_autopilot`, `stop_autopilot` (`POST /autopilots/:repository/pause`, `/resume`, `/stop`) | nothing | Answer `Autopilot`. 409 `fleet.autopilot_not_running`, `fleet.autopilot_not_paused`, `fleet.autopilot_stopped`. Stop closes each picked Job that never ran and marks its pick `withdrawn` |
| `dispatch_set_aside`, `dismiss_set_aside` (`POST /autopilots/:repository/picks/:pick_id/dispatch`, `/dismiss`) | nothing | Answer `Autopilot`. Dispatch starts a Job on the issue, as a sure pick does. 409 `fleet.pick_not_set_aside` |
| `autopilot.changed` (event) | `Autopilot` | On each act, each pick, a cap reached, and a pass that finds the figures moved. Whole, so a reconnecting Bridge replaces what it holds |

**A Job a sure pick starts is an ordinary `job.created` and `job.state_changed`**, its last move by `fleet` rather than `human`: no new kind says a Job skipped the gate, since the origin on its row does.

## A Job's Now: what runs, what is wrong, what it waits on

the retired Job concept, *A Job's Now*. **Additive only**: two fields on `JobDetail`, one on `JobDrone`, a `mode` on three answers and three operations. Every new field is optional, `serde(default, skip_serializing_if)`, so an older reader carries on and an older Fleet sends none. Bridge's half is `packages/protocol/src/job-issues.ts (deleted)` and the `drones`, `detail` and `asking` files beside it, written by hand.

| Where | Carries | Notes |
| --- | --- | --- |
| `JobDrone.activity` | `DroneActivity`: `line`, `tail[]` | The latest action and the few before it, off the transcript pass that yields `turns` and `cost_micros`. Absent on a Drone that has said nothing, never an empty line |
| `JobDetail.issues` | `JobIssue[]`, tagged by `kind` | `breakage` (`check`, `test`), `write_scope_overlap` (`job_id`), `failed_verdict` (`step_id`, `attempt`, `trigger?`), `stuck_drone` (`drone_id`, `step_id`, `quiet_since`). Each is a pointer to a fact `get_job` already holds; empty is none |
| `JobDetail.waiting` | `JobWaiting[]`, tagged by `kind` | `resource` (`name`), `job` (`job_id`, `title?`), `transition` (`step_id`). Empty on a Job that works and on one that waits on a person |
| `answer_question`, `answer_command`, `answer_judge` | `mode?`: `best` or `quick` | Fleet words the instruction to the Drone. A mode alone is an answer, so `chose` is then absent; with a label the label is the answer and the mode rides with it. A stale question id is still refused |
| `retry_check` (`POST /jobs/:job_id/retry_check`) | `check` | Runs that one Check again. 409 where no gate is running Checks, or the gate has no Check of that name |
| `skip_check` (`POST /jobs/:job_id/skip_check`) | `check` | Ends one Check as `skipped`, which advances the step and is not a pass. 409 as `retry_check`, and where that Check has finished |
| `skip_checks` (`POST /jobs/:job_id/skip_checks`) | nothing | Skips every Check not yet finished. 409 where no gate is running Checks |

**A `stuck_drone` is only ever a working Drone.** One at rest or waiting on an answer is quiet by design and yields no issue. The threshold is the one Fleet already pokes at (`quiet_after`), so a Drone is named stuck at the moment Fleet would first poke it, and not on a second clock.

**Redirecting a stuck Drone and retrying a step add no operation**: they are `redirect_one_drone` and `restart_step`, pressed from the issue row.

**Skew.** A Bridge before this draws a Job with no issues and nothing waited on, and a Drone row with no line beyond what its transcript gives. A Fleet before it sends none of the fields and refuses the three Check routes as unknown.

## Autopilot's follow-ups

`#2105`. **Additive only**: one operation, one answer type, three fields. Bridge's half is `packages/protocol/src/autopilot.ts`, `judged.ts` and `detail.ts`, written by hand.

| Where | Carries | Notes |
| --- | --- | --- |
| `propose_autopilot` | answers `AutopilotProposal` | The `AutopilotSettings` fields flattened in, and `quoted`: per setting, keyed as the settings are, the words of the sentence it was read from. A client that read the answer as `AutopilotSettings` still does |
| `update_autopilot` (`POST /autopilots/:repository/settings`) | `AutopilotSettings` | Answers `Autopilot` in the state it was in: it never resumes one. 409 `fleet.autopilot_stopped`, 422 `fleet.autopilot_unsettable` |
| `resume_autopilot` | unchanged | 409 `fleet.autopilot_capped` while any cap is reached, naming it. A reached cap now **pauses** with `capped_by` set, where it ended the run; Resume clears `capped_by` |
| `StepDetail.sent_back` | `JudgesSentBack`: `times`, `of` | How many times the Judges' agreed refusal sent this pass of the step back to its Drone, of `AGREED_RETRIES`. Absent where none did |
| `Judged.risky` | a string | What this member said makes the refused work risky, which is what brings an agreed refusal to a person. Absent where it said only that the work needs redoing, or was not asked |

A Drone can ask several decisions in one ask, and a person answers them all at once. **Additive**: the single question is unchanged, and `answer_question` keeps its route. Bridge's half is `packages/protocol/src/waiting.ts (deleted)`, written by hand. The ID moved with `ipc`, `operations/` and the mirror; no number was bumped.

| Where | Carries | Notes |
| --- | --- | --- |
| `ask_question` (Drone tool, not a route) | `decisions`, in place of `question` and `options` | Two to six, each `{question, options}` with a single question's option rules. Both forms at once, or one decision, is refused to the Drone by name. Any step may ask this way |
| `QuestionInFlight.decisions` (on `JobDetail.asking`, `job.asking`) | `DecisionInFlight`: `decision_id`, `question`, `options` | Absent is a single question. Where present, `question` is blank and `options` empty. Fleet mints `decision_id` as `d1`, `d2`... |
| `answer_question` (`POST /jobs/:job_id/answer_question`) | `ChosenAnswer.decisions`: `ChosenDecision`, `decision_id` and `chose` | One pick per decision, all in one call, with no `chose`. `ChosenAnswer.chose` is now optional: absent on an interview. 409 `fleet.illegal_move`, the reason in `message`, for a missing, unknown or doubled decision, a label the decision did not offer, a label sent for an interview, or picks sent for a single question. Nothing is kept on a refusal |

## Routing a thought: Helm reads words for where they belong

**Since 13 Oct 2026 the `sent` row is a `SessionRow::Sent` in the Helm Job's thread, and `record_thought_sent` answers a `SessionRecord`.** The `observe_helm` message `sent` and `HelmSent` rows below are retired; see *Helm is a Job*.

`docs/concepts/helm.md`, *Routing a thought*. **Additive only**: two operations, one event kind and one message on the Helm socket, all of Bridge's (`agent_access` `No`, `Bridge only` for the event). No type that exists changes. Bridge's half is `packages/protocol/src/thought.ts`, `helm.ts` and `helm-thread.ts`, written by hand. There is no number to take: the protocol ID moves with the wire files, and `armada need` has nothing to order.

| Where | Carries | Notes |
| --- | --- | --- |
| `read_thought` (`POST /helm/read_thought`) | `ReadThought`: `text`. Answers `ThoughtReading`, tagged by `kind`: `question`, or `thought` with `target?` | One model call on the proposer's model, over every live Session and running Job in every repository. Always 200: a call that fails, runs past 20 seconds or answers what was not asked is `question`. An id the model names that was not offered is `thought` with no `target`. A message opening on a wh-word and ending in `?` is `question` with no call |
| `ThoughtTarget` | `kind` (`session` or `job`), `id`, `title`, `manifest_id?`, `slot?` | `slot` is the slot a Job holds. `manifest_id` is absent for a Job in no repository Fleet serves. A plain enum rather than a `wire_enum!`: no registry declares the set, and a third kind would be a new act Bridge has to learn |
| `record_thought_sent` (`POST /helm/thought_sent?manifest_id=`) | `ThoughtSent`: `text`, `target`. Answers 200 `HelmConversation` | Said after the delivery went through, so a refused delivery records nothing. Appends the row below and publishes `helm.thought_routed`. The manifest names the conversation, absent is the first served |
| `observe_helm` message `sent` | `HelmSent`: `ts`, `text`, `target` | A row of the thread's file, so it is in the backfill after a restart. Never delivered to Helm's job |
| `helm.thought_routed` (event) | `HelmThoughtRouted`: `manifest_id`, `to` (`session` or `job`), `id`, `at` | The audit event of *Audit trail*. Names where and never the words. Rust only: Bridge mirrors no `helm.*` audit kind |

**This reads across repositories, which `helm.md` otherwise says Helm does not do.** The owner decided it on 12 Oct 2026 for this one read. `read_thought` takes no `manifest_id`, so what it can see does not depend on where Helm points.

**The event stream.** One small event per delivery, a person's press apart, on the shared drop-oldest channel. It adds no queue and assumes no delivery is complete, so it leaves the unmeasured sink risk where it was. The `sent` message rides the conversation's own bounded channel, which is drop-oldest with a `missed` count, and the thread's file keeps what a slow viewer lost.

**`tell_session` is Helm's own delivery, and the one operation here Bridge never calls.** `POST /helm/tell_session?manifest_id=`, body `TellSession`: `session_id`, `text`. `agent_access` is `Helm only`, so the door offers it to a Helm job alone and a Drone's endpoint never lists it. It answers 200 with the `ThoughtTarget` the words went to. Fleet sends through `send_session_message`'s path, so a refusal is that send's, and only a send that was taken appends the `sent` row and publishes `helm.thought_routed`. The door names the Manifest the job stands in as `manifest_id` (`NAMES_ITS_SCOPE`), which picks the thread. No type that exists changes, and `packages/protocol` has no mirror of `TellSession` because nothing in Bridge sends it.

**Skew.** A refusal on an ID mismatch means no peer meets this change half built. A Fleet without the routes would answer 404 to a Bridge that has them, and Bridge treats a read that fails as `question`, so the words still reach Helm.

## Helm's message box: files, tags, model and effort, the `/` list

**Retired 13 Oct 2026 but for `AskHelm`'s `attachments` and `mentions`, `get_issue` and `TagKind::Issue`.** `tune_helm`, `HelmBoxFacts`, the socket's `facts` message and `get_helm_file` are gone: the box is a Job's, tuned with `tune_session` and fed by `session.hosted`. See *Helm is a Job*.

`docs/concepts/helm.md`, *The message box*, and `docs/concepts/session.md`, *What a message names*. **Additive only**: three operations, all `agent_access` `No` and Bridge's, one `TagKind`, and optional fields on existing DTOs. Two required fields are new, `HelmOpened.facts` and the `facts` message, which a Bridge from before them cannot read; the ID refuses that pairing, so nothing meets it half built. Bridge's half is `packages/protocol/src/helm.ts`, `helm-thread.ts`, `hosted-sessions.ts` and `issues.ts`, written by hand.

| Where | Carries | Notes |
| --- | --- | --- |
| `AskHelm` (`ask_helm`) | `attachments[]` of `SessionUpload`, `mentions[]` of `SessionTag`, beside `text` and `context` | `text` is a plain string, no longer a type that refuses blank (`AskCommander` keeps `HelmText`), and may be blank when something is carried; blank words with nothing carried are 422 `fleet.helm_message_empty`. Uploads are kept as a Job's are, in a directory of the conversation's own, and the refusals are the same |
| `ThoughtSent` (`record_thought_sent`), `HelmSent` (`sent` message) | `ThoughtSent` gains `attachments[]` and `mentions[]`; `HelmSent` gains `files[]` of `SentFile` and `tags[]` of `SessionTag` | A thought routed to a Job keeps what went with it on its `sent` row, so the chips survive a restart. Fleet keeps the uploads in Helm's own directory as an ask's, and `get_helm_file` serves a `sent` row's files too. `tell_session` records words only, so both lists are empty |
| `HelmAsked` (`asked` message) | `files[]` of `SentFile`, `tags[]` of `SessionTag` | A row of the thread's file, so a reloaded Bridge draws what each message carried |
| `get_helm_file` (`GET /helm/file?manifest_id=&file=`) | The bytes of a `SentFile.id` from an `asked` row, under the media type they were sent as | `get_session_file`'s shape and refusals |
| `tune_helm` (`POST /helm/tune?manifest_id=`) | `TuneHelm`: `model?`, `effort?`. Answers `HelmBoxFacts` | Absent or blank is the machine's own. 422 `fleet.unacceptable_effort` for anything but `low`, `medium` or `high`. Kept per conversation in the store, passed at the launch of every message, kept through Start fresh |
| `HelmBoxFacts` | `model?`, `effort?`, `commands[]` | `commands` are the agent's own `init` names, else the last any Job read |
| `HelmOpened.facts` | `HelmBoxFacts`, always present | What a viewer starts from |
| `observe_helm` message `facts` | `HelmBoxFacts` | **Live only**: never in the backfill and never written to the thread's file. Sent when the tuning or Helm's own commands change; a change in the Jobs' list is read at the next `opened` |
| `TagKind::Issue`, `SessionTag.url?` | `issue`; the forge address of an issue or pull request tag | Fleet writes `issue #214 "title" (address)` for the agent, and a leading `#N ` in a title is dropped from the line |
| `get_issue` (`GET /issues/:repository/:reference`) | Answers `IssueRead`: `reference`, `title`, `url`, `state` (`open` or `closed`) | 404 `fleet.no_such_issue`, and a pull request's number is not an issue; 500 `fleet.issues_unreadable`; 422 for a repository Fleet does not serve |

**Start fresh deletes the conversation's uploads**, with the thread that named them. The turn a words-only ask writes is unchanged; an ask with a picture is written as a block list, as a Job's is.

**The event stream.** `facts` rides the conversation's own bounded, drop-oldest channel with its `missed` count, and costs a viewer nothing it cannot recover: the next `opened` carries the current facts. It adds no queue to `/events`, so it leaves the unmeasured sink risk where it was. Files are not on a stream at all.

**Skew.** None to meet: a refused ID mismatch means no peer sees this half built. A file's bytes are read once by a picture's thumbnail and held by the window.

## The plan's windows, and what the limit holds

Since 12 Oct 2026. **Additive only**: one operation, one event kind, one fact, five optional fields. Bridge's half is `packages/protocol/src/plan-usage.ts`, `sessions.ts`, `protocol.ts` and `events.ts`, written by hand. Nothing here is the lifeboat's.

| Where | Carries | Notes |
| --- | --- | --- |
| `get_plan_usage` (`GET /plan_usage`) | `PlanUsage`: `windows` (`PlanWindow`: `kind`, `percent_used`, `resets_at?`) and `read_at?` | The newest reading, from whichever job reported last. Empty `windows` is no reading. For display: nothing is held back for a percentage |
| `plan_usage.changed` (event) | `PlanUsage`, whole | Published when a report moves a window a whole percent or changes the windows. A repeat publishes nothing. Served and published, so a Bridge opened later asks |
| `SessionFact::Measured.plan?` (mod to Fleet) | `PlanWindow[]` | Left out off a subscription. An empty report never clears the reading held |
| `SessionFact::Limited` (mod to Fleet) | `resets_at` | The turn ended on the plan's limit |
| `SessionFact::Measured.context_breakdown?` (mod to Fleet) | `ContextBreakdown` | What fills the context window. Names, paths and counts; the ten heaviest MCP tools. Fleet keeps the latest and an absent one leaves it |
| `SessionRecord.context_breakdown?` | `ContextBreakdown`: `categories` (`name`, `tokens`, `kind` of `used`, `free`, `buffer` or `deferred`), `memory_files` (`path`, `tokens`), `mcp_tools` (`name`, `server`, `tokens`), `total_tokens`, `max_tokens`, `autocompact_threshold?` | Mirrored in `packages/protocol/src/context-breakdown.ts`. `kind` is a plain enum Bridge matches on, so a fifth value is a change to tell every reader |
| `SessionRecord.held_until?` | an instant | The job is held for the limit and is woken a minute after it |
| `JobSummary.paused.until?` | an instant | The Job's Drone was stopped by the limit; admission starts it again a minute after |

**The event stream.** `plan_usage.changed` is one small message per moved whole percent, through the shared drop-oldest backlog, and a mod reports at most every ten seconds a session. It adds no queue and assumes no delivery is complete: a Bridge that missed one reads `get_plan_usage` on the resync that follows a `Missed`. It leaves the unmeasured sink risk where it was. Held sessions and Jobs add no event of their own; they ride `session.changed` and `job.paused`.

## The Job model steps down

**Additive only**: five operations, one event kind, two types. Bridge's half is `packages/protocol/src/model-downshift.ts` and `events.ts`, written by hand.

| Where | Carries | Notes |
| --- | --- | --- |
| `get_model_downshift` (`GET /model_downshift`) | `ModelDownshiftState`: `downshift?`, tagged by `phase` | `offered` (`percent`, `was`, `to`), `shifted` (`was`, `to`, `until`) or `reset` (`was`, `to`). Absent is none |
| `model_downshift.changed` (event) | `ModelDownshiftState`, whole | Published when the state moves, the offer's percent included. Served and published, so a Bridge opened later asks |
| `accept_downshift` (`POST /model_downshift/accept`) | no body | Valid while `offered`. Writes `models.session` through `save_settings`' path; a refused write keeps the state |
| `restore_downshift` (`POST /model_downshift/restore`) | no body | Valid while `shifted` or `reset`. Puts the saved word back, or removes the key |
| `keep_downshift` (`POST /model_downshift/keep`) | no body | Valid while `reset`. Leaves the lower model as the default |
| `settle_downshift` (`POST /model_downshift/settle`) | no body | Valid while `offered`. Makes no second offer in this window |

**A command in the wrong phase is a 409** (`fleet.downshift_wrong_phase`), never a silent no-op, and each command answers the state it left. All four are `agent_access` `No`: a step down answers an offer Bridge draws, and Helm moves `models.session` with `save_settings`. Settings: `plan.downshiftAt` and `plan.downshiftTo` in a new `Plan` section, and `models.default` and `models.session` are `live`. The step down moves `models.session` and never `models.default`, which Drones and Jobs keep.

**The event stream.** `model_downshift.changed` is rare: an offer, an accept, a reset, a restore, and the percent moving a whole point while an offer stands. It adds no queue and assumes no delivery is complete: a Bridge that missed one reads `get_model_downshift` on the resync that follows a `Missed`.

**Bridge's side.** Main passes the event to every window and to `downshift-notice.ts`, which tells the OS when the state enters `offered` or `reset`, once for each entry; the state read on a connection is remembered and told to nobody. A window's source (`fleet-downshift.ts`) reads once Fleet is connected, and a 409 from an act makes it read again rather than show an error.

## A Job's own default model

**One required field and one setting.** `ModelChoices.session` is `models.session` as it stands when `list_models` is asked, always a member of `models`, and `default` stays `models.default`. It is required, not additive: Bridge and Fleet ship together, and a Fleet that omits it fails Bridge's read rather than drawing a picker with no Job default. Bridge's half is `packages/protocol/src/setup.ts`, written by hand.

| Setting | Ships as | Read by |
| --- | --- | --- |
| `models.session` | The roster's strongest model, derived in `adapters` | A hosted Job whose process starts with no model chosen, and the step down |
| `models.default` | The Drone's model | A Job proposed with no model, and nothing that starts a Session |

**No environment variable** names `models.session`; `ARMADA_MODEL` still names `models.default`. The setting is `live` and a Job already running keeps its process until the next message starts another.

## The Fleet's timeline: kept as it happens, read as a window

`get_fleet_timeline` (`GET /timeline?since=&until=&limit=`) and `timeline.appended` serve the Cockpit's
timeline: what happened to Jobs, Sessions, pull requests, slots and main, each with its instant. Before
them nothing answered it. `get_events_since` counts kinds, `get_job_events` is one Job's status moves,
and a Session or a merge line moves as a row replaced whole, with no instant to cut a window by.
Since 23.82.

| Source | How it is entered | Why not an event of its own |
|---|---|---|
| A Job's moves, a Session row | A tap on the stream (`Broadcaster::tap`) turns each published event into an entry | They are events already; they only lacked a record |
| Pull requests, main's colour | `merge_lines::keep_reading` compares each read of the hub with the one before | `merge_lines.changed` carries the list whole, so what happened is the difference |
| A person's answer | The three Job answers and the Session ones enter it after they succeed | The answer is a command, and the row it clears is replaced |
| A slot leased or given back | The ledger calls that take and give back a Job's or Session's slot, when the row changed | The pool is read whole and never published |

**A tap and not a subscription.** A subscription drops its oldest when it falls behind, and a timeline
that silently lacks a Job's failure is worse than none. The tap runs inside `publish`, so it only
queues; one writer takes the store lock and numbers entries in the order queued. `timeline.appended` is
published after the write, so its `id` is the number the store gave it.

**Entries are dated by whoever saw the thing**: a Job's event carries its instant, a Session's row states
its start and each item's `since`, the forge dates a merge, and the rest are stamped by the read that saw
them. The window is cut by `at` and never by `id`, which only says what a client holds already.

**Kept thirty days**, pruned on the next write, as `check_runs` is. The row is the fact as it read then,
so a forgotten Job keeps its history.

**A Session's turn is entered once, where it ends**, and the fact is its length (`session.turn`, "Turn took
2m 14s"), from the start Fleet heard to the end it heard. Never on the rows between, and not on the end of a
turn held for a slot, which resumes in it as one turn. A turn reattached after a restart has no start heard,
so it enters nothing.

**Reads, edits and commands are entered one to a burst**, because an agent makes hundreds of tool calls an hour and a row for each would bury the turns. `session.read`, `session.edited`, `session.ran`, `job.read` and `job.ran` count what one Session or Job did of one act: `Read 14 files`, `Edited 3 files`, `Ran 5 commands`, with the latest path or command as `detail`. A burst ends when its subject has done none of it for fifteen seconds, when the Session's turn or life ends, or two minutes after it began, so a long run still shows as it goes (`QUIET` and `MOST` in `timeline/tools.rs`). **The entry is written when the burst closes and dated by its first call**, so a client orders a window by `at` and not by `id`. Only a path is kept of a file, and a command is cut to the line and has the value of a `NAME=`, a `--flag` or a query pair whose name says secret, what follows `Authorization:` or `Bearer`, a password in an address and any long token-shaped word hidden: it errs toward hiding.

**`items` is what the burst named**, so a row can open to its files or commands: newest first, a path or command appearing once however often it was named, **at most 25** (`MOST_TIMELINE_ITEMS`), each cut and hidden exactly as `detail` is, and the first is `detail`. 25 bounds an entry at about 3 KB and a list at about a screen; the count in `says` is still the whole burst's. The store keeps them in `timeline_events.items`, one to a line (a line is all an item can be), and an entry written before the column has none: Bridge then has nothing to open. Absent from every other kind. Additive.

| Source | Where it is heard |
|---|---|
| A terminal Session | The mod's `touched`, `read` and `ran` facts (`sessioning.rs`), counted when the Session's origin is a terminal |
| A Session Bridge hosts | Its own process's calls (`session_host::hearing`), through the harness's `edited` and `used`, so the mod's facts for it are not counted twice |
| A Job's Drone | A tap on its stream (`JobToolsSeen`). A Job's edits stay `job.files` |

**A call outside the repository is `session.called`, one entry for each call** (17 Oct 2026). A Session's `gh` command, a page it fetches, a web search, and a tool of any MCP server that leaves the machine. The entry's `says` is `Called {name}` (`Called GitHub`, `Called the issue tracker`, `Called the web`, else the server's name), `detail` is the one line that says what was asked, and `call` is `{ service, name, line }`: the service, what it is called (the `Reach.name` the adapter already has, the same words `says` uses) and the same line. Bridge labels a service's outpost with the newest call's `name`, the leading `the` dropped and the first letter capitalised, so it holds no service's name of its own. **The line names the act and never what it was given**: a `gh` command is `gh` and its subcommand words up to two (`gh pr view`, never a number, flag or body); a fetch is the page's host and no scheme, user, port, path or query; a search is `search`, never the query; an MCP call is the tool's name. `gh issue …` is the `tracker` service, every other `gh` command and the forge's own server the `forge` service, and so is a Linear, Jira, Atlassian, Asana, ClickUp or Monday server; any other server is its own lowercase name with a connector or plugin prefix dropped (`claude_ai_Figma` is `figma`). Armada's own servers and the code-graph helper never leave the machine and enter nothing. A `gh` command is this and not a `session.ran`, so it is in no burst. **`service` is an open string**: Bridge draws a plug for one it has no word for, so a new server moves nothing in Bridge. The set is read in `crates/adapters/src/outside.rs`, because the tool names are the harness's words and the commands the forge's; Fleet only carries the `Reach` it is handed (`ToolUse::Reached`). The store keeps the service in `timeline_events.service`, its name in `service_name` and the line in `detail`; a row written before the column has none. A terminal Session's `gh` command is told by the mod's `ran` fact like any command. **Its MCP and web calls are not entered yet**: the mod does not tell them, and the `used` fact it does tell also fires for a tool merely loaded, so it cannot stand in for a call. Additive: a new kind, one optional field left out when absent, two nullable columns. **It adds entries to the event stream**, one per call rather than one per burst, so a busy Session writes more than it did; that makes the unbounded sink's risk a little worse and not better, and the drop-oldest bound with its resync message stays owed.

**The agent's own list is `session.waiting` and `session.resolved`**: an item put on the `waiting_for` list is a wait, dated by its `since`, and one gone from it while the Session lives is resolved, dated by the row that dropped it. What Fleet raised (a question or permission card, a question in a walk window, a reply to read) stays `session.asked`, and a person's answer stays `session.answered`. A Session ending clears its list, which resolves nothing. `session.subagent` is a subagent attached to the row (`Subagent started`), standing again after it settled (`Subagent resumed`, dated by its new `since`) or spent (`Subagent finished`), with `Type: description` as the detail; a Session already holding one when Fleet first saw it enters nothing for it. `session.spawned` is entered under the Session whose agent started another, or forked itself (`start_another_session`, `fork_this_session`), with the new Session's title as the detail.

**A Session's walk, commit and push are entered as `session.walked`, `session.committed` and `session.pushed`.** The walk is written when the page is shown (`show_window`), with its title as the detail, and apart from the `walk` item on the Session's list, which is not entered. A commit and a push come from the mod's `committed` and `pushed` facts, which it tells only where git printed a commit made or a ref updated; the detail is the branch. They are the road a Session's `build` reads its stops from (`docs/concepts/session.md`, *The build record*), so the Session's row is published again once one is written. The store holds an index on `timeline_events (subject_kind, subject_id, kind)` for that read.

**A pull request is entered whether or not anyone is shown it**, but Bridge draws it only on the track of the Job or Session that owns it, and leaves out one it cannot attribute (`fromWire` in `cockpit/fleet-events.tsx`).

**Not entered, deliberately.** A Check's start (its result is entered), a slot an agent leases with `armada worktree lease` (Fleet never hears of it; *A bay's history* below reads it from the pool), and
anything before this shipped. A Session already live when Fleet starts is entered as started the first time
its row changes.

**The event wraps the entry under `event`.** The stream tags its events with `kind`, and an entry has a
`kind` of its own; spread into one message they produced a duplicate key, which a test caught.

## When a Job's Drone was last told

`JobSummary.last_told_at`, optional and additive: when a person or Helm last sent the Job's Drone
words (`redirect_drone`, or the answer to a command it was refused) or restarted a step (`restart_step`).
Bridge reads it to decide that a failing pull request or red main the Job owns has somebody on it again, so
the Cockpit raises no call for it.

**Held in memory and lost on a Fleet restart**, as `releasing::Spared` is: a redirect into a healthy Drone
writes nothing to the log, so there was no record to read it from, and a surface treats the field as "told
since X", never as a history. A restart Fleet makes of its own after a boot does not set it. It rides the
rows Fleet already serves and adds no event, so it reaches a Board with the next summary of the Job.

## A stranded slot's last holder, and whether Fleet told it

Since 13 Oct 2026. `docs/concepts/fleet.md`, *Rescuing a stranded slot*. **Additive only**: two optional fields on `SlotStranded`, one new type and one new plain enum. Bridge's half is `packages/protocol/src/holding.ts`, written by hand. There is no number to take: the protocol ID moves with `crates/ipc/src/holding.rs`.

| Where | Carries | Absent |
| --- | --- | --- |
| `SlotStranded.last_holder` | `SlotLastHolder`: `kind` (`session` or `job`), `id`, `title?`, `open` | A holder the lease cannot name: a process outside any Job Fleet hosts, or a record written before Fleet noted it |
| `SlotStranded.told` | An instant: when Fleet told `last_holder` to commit, push and release | Not told: the holder is not an open Job, or Fleet has not yet read the slot since it was stranded |
| `SlotHolderKind` | `session`, `job` | |

**`open` is what decides.** A Session is open while the ledger holds it live, a Job until it ends. Fleet tells only an open Session, and a slot whose last holder is not open, or is absent, is read by a Scout without a press, so `rescue` appears on a slot nobody pressed.

**`SlotHolderKind` is a plain enum Bridge matches on**, as `CommitHome` is, because no registry declares the set; a third kind is an act Bridge has to learn.

**`stranded` is where these ride**, so they are present on a slot a Job's kept release holds as well as a stranded one, and absent on the slot a live process holds with only uncommitted files. Nothing about `rescue_slot` changes: `start` is still accepted from a person, and refused with `fleet.rescue_reading` while Fleet's own Scout reads.

**Skew.** A Bridge that predates this reads no `last_holder` or `told` and draws a stranded bay as before. A Fleet that predates it sends neither, and Bridge treats both as absent.

## An agent lists Jobs in a line each

`list_sessions` through the agent's door answered 696 KB: every Job ever, each with its whole ledger, and the pull request bodies in it. The door cuts at 64 KiB, and a Helm job that read the cut file saw two of nine live Jobs and said only two were live. **Additive only**: one query parameter, three types and one field on the door's answer. No operation is added, so nothing is tagged in `commander_reach.rs`. There is no number to take: the protocol ID moves with the wire files. Bridge's half is unchanged, because Bridge and Pocket send no `view` and are answered the row they were.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_sessions` (`GET /sessions?manifest_id=&q=&state=&view=`) | `view` is `full` (absent) or `summary` | `full` is `SessionList`, as before. `summary` is `SessionSummaries` |
| `SessionSummary` | `id`, `title?` (160 characters at most), `manifest_id?`, `origin`, `state`, `turn?` (`idle` or `working`), `last_turn_at?`, `last_seen_at`, `slot?`, `branch?`, `pull_requests` (`number`, `title`, `state?`), `jobs` | What is **standing** only: a merged pull request or a slot given back is history, and `get_session` has it. `turn` is absent on a terminal Job, which reports where a turn ended and not where one began. `jobs` are the Jobs it holds that have not finished, by id |
| The agent's door, `list_sessions` | Asks for `view=summary`, and for `state=live` where the call names no `state` | `view` is not among the tool's parameters, so an agent cannot ask for the wide row. `state=ended` is its way to a finished Job |
| `door::Answer.narrowed_by` | The tool's query parameters | Said in a cut answer: *To be answered less, call this tool again with `q` or `state` set*, after the route that serves it whole. A tool with no query parameter says *Ask a narrower question if you can* |

**A narrow row beside the wide one, and not a cut of it.** The summary has no field that grows with a Job's history, so how many Jobs fit is a count of lines and not a hope. `crates/api/src/tests/door_sessions.rs` holds the bound against a Fleet whose Jobs each have a long ledger of pull requests with their bodies. A truncation would still have been a guess about which half to keep. The default belongs to the door and not the route, because a route that answered a summary unasked would change Bridge's list.

**Who reads the route.** Bridge's main process (`apps/desktop/src/main/sessions.ts`) and Pocket send no `view` and are unchanged. Fleet's own readers (`read_thought`, the routing of a thought) call the daemon's `list_sessions` directly and never this route.

## What a Job is doing, from its tool calls

Since 14 Oct 2026. `docs/concepts/session.md`, *Working on*. **Additive only**: one optional field on `SessionRecord` and three types. No operation, event kind or error is new. Bridge's half is `packages/protocol/src/session-activity.ts` and `sessions.ts`, written by hand. There is no number to take: the protocol ID moves with `crates/ipc/src/session_activity.rs`.

| Where | Carries | Absent |
| --- | --- | --- |
| `SessionRecord.activity` (`session.changed`, `get_session`, `list_sessions`) | `SessionActivity`: `now?`, `todos[]` | Before a hosted Job's first tool call, on a Job a terminal runs, and once it ends |
| `SessionActivity.now` | One short fact for the latest call Fleet can name: `Editing gate.rs`, `Reading gate.rs`, `Running cargo check`, `Searching`, `Fetching`, `Running a subagent` | A call Fleet cannot name leaves it as it was; the field is left out until one is |
| `SessionActivity.todos` | `SessionTodo`: `text`, `state` (`done`, `doing` or `todo`) | Left out while empty |

**`state` is a plain enum Bridge matches on**, as `WaitingMode` is, because no registry declares the set. **The gate that detects it is `gate_session_call`**, and its route is unchanged: Fleet notes the call before it decides it.

**Publishing is coalesced.** A change to `now` or `todos` publishes `session.changed` at once when none went out for the Job's activity within two seconds, and otherwise once, when the two seconds are up. A call that changes neither publishes nothing. A turn starting clears `now` on the row that announces it and keeps `todos`. No call time is carried: a silent turn is the `stuck` state. A Fleet restart starts the field empty.

**Skew.** A Bridge that predates this ignores an `activity` it does not read. A Fleet that predates it sends none, and Bridge treats that as no activity. The same change moves `HostedSessions::gate_session_call` to `self: Arc<Self>` in `crates/api`, which is Rust only and not on the wire.

## A Job's build record

Since 14 Oct 2026. `docs/concepts/session.md`, *The build record*, and `.claude/decisions/2026-10-14-a-session-builds-a-ring.md`. **Additive only**: one optional field on `SessionRecord`, new fact kinds, new timeline kinds and the types they carry. No operation, event kind or error is new. Bridge's half is `packages/protocol/src/session-build.ts`, `sessions.ts` and `timeline.ts`, written by hand. There is no number to take: the protocol ID moves with the wire files, and `armada need` has nothing to order. The migration `20261014T2100Z-session_build.record` has a name and not a number, and is additive.

| Where | Side | Change |
| --- | --- | --- |
| `SessionRecord.build` (`session.changed`, `get_session`, `list_sessions`) | Fleet writes, Bridge draws | `SessionBuild`: `phase`, `pieces[]`, `stops[]`, `forecast[]`, `landed_at?`, `quiet_since?`, `stalled`, `workers[]`. Left out for a Job that has done nothing Fleet observed |
| `SessionBuild.phase` | Fleet writes, Bridge matches | `scope`, `plan`, `implement`, `road`, `landed` |
| `SessionBuild.pieces[]` | Fleet writes, Bridge matches `piece` | `BuildPiece`: `id`, `text`, `piece` (`solid`, `hollow`, `scaffold` or `ghost`), `working?` and `by?`, the writing subagent's label in the form `workers` uses (`Type: what it was asked`), left out for the Job's own tasks |
| `SessionBuild.stops[]`, `forecast[]` | Fleet writes, Bridge matches `id` | `BuildStop`: `id` (`walk`, `pushed`, `pr`, `checks` or `auto_merge`) and `at?`, left out for a stop not yet reached |
| `SessionFact` `tasks`, `plan_left`, `committed`, `pushed` | The mod writes | `tasks` carries `ReportedTask`: `id`, `text`, `status` (`pending`, `in_progress` or `completed`) and `by?`, the `agentId` of the subagent that wrote the task, absent for the Job's own (mod 0.3.24). `committed` and `pushed` carry `branch?` |
| `TimelineKind` `session.walked`, `session.committed`, `session.pushed` | Fleet writes, Bridge draws | Strict. A walk's detail is the page's title, the others' the branch |

**Every closed set here is a plain enum Bridge matches on**, as `SessionStanding` is, because no registry declares them. **The arrays are always sent**, empty when they hold nothing, so a reader never tests for them. An unreached stop has no `at`, and none is sent as `null`.

**Subagents' tasks, 15 Oct 2026. Additive:** `ReportedTask.by?` and `BuildPiece.by?`, both optional and left out when absent, and the migration `20261015T2000Z-session_build.task_worker` adding a nullable `made_by` to `session_build_tasks`. A row kept before it reads as the Job's own tasks. A Bridge that predates `by` ignores it and draws every piece as the Job's. A mod before `0.3.24` tells no subagent's task, so the ring lacks them.

**The agent's tasks on Progress, 17 Oct 2026. Additive:** `BuildPiece.working?`, true where the agent's task tool has the task in progress and left out otherwise. Fleet already kept the status (`session_build_tasks.status`), so there is no migration and no new event: the row is published when the list changes, as before. Bridge's `SessionWork.tasks` is these pieces for a Job with no recorded plan, in the order made: `solid` and `hollow` are done, `working` is working, the rest are open. A Bridge that predates `working` lists every unfinished task as open.

**Skew.** A Bridge that predates this ignores a `build` it does not read. A Fleet that predates it sends none. A mod before `0.3.23` tells none of the new facts, so its Job's `build` holds only what Fleet heard otherwise, and the Job is marked `mod_out_of_date`. A Fleet that predates the facts refuses them, and the mod goes on.

## A Job's plan and its file touches

Since 16 Oct 2026. `docs/concepts/session.md`, *The plan*. **Additive.** No protocol number moves; the protocol ID changes with the wire it hashes. Types live in `crates/ipc/src/session_plan.rs` and `packages/protocol/src/session-plan.ts`, apart from `SessionRecord`'s file so a parallel change to the record merges cleanly.

| Where | Side | Change |
| --- | --- | --- |
| `record_plan` (`POST /sessions/plan`, on the agent's door) | The agent calls | `RecordPlan`: `groups[]` (`id?`, `title`, `agent`, `paths[]`, `tasks[]` of `id?` and `title`), `sketch?` (`title`, `scene`), `session_id?`. Answers the receipt `{version, groups[]}` with each id Fleet gave. Refused 422 `fleet.plan_refused` with every reason, `fleet.plan_unnamed` where no Job is placed |
| `approve_plan` (`POST /sessions/plan/approve`) | Bridge calls | `ApprovePlan`: `session_id`, `version`. Answers the `SessionRecord`. 409 `fleet.plan_version_stale`, `fleet.plan_none` |
| `send_plan_changes` (`POST /sessions/plan/changes`) | Bridge calls | `SendPlanChanges`: `session_id`, `version`, `marks[]`, `approve?`. `PlanMark` is tagged by `kind`: `rename`, `remove`, `add_task`, `add_group`, `move_task`, `subagent`, `note`. 422 `fleet.plan_mark_refused` |
| `SessionRecord.plan` | Fleet writes, Bridge draws | `SessionPlan`: `version`, `recorded_at`, `approved`, `approved_at?`, `scope[]`, `groups[]`, `changes?`, `sketch?`. A task is `open`, `working`, `handed_in`, `done` or `failed` and carries `seen?`; a group `worked_at?` |
| `SessionRecord.touches` | Fleet writes, Bridge draws | `FileTouch[]`, oldest first, at most 60: `by?` (`agent`, `id`), `path`, `act` (`write`, `edit`, `remove`), `at`. Left out while empty |
| `WaitingSource` `plan` | Fleet writes, Bridge matches | A recorded plan not yet approved; id `plan:<version>` |
| `SessionRow` `plan`, `plan_changes` | Fleet writes, Bridge draws | `plan`: where the plan was recorded. `plan_changes`: `version`, `approved?`, `lines[]` of `verb`, `place`, `from?`, `text?` |
| `SessionFact` `touched` | The mod writes | Gains `by?` (the subagent's `agentId`) and `act?` (`write` or `edit`) |

**Optional members are left out, never sent as `null`.** A Bridge that predates this ignores the members it does not read, and draws a `plan:` waiting item by its text; a `plan` or `plan_changes` row it does not know is one it skips. A Fleet that predates sends none, and a mod that predates `by` and `act` tells a touch with neither, which reads as the Job's own edit. Migration `20261016T1500Z-session_plan`: five tables, none read by anything else.

## Protocol 23.91: a Job's request is read, and its workflow is on the record

`docs/concepts/session.md`, *A Job's proposal*, and `docs/concepts/job-proposer.md`, *A Job's request*. **Additive only**: one optional field on a request, one operation, two fields on a record, three timeline kinds.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_session` | `StartSession` gains optional `request` | The words the person dispatched. Fleet reads them once with the Job proposer's model and fills in the `title`, `model` and `effort` the start left unset, and the Job's `workflow`. **The answer is the `SessionRecord` at once**, with `proposal.reading` true and no process; the process starts when the answer lands. Not read with `pilot`, `fork` or `root`. A call that fails or cannot be read starts the Job on the machine's defaults and nothing is refused |
| `set_session_workflow` (`POST /sessions/workflow`) | `SetSessionWorkflow`: `session_id`, `workflow?`. Answers 200 `SessionRecord` | Sets the workflow a hosted Job follows, or clears it (absent) so it is a custom Job. Fleet tells the agent at once and enters `session.workflow`. 422 `fleet.session_workflow_not_held` for an id the repository does not hold, 422 `fleet.session_command_refused` for a terminal Job, 409 `fleet.session_closed` for one that ended. `agent_access: No` |
| `SessionRecord` | `workflow?`, `proposal?` | `workflow` is absent for a custom Job. `proposal` is `{ reading, picked }`, each skipped where false or empty; `picked` lists `title`, `workflow`, `model` and `effort` as the proposer filled them, and a field leaves it when the person sets it |
| `timeline.entry` | `TimelineKind` gains `session.workflow`, `session.model`, `session.effort` | `says` is `workflow: <old> → <new>`, `model: <old> → <new>` or `effort: <old> → <new>`, naming a workflow by its name and none as `custom`, a model or effort not set as `the default` |
| `tune_session` | Behaviour only | A changed `model` or `effort` is now told to the agent on the next turn that starts on it, and entered as above. No wake |

**Skew.** A Fleet before 23.91 ignores `request` and answers 404 for `/sessions/workflow`; a Bridge before it never sends either. A Bridge before reads a `session.*` timeline kind it does not know and draws nothing or draws it badly, and ignores the two record fields. The IDs differ, so the two are refused regardless and the lifeboat serves. Bridge's half is `packages/protocol/src/{hosted-sessions,sessions,timeline}.ts`, written by hand like the rest.

## A Job's rail and its judges

Since 16 Oct 2026. `docs/concepts/session.md`, *The rail* and *The judges*. **Additive only**: three operations, four fields on a record, two on a waiting item, one source and one timeline kind. The protocol ID is derived from the wire files and needs no bump.

| Where | Carries | Notes |
| --- | --- | --- |
| `SessionRecord` | `workflow_step?`, `judging?`, `judge?`, `settled?` | `workflow_step` is the index of the step Fleet observed the Job on; absent for a custom Job, past the last step, and while `features.workflows` is off. `judging` is a judge reading now (in memory). `judge` is `{ at, concerns }` of the last reading, `concerns` 0 being *No concerns*. `settled` is the concerns the person settled, oldest first, at most 40, each a `judge` item with `concern.settled` |
| `WaitingItem` | `concern?`, `cites?` | `concern` is `{ from, settled?, expected?, produced?, consequence? }`, `from` being `{ kind: "tile" }` or `{ kind: "step", index, label }`; `settled` is `{ how: "sent" or "dismissed", at }`; the three parts are the judge's refusal, each one short sentence, which `text` joins for the agent, and are absent on a concern kept before they were. `cites` is `[{ path, line? }]`, the files of the diff the judge's quotation landed on |
| `WaitingSource` | `judge` | id `judge:<n>`, `act` a `message` naming the Job. Sent or dismissed from its own card; `answer_waiting` and `dismiss_waiting` refuse it with `fleet.session_waiting_empty` |
| `judge_session` (`POST /sessions/judge`) | `JudgeSession`: `session_id`. Answers 200 `SessionRecord` | Answers at once with `judging` true; the reading runs in a task and the row is published when it ends. 409 `fleet.session_closed`, 422 `fleet.session_command_refused` for a terminal Job. `agent_access: No` |
| `send_concern` (`POST /sessions/judge/send`) | `SendConcern`: `session_id`, `item_id`. Answers 200 `SessionRecord` | The concern becomes the person's message to the agent, sent before it is settled. 409 `fleet.session_waiting_unheld` for an item not waiting. `agent_access: No` |
| `dismiss_concern` (`POST /sessions/judge/dismiss`) | `DismissConcern`: `session_id`, `item_id`. Answers 200 `SessionRecord` | Settled; the agent is told nothing. Same 409 |
| `timeline.entry` | `TimelineKind` gains `session.step` | `says` is `step: <old> → <new>` by label, `start` before the first and `done` past the last |
| `set_session_workflow` | Behaviour only | 422 `fleet.session_workflows_off` to set a workflow while `features.workflows` is off; clearing one still works |
| Settings | `features.workflows` (boolean, live, shipped on) | Served with the rest; Bridge hides the workflow picker and the steps when it is off |

**Skew.** A Fleet before this answers 404 for the three routes; a Bridge before it ignores the record fields and draws a `judge` source as an item it has no card for. The IDs differ, so the two are refused regardless and the lifeboat serves. Bridge's half is `packages/protocol/src/{session-rail,sessions,timeline}.ts`, written by hand like the rest.

## A terminal's permission prompt is a call

Since 14 Oct 2026. `docs/concepts/session.md`, *A terminal's permission prompt*; measured in `docs/spikes/031-can-bridge-answer-a-terminal-permission-prompt.md`. **Additive only**: two `SessionFact` variants a terminal's mod reports, an optional `tool` on the mod's `asks`, and a `WaitingItem` with the existing `permission` source. No operation, event kind or error is new. Bridge's mirror of the facts and of `asks` is the mod's `plugins/armada/hooks/fleet.ts`; `packages/protocol` is unchanged, because the item and the open ask are existing shapes.

| Where | Carries | Notes |
| --- | --- | --- |
| `SessionFact::PermissionAsked` (`report_session`) | `tool`, `detail?`: the command's first line, the path or the address, cut at 300 characters | The harness opened its own permission prompt (Claude Code's `PermissionRequest` hook, where no hook beneath decided it). Ignored for a hosted Job, whose permission is Fleet's own ask |
| `SessionFact::PermissionSettled` | nothing | The tool call that raised the prompt ended, whichever way it was answered |
| `TerminalAsk::Asks` (`ask_from_terminal`) | `tool?`, since mod `0.3.22` | The mod puts the prompt to Fleet as it puts a question, and polls for the answer. Absent is `AskUserQuestion`. A permission offers Allow once and Refuse, never Allow and remember, draws no row in the thread, and is not refused while a walk is open |
| `WaitingItem` (`source: permission`) on a terminal Job | `id` `perm:<call>`, `text` the tool and its detail, `act` `answer`, `options` Allow once and Refuse | The open ask, as a hosted Job's is: `answer_session` and `answer_waiting` reach the polling mod as `answered` or `refused`. The terminal answering first settles it as answered there |
| `WaitingItem` from the fact alone | `id` `perm:<when Fleet heard it>`, `act` `message` naming the Job, no `options` | Until the ask arrives, and for a mod that reports the fact but cannot put it. `answer_waiting` on it is refused as `fleet.session_waiting_unheld`: only the terminal answers it |

**Held, not stuck.** While a prompt is open the terminal's turn is waiting on the person, so the stuck tick leaves it out, as it does a hosted Job's open ask. A turn starting or ending, and the Job ending, closes the prompt in Fleet without a `permission_settled`. It is kept in memory only; the mod tells it again when a Fleet that was out of reach answers.

**Skew.** A mod before `0.3.22` reports neither fact and puts no permission, so its Job's prompt raises no call and reads stuck after ten minutes, as before. A Fleet before this refuses both facts as unreadable and reads a permission's `asks` as a question; the mod's report is not awaited, so that costs the job nothing.

## Helm is a Job

`docs/concepts/helm.md`, `.claude/decisions/2026-10-13-helm-is-a-session.md`. Helm's dock moves off its own host and socket onto the hosted-Session engine. Bridge has moved with it: the dock reads the Helm Job through the Jobs pipeline and calls only the routes below.

| Where | Side | Change |
| --- | --- | --- |
| `SessionRecord.helm` (`ipc::SessionRecord`, `SessionRecord.helm?: true`) | Fleet writes, Bridge draws the mark | `true` on a repository's Helm Job, skipped when false. Marks it in `list_sessions` and in `get_session` |
| `AskHelm.model`, `AskHelm.effort` | Bridge writes | Optional. The model and effort for the Job this message starts. Ignored once one is live: `tune_session` changes a live one. 422 `fleet.unacceptable_effort` for anything but `low`, `medium` or `high` |
| `ask_helm` (`POST /helm/ask`) | Bridge calls | Answers 200 `SessionRecord` (was 202 `HelmConversation`). Finds or starts the repository's Helm Job, sends the message with the context line ahead of the words, and returns. The reply is `session.row` |
| `get_helm_session` (`GET /helm/session?manifest_id=`), new | Bridge calls | Answers `HelmSession`: `manifest_id` and `session?`, a `SessionThread`. `session` is absent before the first message and after Start fresh |
| `start_helm_fresh` (`POST /helm/start_fresh`) | Bridge calls | Answers `HelmSession` (was `HelmConversation`). Ends the live Helm Job with `close_session`, giving back the slot it held. The next message starts a new one |
| `record_thought_sent` (`POST /helm/thought_sent`) | Bridge calls | Answers `SessionRecord`. Appends a `sent` row to the Helm Job's thread, starting the Job if the repository had none |
| `SessionRow::Sent` (`kind: "sent"`) | Fleet writes, Bridge draws | `id`, `at`, `text`, `target: ThoughtTarget`, `files?`, `tags?`. Only a Helm Job's thread has one, written by `record_thought_sent` and by `tell_session` |
| `tell_session` (`POST /helm/tell_session`) | Helm calls | Unchanged on the wire. Its `Helm only` reach and its `sent` row are what changed |
| `get_helm_debug_info` (`GET /helm/debug`), `HelmDebugInfo` | Fleet writes, Bridge draws | `brief` renamed `note`; `servers` removed. `session` is the Helm Job's id and is absent when none is live |
| `HelmDebugSaid` | Fleet writes, Bridge draws | `asked`, `said`, `called`, `refused`, and `sent` (`text`, `to`). `ended`, `fresh` and `unanswered` are gone: a Job's thread has no such rows |
| `get_session_file` | Bridge calls | Also serves the files of a `sent` row |
| `observe_commander`, `read_in_link`, `helm.call_answered`, `helm.thought_routed` | Fleet | Descriptions only. No field changed |
| `prompts.helm`, `prompts.helmReadOnly` (settings) | Machine setting | Placeholders `{manifest}`, `{folder}`, `{unasked}` and `{manifest}`, `{folder}`. The brief's texts and their placeholders are gone. `agent-prompt.md` section 5a |

**Retired from the wire.** Operations `observe_helm`, `tune_helm`, `get_helm_file` and the event `helm.changed_checkout`; types `TuneHelm`, `HelmBoxFacts`, `HelmConversation`, `HelmMessage`, `HelmOpened`, `HelmClosed`, `HelmSilence`, `HelmChangedCheckout` and `HelmSent`. They are deleted in Rust and in `packages/protocol/src` (`helm-thread.ts` with them). `observe_commander` and the Commander's types keep the old host. A route typo is a runtime 500 here as everywhere (the accepted cost), so the dock should be walked against a Fleet that has this change.

**What lands in Bridge.**

| Where | Side | Change |
| --- | --- | --- |
| `BridgeState.helm` (`HelmAim`: `{state: "none"}` or `{state: "pointed", manifestId}`) | Bridge | Was `HelmThread`, the dock's whole conversation. It is only which repository Helm answers for. The thread, the turn, the model and the commands are the Helm Job's, in `BridgeState.sessions` |
| `HelmCarry.model`, `HelmCarry.effort` | Bridge | Optional. What the box held before the Job existed, sent as `AskHelm.model` and `AskHelm.effort` on the first message. Once the Job is live the box tunes it with `tune_session` |
| `SessionsDraft.helmOf(manifestId)` | Bridge | The Helm Job of the repository Helm is pointed at, which the Jobs list does not hold when the picker picks another repository |
| `Session.helm` (`sessions-wire.ts`) | Bridge | Read from `SessionRecord.helm`. The Jobs list draws Helm's mark, and opening that row opens the dock |
| `rowsOfThread`, kind `sent` | Bridge | Draws `SessionRow::Sent` as the person's words with a "Sent to" chip. `Beside.repository` names the target's repository |
| `HelmConnection` (`apps/desktop/src/main/helm.ts`) | Bridge | No socket and no thread. `ask_helm` and `record_thought_sent` answer a `SessionRecord`, which main folds into the Jobs host at once. `HelmSocket` is gone |
| `tuneHelm`, `readHelmFile` (preload and `ipc` channels) | Bridge | Removed. `tune_session` and `get_session_file` serve the same acts |

**How the dock reads a thread.** `get_helm_session` once, then `session.row` and `session.changed` on `/events` by the job's id, and `session.hosted` for `commands`, `model`, `effort` and `turn`. A Bridge that reconnects reads `get_helm_session` again, as a Job page does: that is what the `missed` message on `/events` is for.

**The event stream, and the open risk.** Better in one respect and no worse in any other. The Helm socket had a bounded channel of its own, per conversation, with a drop-oldest `missed` message; that is gone, and with it a second thing that could outrun a slow viewer. Helm's thread now rides `session.row` and `session.changed` on the shared drop-oldest `/events` channel. It adds no queue and no event kind. **It does add rows**: a Helm Job streams what a Job streams, including its tool calls, and a Helm Job that works hard against a minimised Bridge is a source like any Job's. The unbounded sink on the application side of axum's WebSocket stays as it was, unmeasured. A viewer that fell behind learns it from `missed` on `/events` and resyncs with `get_helm_session`. Nothing here makes the "you missed N events, resync" message less necessary.

**Skew.** None to meet beyond the rule: Protocol IDs equal or Bridge opens no socket and the lifeboat is the fallback. The change touches the wire, so the ID moves; there is no order between two IDs and nothing here depends on one. **The lifeboat is untouched**: `/v0/` is still four hand-written routes with no dependency, and none of this is on them.

**What is not on the wire.** The old Helm thread files under the records root are not read. The Helm Job's `helm` flag is on the store's `hosted_sessions` row (migration `20261013T1700Z-helm_session.hosted_helm`), not a second table; `live_helm_of` and `latest_helm_of` find it.

## A waiting item's steps

`docs/concepts/session.md`. **Additive only**: one optional field on each of two DTOs. No operation, event or route.

| Where | Carries | Notes |
| --- | --- | --- |
| `WaitingInput.steps?` (`waiting_for`, Fleet) | The procedure the person follows, in order, as short lines | Fleet trims each and drops a blank one. An empty list is left off, never sent as `null` |
| `WaitingItem.steps?` (Bridge) | The same lines, as the agent's item was kept | Only an agent's item carries them. Bridge opens a press on one as a panel beside the thread, and Done or Send answers through `answer_waiting` `text`: Done is the word `Done`, with the person's corrections as lines after it |

**Skew.** The ID moves, so a Bridge and a Fleet that differ are refused and the lifeboat serves. Bridge's half is `packages/protocol/src/sessions.ts`, written by hand. **Kept.** `session_waiting.steps`, a JSON array column (store migration `20261015T1200Z-session_waiting.steps`), additive with a default of `[]`, so an item kept before it reads as no steps. **The event stream.** `session.row` carries the lines with the item, a few short lines at most, so this is neither better nor worse for the unbounded sink.

## A Job moves to another repository

`docs/concepts/session.md`, *A moved job*. **Additive only**: one operation, one DTO, three error codes and a detail on a ledger row that already crosses the wire.

| Where | Side | Change |
| --- | --- | --- |
| `move_session` (`POST /sessions/move`) | Bridge calls, an agent may | `MoveSession`: `session_id`, `manifest_id`. Answers 200 with the new Job's `SessionRecord`. `agent_access` Yes |
| `fleet.no_such_session`, `fleet.no_such_manifest` | Fleet refuses | 404 for a Job or repository Fleet does not hold. `fleet.no_such_session` is 422 as `start_session`'s `fork` answers it and 404 here |
| `fleet.session_move_same`, `fleet.session_move_listening` | Fleet refuses | 409 for a move into the Job's own repository, and for a terminal Job whose mod is still asking |
| `Attachment.detail.moved` on `forked_to`, `forked_from` | Fleet writes, Bridge reads | The id of the repository on the other side. Present only on a move |
| `Attachment.detail.branch` on `forked_from` | Fleet writes, Bridge reads | The old Job's branch or branches, comma-separated. Present only on a move that left one |
| `moveSession` (preload) | Bridge | `moveSession(sessionId, manifestId)`, answering `SessionActed` as `startSession` does |

**Skew.** The ID moves, so a Bridge and a Fleet that differ are refused and the lifeboat serves. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand. There is no number to take: `armada need` has nothing to order. Attachment kinds and details are open text, so no type changes and a Bridge that has not met `moved` draws the pair as a fork. **The event stream is untouched** beyond the `session.changed` each Job already publishes, so the back-pressure risk below is neither better nor worse. The lifeboat is untouched.

## An issue is named by its tracker's reference

`docs/concepts/tracker.md`. An issue crossed the wire as a number, which names nothing once a tracker has more than one project and drew as `#123` for Jira's `ABC-123`. The protocol ID moves with the files below, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `TrackedIssue.number` (`list_issues`) | Fleet writes, Bridge draws | Replaced by `reference`: the tracker's own name as a person writes it, `#123` on the forge and `ABC-123` on Jira. Bridge draws it as it is and never builds one |
| `IssueRead.number` (`get_issue`) | Fleet writes, Bridge reads | Replaced by `reference`, as above |
| `get_issue` path | Bridge calls | `GET /issues/:repository/:reference`, percent-encoded. A forge takes `214` or `#214`; Jira takes a key. A reference the tracker does not use, such as a bare number on Jira, is a 404 `fleet.no_such_issue` |
| `AutopilotPick.issue_number` | Fleet writes, Bridge draws | Replaced by `issue_reference`. The store keeps `issue_number` for picks written before the column, and reads one with no reference as `#` and its number |
| `getIssue` (preload) | Bridge | Takes the reference as text where it took a number |

**The `#` picker takes a key as well as digits.** An issue's tag id is its key where the tracker names issues by one, and the picker matches what is typed against the reference, ignoring case. A lookup by number reads only numbers. **The event stream is untouched**: no event carries an issue, so the back-pressure risk below is neither better nor worse. The lifeboat is untouched.

## A bay's history: the pool's own record, and the slot's reflog

`get_slot_history` (`GET /slot_history?manifest_id=&slot=`) answers the Cockpit's Bay pane: who held one slot, and what was checked out in it, newest first, for thirty days, at most 300 (`MOST_BAY_HISTORY`) with `cut` when older ones were left out. Since 15 Oct 2026. `agent_access` is `No`.

| Entry | Source | Why not the timeline |
|---|---|---|
| `leased`, `released` | `slot-<n>.history`, which the pool writes beside the lease record | The timeline's `slot.leased` is written by Fleet's own calls, so `armada worktree lease` never reaches it. A slot held 65 hours by an agent's lease had no history at all |
| `checkout` | The slot's own `HEAD` reflog, `<git dir>/worktrees/<slot>/logs/HEAD`, read as a file | Checkouts inside a bay are recorded nowhere else |

**The pool writes the lease line, in whichever process takes the lease.** Fleet noticing a record change was rejected: a read of the record misses a lease taken and given back between two reads and every one taken while Fleet is down, and the lease command is a process of its own. The pool already holds the slot's lock when it writes or clears the record, so the line goes down there (`crates/adapters/src/leasing/history.rs`) and no caller has to remember. A failure to write it never fails the lease. The file is cut to thirty days on its next write.

**A lease taken from a holder that had gone, without giving the slot back, is entered as that holder's release** first, dated when the lease took the slot: nothing dates the holder's going, and two leases in a row would leave a reader to guess.

**Fleet names the holders when asked.** The pool writes a process (pid and start time, with the agent session where its record has one) or an id. A Job's lease is its id, a hosted Session's is a Job of its own id, and a repair's is `<job>-repair-…`, so an id Fleet has no Job for is a `session`. A process is an `agent`, with the Session as `id` where the record has one and the pid where not. Titles are read when asked and not kept with the entry, so a Job forgotten since has none. This is the same resolution `list_worktrees` makes for a slot's holder.

**Additive.** One new operation, `BayHistory`, `BayHistoryEntry` (tagged by `kind`, strict), `BayHolder` and `BayHolderKind` (strict). Bridge's half is `packages/protocol/src/bay-history.ts`, written by hand, and `getSlotHistory(manifestId, slot)` on the preload. Every optional field is left out when empty. There is no number to take and `armada need` has nothing to order: the protocol ID moves with the wire files.

**The event stream is untouched**, so the back-pressure risk above is neither better nor worse. The lifeboat is untouched. The read is two small files and the reflog on each ask, in a blocking task, with nothing kept; a reflog of thirty days of an agent's `git switch` is the largest of them.

## A Job walks any page: `steps` on `show_window`

`docs/concepts/session.md`, *Showing a window*, and `docs/practices/capture-window.md`, *A walk in the page*. A Job in any repository passes the mock walks' own steps with the page it shows, and Bridge plays them from outside the page. The protocol ID moves with the files below, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `ShowWindow.steps` (`show_window`, `POST /sessions/window`) | Job writes, Fleet checks | `Vec<WalkStep>`, empty and left out when there are none. A step is `say` and exactly one verb of `press`, `look`, `hover`, `later`, `type` (with `into`), `key` (with `on`), `select` (with `in`), `paste` or `drag` (with `by`); a target is `{role, name?, exact?, within?}`. `ipc::walk_steps`, strict: a key it does not know is a 400 from the decode, naming it |
| Fleet's check (`sessioning.rs`, `steps_shown`) | Fleet refuses | 422 `fleet.session_unnamed`, nothing written, naming the first step that is wrong: no or more than one verb, a verb without its own target, a target with no role, a caption missing or too long, nesting too deep, steps that weigh more than the bound written out. The bounds are the constants beside it |
| `SessionRow::Window.steps` (`session.row`) | Fleet writes, Bridge reads | The steps the window carries, kept when the walk ends as the takes are. Bridge opens the window with them when the row arrives |
| `Attachment.detail.steps` on the `window` artifact | Fleet writes, Bridge reads | The same steps as a JSON string, because `detail` holds strings only; `openSessionWindow` reads them back from the ledger. Anything that is not a list of steps in Fleet's shape reads as none in Bridge |
| The door's receipt for `show_window` | Fleet writes | `steps`, the captions in order, left out where there are none |
| `window_failed` (`POST /sessions/window/failed`) | Bridge writes | Unchanged in shape. A `why` that begins `step` is a step Bridge could not play, and Fleet tells the Job to fix its steps rather than that its page failed to load |
| `WalkTarget`, `WalkStep`, `ShowWindow.steps`, `SessionRow` `window` `steps` | Bridge reads | `packages/protocol/src/sessions.ts` and `hosted-sessions.ts`, written by hand like the rest |

**Additive.** Every new field is optional and left out when empty. A Fleet before this one ignores `steps`, and a Bridge before it ignores the field on the row. The IDs differ, so a Bridge and a Fleet that differ are refused regardless. There is no number to take: `armada need --status` showed nothing standing on these paths, and no migration is needed since the steps ride rows and an artifact's open `detail`. **The event stream is untouched** beyond the `session.row` a window row already publishes, so the back-pressure risk below is neither better nor worse. The lifeboat is untouched. The steps ride the record on the window artifact, which is published whole on every change, so they are bounded the way a sketch's scene is.

## A Job's window: origins and a sign-in

Since 15 Oct 2026. `docs/concepts/session.md`, *Showing a window*, and `docs/practices/capture-window.md`, *Origins a Job names*. **Additive only**: optional fields on `ShowWindow` and on the `window` row, and an operation with its DTOs. No event kind or error code is new. Bridge's half is `packages/protocol/src/sessions.ts` and `hosted-sessions.ts`, written by hand. There is no number to take: the protocol ID moves with the wire files, and `armada need --status` showed nothing standing on these paths.

| Where | Side | Change |
| --- | --- | --- |
| `ShowWindow.origins?` (`show_window`) | The agent writes, Fleet checks | A list of `http` or `https` origins, a host and an optional port with no path. A call with another entry is 422, nothing written. Left out when empty |
| `ShowWindow.sign_in?` (`show_window`) | The agent writes, Fleet holds | A list of addresses, each on the shown origin or a named one (422 otherwise). **A secret**: the type's `Debug` prints a count, and the field is in no row, record, event, timeline entry or receipt. Left out when empty |
| `SessionRow::Window.origins?` (`session.row`) | Fleet writes, Bridge reads | The origins, kept when the walk ends as `variations` are. The ledger keeps them too, as the window artifact's `detail.origins`, a JSON string |
| `take_window_sign_in` (`POST /sessions/window/sign_in`) | Bridge asks, Fleet answers | `TakeWindowSignIn` (`session_id`, `url`) answers `WindowSignIn` (`sign_in`). Once: the addresses leave Fleet's memory with the answer, and it forgets them after two minutes. `agent_access` `No` |

**Why a call and not the row.** The row is stored, replayed to every viewer that reads a thread, and folded into the state Bridge's renderer holds, so a login on it would be kept and shown far from the window that needs it. The call hands it to main once.

**Skew.** The ID moves, so a Bridge and a Fleet that differ are refused and the lifeboat serves. A Bridge that predates the row field opens the window with no origins, as it has no word for them. The migration is none: the ledger's `detail` holds strings and `origins` is one more.

## The Reviews tab: the lists waiting on a person, a page's threads and a review's comments

`docs/concepts/pull-review.md`. Bridge reads the pull requests waiting on a person from the forge through Fleet, opens any pull request of a served repository, and posts a review as one call with its line comments. **Additive**: two operations, new DTOs in `ipc::pull_reviews`, and optional fields on the page and the write of *Protocol 23.83* above. Bridge's half is `packages/protocol/src/pull-reviews.ts` and `pull-requests.ts`, written by hand like the rest.

| Where | Side | Change |
| --- | --- | --- |
| `get_review_lists` (`GET /review_lists`) | Bridge reads | `PullReviewLists`: `me`, `fetched_at`, and `requested[]`, `team[]`, `assigned[]`, `mine[]` of `PullReviewRow`, each capped at `MOST_PER_LIST`. `Bridge only` |
| `PullReviewLists.silent` | Fleet writes, Bridge reads | `Vec<PullReviewList>`, left out when empty: a list the forge would not answer, so Bridge keeps what it held. All four silent is a 500 `fleet.review_lists_unreadable` |
| `PullReviewRow.team` | Fleet writes, Bridge reads | The team asked, left out off the Team list |
| `PullRequestPage.head`, `.body` | Fleet writes, Bridge reads | The whole head commit and the description, left out where the forge gave none |
| `PullRequestPage.threads` | Fleet writes, Bridge reads | `Vec<PullReviewThread>`, left out when empty: `id`, `path`, `line?`, `side`, `resolved`, `comments[]`. A page whose threads the forge would not give is a 500 `fleet.pull_request_unreadable`, not a page |
| `WritePullRequest.comments` | Bridge writes, Fleet reads | `Vec<PullReviewComment>`, left out when empty: `kind` `line` (path, line, side), `file` (path) or `reply` (thread), each with `body`. Set, the write is one review with `kind` as the verdict, and `body` may be empty |
| `WritePullRequest.commit` | Bridge writes, Fleet reads | The commit the person read, left out for the branch as it stands |
| Fleet's check (`pull_request_panel.rs`, `write_of`) | Fleet refuses | 422 `fleet.pull_request_write_blank` for a comment with a blank `body`, as for a plain comment, nothing sent to the forge |
| `resolve_review_thread` (`POST /review_threads/:repository`) | Bridge writes | `ResolvePullThread` (`thread`, `resolved`), answers `PullThreadResolved`. A 409 `fleet.review_thread_refused` carries the forge's sentence. `Bridge only` |

**Additive.** Every new field is optional and left out when empty. The IDs differ, so a Bridge and a Fleet that differ are refused regardless.

## The Reviews tab: agent runs and review tools

Since 16 Oct 2026. `docs/concepts/pull-review.md`, *Agent runs*. **Additive only**: five operations, new DTOs in `ipc::review_runs`, one event, one optional field on a row, and a setting. Bridge's half is `packages/protocol/src/review-runs.ts` and `events.ts`, written by hand like the rest. There is no number to take: the protocol ID moves with the wire files, and `armada need --status` showed nothing standing on these paths.

| Where | Side | Change |
| --- | --- | --- |
| `start_review_run` (`POST /review_runs/:repository`) | Bridge writes, Fleet answers | `StartReviewRun` (`number`, `kind` `tool`, `explanation` or `ask`, `name`, `again`) answers `ReviewRun`. **Answers at once**: a kept result as it stands, the run already going, or a new one `queued`. 422 `fleet.review_unknown_tool`, 422 `fleet.review_blank`, 500 `fleet.review_pull_request_unreadable`. `Bridge only` |
| `get_review_runs` (`GET /review_runs/:repository/:number`) | Bridge reads | `ReviewRuns`: every run kept or going for the pull request, one for each kind, name and head it ran on, newest first. `Bridge only` |
| `list_review_tools` (`GET /review_tools/:repository`) | Bridge reads | `ReviewTools`: `tools[]` of `ReviewTool` (`id`, `kind`, `name`, `prompt`, `source`, `replaces?`) and `left_out[]` (`file`, `source`, `said`), left out when empty. `Bridge only` |
| `save_review_tool` (`POST /review_tools/:repository`) | Bridge writes, Fleet checks | `SaveReviewTool` (`scope` `kit` or `repository`, `kind`, `name`, `prompt`, `overwrite`) answers `ReviewTools` read again. 422 `fleet.review_tool_blank`, `fleet.review_tool_not_a_name`, `fleet.review_tool_exists`; 500 `fleet.review_tool_unwritable`. `Helm only` for an agent |
| `review_run.changed` | Fleet publishes, Bridge reads | `{ kind, run }`: the run whole, **nested under `run`** because the run has a `kind` of its own that a flattened event would overwrite. Not replayed |
| `ReviewRun` | Fleet writes, Bridge reads | `manifest_id`, `number`, `kind`, `name`, `head` (the whole commit it ran on), `state` (`queued`, `running`, `done`, `failed`), `started_at`, and when set `finished_at`, `failed` (Fleet's reason), `result` and `cost_micros` |
| `ReviewResult` | Fleet writes, Bridge reads | Tagged by `shape`: `themes`, `impact`, `steps`, `reason`, `text` or `findings`. A place is `ReviewPlace` (`path`, `side`, `start`, `end`) and a step's source is `description` or `doc` with a `path` |
| `PullReviewRow.holder?` | Fleet writes, Bridge reads | `PullReviewHolder` (`id`, `title`): the Session of the person's that holds the pull request, from the pull request owners. Left out when none does, and never a Job |

**An answer is validated before it is kept**, and one that is not valid is a `failed` run with `failed` set and no `result`. Fleet checks that every place names a file the pull request changes, that a range starts at line 1 or later and does not end before it starts, that a step's doc exists in the checkout at the head, and that a link is http or https.

**A kept run is keyed by repository, pull request, head, kind and name** (`review_runs` in the store, migration `20261016T1700Z-review_runs.kept.sql`, a new table and nothing altered). A reader marks a run out of date when it holds a head newer than the run's `head`. `name` is a definition's id, or for `ask` the question's words lower cased and single spaced.

**One setting.** `models.review` (Model section, `live`, no environment variable) ships as the Job model.

**Skew.** The IDs differ, so a Bridge and a Fleet that differ are refused regardless, and the lifeboat serves. Nothing here touches the four `/v0/` routes.

**The event stream: neither better nor worse.** A run adds three small events, `queued`, `running` and `done` or `failed`, on the one bounded drop-oldest broadcast every event shares. No text is streamed and no queue is added, so a run costs three events however long its answer. The message `review_run.changed` carries is not replayed, which is why a Bridge that missed some, or reconnected, reads `get_review_runs` again when it opens a pull request and on Refresh, and never believes the events alone are complete. The unmeasured risk above is unchanged: nobody has a number for how many events a real fleet produces against a minimised Bridge.

## A Job's running Check: its log live, and a stop

Since 16 Oct 2026. `docs/concepts/session.md`. **Additive only**: one value on `SessionCheckAct` (`chunk`), one optional field on `SessionCheckAnswer` (`stop`), `SessionCheckStop`, `SessionCheckOutputMessage` and its opening, and three operations, `observe_session_check` (`GET /sessions/checks/:run/observe`, a WebSocket), `stop_session_check` and `force_stop_session_check` (`POST /sessions/checks/:run/stop` and `/force_stop`), all `agent_access` `No`. Fleet and Bridge both carry them; Bridge's half is `packages/protocol/src/sessions.ts`, written by hand. There is no number to take: the protocol ID moves with the wire files, and `armada need` has nothing to order.

**Fleet never reaches the Check's process.** `armada check` runs in the Job's own process, so it tells Fleet what it printed in `chunk` calls (a quarter-second apart) and Fleet answers each with the stop a person asked for. `armada check` then ends its own process group: `SIGTERM` and `SIGKILL` after a grace period, or `SIGKILL` at once, and tells `end` as for any run. Fleet holds a running run's log in memory, up to 8 MiB, and serves it over `api::Follow`, so one reader draws a Job's Check, a merge line's and a Job's.

**Skew.** The IDs differ, so a Bridge and a Fleet that differ are refused regardless. **The event stream is untouched**, so the back-pressure risk below is neither better nor worse: the log has its own socket and reads from a cursor, so nothing on it is dropped. The lifeboat is untouched.

## A Job's reach past its slots: the folder answers

Since 16 Oct 2026. `docs/concepts/session.md`, *Modes*, and #2441. In `auto` a guarded read or a write outside the slots is put to the owner on the job's card, and two answers put a folder on a list the guard reads first. The protocol ID moves with the files below, and there is no number to take: `armada need --status` showed nothing standing.

| Where | Side | Change |
| --- | --- | --- |
| `HelmCallAnswer` `always_allow`, `always_block` | Bridge sends, Fleet takes | Offered only on a card whose `reaches` is set, beside `allow_once` and `refuse` and never with `allow_and_remember`, whose rule matches a command and not a folder. Helm's dock never offers them |
| `HelmCallInFlight.reaches?` (`session.row`, the hosted record's `asked`) | Fleet writes, Bridge reads | The absolute path the guard named first: a tool's `file_path` or `path`, a shell line's first refused word with `~` and `$HOME` read as the home. Left out for any other call |
| `AnswerSessionAsk.folder?`, `AnswerHelmCall.folder?` (`answer_session_ask`) | Bridge sends, Fleet checks | The folder an "always" answer is for: `reaches` or a folder above it, compared a part at a time. Without one that holds `reaches` it is a 422 `fleet.session_answer_not_a_folder`, and the card stays |
| `SessionAskState` `always_allowed`, `always_blocked` (`session.row`) | Fleet writes, Bridge reads | How a card settled where the folder went on the list |
| `HelmCallAnswer` `allow_for_session` | Bridge sends, Fleet takes | Since 16 Oct 2026, offered beside the two "always" answers and carrying `folder` the same way. Fleet keeps the folder on the hosted job's in-memory state (`session_host::State::allowed_folders`), read by the guard after the owner's list, and clears it when the job is closed. Nothing is written to `features.reachFolders` |
| `HelmCallInFlight.reaches_to?` (`ReachesTo`: `read`, `write`) | Fleet writes, Bridge reads | Since 16 Oct 2026. Whether a reach card's call reads the path or writes to it: `Read`, `Grep`, `Glob`, `LS` and a shell segment of reading programs with no redirect are `read`, everything else `write`. Set where `reaches` is, and where the path could not be read as one. Left out for any other call |
| `SessionAskState` `allowed_for_session` (`session.row`) | Fleet writes, Bridge reads | How a card settled on `allow_for_session` |
| `ipc::reach_folders::ReachFolders` (`features.reachFolders` in `get_settings`, `save_settings`) | Both | `{allow?, block?}`, absolute paths, a `Json` setting in Features. Fleet writes it on an "always" answer; Settings removes an entry by saving the list without it, or `null` when both are empty. Read leniently: an entry that is not an absolute path is dropped |
| `ReachFolders`, `reachFoldersOf`, `REACH_FOLDERS_KEY`, the fields above | Bridge reads | `packages/protocol/src/reach-folders.ts`, `helm-calls.ts` and `hosted-sessions.ts`, written by hand like the rest |

**Additive.** Three variants on each of two enums, one new enum (`ReachesTo`), four optional fields left out when empty, and one setting. No operation, event kind or route is new. A Bridge before this draws a reaching card with Allow once and Refuse, since `helmOfferedOf` drops an offer it has no words for, and reads the settled states as words it has none for. A Bridge before `reaches_to` draws the card with no read or write word. The IDs differ, so the two are refused regardless. No migration: the list is a settings.json key. **The event stream is untouched** beyond the `session.row` and `settings.changed` an answer already publishes, so the back-pressure risk above is neither better nor worse. The lifeboat is untouched.

## A Job follows the Jobs its plan was handed to

`docs/concepts/session.md`, *Linked Jobs*. **Additive throughout**; the protocol ID moves with the wire files, and nothing is bumped.

| Where | Carries | Notes |
| --- | --- | --- |
| `StartAnotherSession.group` | The id of a group of the caller's recorded plan. Absent is the start of 23.80 | Fleet titles the Job with the group's title, in the caller's repository whatever `manifest_id` says, and holds the group's tasks word for word, the caller's handoff and `message` as its first message. 422 `fleet.group_unknown`, 409 `fleet.group_handed` |
| `hand_to_session` (`POST /sessions/hand_to_session`) | `HandToSession`: `session_id`, `group`. Answers `SessionHeld` | `agent_access` No: Bridge's. The same start with the first message **sent**, since the press is the person's |
| `Attachment.detail.group` on `started_by` and `started_to` | The group's id | No field is added: the existing `detail` map carries it on both ledgers, so Bridge reads a Job's group off its own record |
| `SessionRow.linked` (`session.row`, `get_session`) | `id`, `at`, `session` (`SessionLink`), `event` (`needs`, `done` or `merged`) | Written in the **parent's** thread where it happened, and the parent's agent is sent a message in the same moment. Bridge draws it as the press that opens `session` |

**Every Job with a `started_by` row is followed, grouped or not**, which is also the rule for nesting it under its parent in Work. **Each notice is told once**: `done` and `merged` are kept in the store (`session_linked_told`, an additive migration), so a restart tells nothing again. `needs` is a move from not needing to needing as the loop saw it, so a Job already needing when Fleet starts is not told.

**The event stream is neither better nor worse.** One `session.row` and one message to a Job's process for each notice, and a notice is rare beside a turn's rows. The unbounded sink and the missing resync message are untouched.

## A Job waits on another

`docs/concepts/session.md`, *A Job that waits on another*. **Additive throughout**; the protocol ID moves with the wire files, and nothing is bumped. The mirror is `packages/protocol/src/session-waits.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `StartSession.after`, `StartAnotherSession.after` (`StartAfter`) | Bridge or an agent sends, Fleet takes | `session` (the Job waited on), `condition` (`StartCondition`, default `ends`), `base` (`StartBase`, default `main`), `needs`. With it `request` is the brief and nothing is sent. 422 `fleet.wait_session_unknown`, `fleet.wait_part_unknown`, `fleet.wait_on_itself`; 409 `fleet.session_closed` for one that ended. Checked before the Job is made |
| `StartCondition` | Both | A tagged union on `kind`: `ends`, `group_done` (`group`), `task_done` (`task`), `pr_pushed`, `pr_lands`. A kind Fleet does not have does not decode |
| `StartBase` | Both | `main` or `branch`, the branch of the pull request of the Job waited on |
| `SessionRecord.waits` (`SessionWait`) | Fleet writes, Bridge reads | `on` (`{id, title}`), `condition`, `base`, `needs`, `brief`, `pull?` (`{number, branch}`, once the Job waited on has one; Fleet picks the first and keeps it), `by` (`you` or `agent`). Present while it waits, absent once it started. Left out when empty |
| `read_session_wait` (`POST /sessions/read_wait`) | Bridge sends | `ReadSessionWait`: `session_id`, `request`. Answers `WaitReading`: `condition`, `base`, `needs?`. `agent_access` No. A reading that cannot be had is the defaults, never a refusal |
| `change_waiting` (`POST /sessions/change_waiting`) | Bridge or the agent waited on sends | `ChangeWaiting`: `waiting_session`, `condition?`, `base?`. Answers `SessionWait`. `agent_access` Yes. 422 `fleet.wait_not_yours`, `fleet.wait_unplaced`, `fleet.wait_part_unknown`; 409 `fleet.wait_over` |
| `start_waiting_now` (`POST /sessions/start_waiting`) | Bridge sends | `StartWaitingNow`: `session_id`. Answers `SessionRecord`. `agent_access` No. 409 `fleet.wait_over` |
| `SessionRow.waiter` (`session.row`, `get_session`) | Fleet writes, Bridge reads | `id`, `at`, `session` (`SessionLink`), `event` (`waits`, `changed`, `started`), `side?` (`waiting`; absent is `waited`), `says?`, `from?`, `to?`, `by?`. In the thread of the Job waited on (`session` is the one that waits), and, for `changed`, also in the waiting Job's own thread with `side` `waiting` |

**Not a wait in the store's sense, a fact on the record.** The wait is kept in `session_waits` (an additive migration: one table and one index, no foreign key) and read into `SessionRecord.waits` on every read of a Job that has not started. The row stays after the start, with `started_at`, so a restart cannot start it twice.

**The event stream is neither better nor worse.** A wait is one `session.changed` for the waiting Job and one `session.row` for the Job waited on, then one of each when it starts. Fleet reads a wait against a Job only when that Job publishes `session.changed`, so a Job nobody waits on costs nothing new.

**Skew.** The IDs differ, so a Bridge before this is refused regardless. A Bridge that draws a record with `waits` it does not know shows a blank Job.

## A full pool asks the person: `slot_choice`

Since 16 Oct 2026. `docs/concepts/session.md`, *A full pool asks the person*. **Additive only**: one value on `WaitingSource`, no operation, no event kind and no route. The protocol ID moves with the files below, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `WaitingSource` `slot_choice` | Fleet writes, Bridge reads | Derived on every read, never kept. Id `slot-choice`, text `Every slot is in use`, `act` `{kind: "answer", target: "slot-choice"}`, `options` exactly `Wait for a slot` (choice 0) and `Add a slot` (choice 1), labels only. Present while the job is in the line for a slot and nobody has chosen |
| `answer_waiting` | Bridge sends, Fleet takes | `choice` 0 or 1 on that item. **No message goes to the agent**: it was told to end its turn when it was queued, and is moved into its slot when one is granted. Words or a mode without a `choice` are a 422 `fleet.session_waiting_empty` |
| `dismiss_waiting` | | Refused 422 `fleet.session_waiting_empty` for a `slot_choice` item: it is answered |

**The same call from a command.** `ask_for_slot` (`POST /sessions/slot_wanted`, `agent_access` `No`) is `armada worktree lease` run by a hosted Job telling Fleet every slot is held. Body `AskForSlot`: `act` (`ask`, repeated about once a second, or `done`) and `session_id`, read by the command from `ARMADA_SESSION`. Answers `SlotAsked`: `standing` (`asking`, `waiting`, `added` or `unasked`). Bridge never sends it, and `packages/protocol/src/sessions.ts` carries both types so the wire reads whole. It raises the same `slot-choice` item, and one item where the Job is also in the line: **one call, never two**. Fleet forgets a command it has not heard from for ten seconds. *Add a slot* for a command with no line makes a slot and holds it for that command until `done` or thirty seconds; for a Job in the line, the slot goes to the Job and the command keeps waiting. A command with no `ARMADA_SESSION`, or no Fleet, does not call it. `armada need` has nothing to order here either.

**Skew.** A Bridge before this meets `slot_choice` as a source it does not know. The IDs differ, so the two are refused regardless. The event stream is untouched beyond the `session.row` and `session.changed` the item already rides on.

## A sketch wireframes a screen: the `screen` node

Since 16 Oct 2026. `docs/concepts/sketch-pane.md`, *Screens*, and `docs/concepts/session.md`, *Drawing a sketch*. **Additive only**: one node kind and one node field in the scene `show_sketch` carries, no operation, no event kind and no route. The protocol ID moves with the files below, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| Scene node `kind` `screen` and field `screen` (`show_sketch`, `POST /sessions/sketch`) | Job writes, Fleet checks | A layout tree of rows, columns, grids and wireframe parts. `crates/ipc/src/sketch/screen.rs`, the same rules as `screen.ts`; a tree on any other kind, or a screen with none, is refused |
| Fleet's check | Fleet refuses | 422 naming the part by its path in the tree, what it holds and what is allowed. Nesting is bounded |
| `Attachment.detail.scene` on the `sketch` artifact | Fleet writes, Bridge reads | Unchanged in shape: the scene is JSON text, so Bridge's `parseScene` reads the new kind with no DTO to mirror |

**Skew.** A Bridge before this draws a scene holding a screen as an empty frame, because `parseScene` refuses a kind it does not know. The IDs differ, so the two are refused regardless. The scene stays under the sketch byte cap: a desktop screen of the 16 Oct layout measured 2560 bytes.

## Autopilot starts Sessions, not Jobs

`docs/concepts/autopilot.md`, `.claude/decisions/2026-10-15-sessions-absorb-jobs.md`. A pick starts a Session briefed on its issue, and the figures count Sessions. **One field is renamed and none is added.** The protocol ID moves with `crates/ipc/src/autopilot.rs`, and there is no number to take. The migration `20261017T0900Z-autopilot.started_session` has a name and not a number, and is additive.

| Where | Side | Change |
| --- | --- | --- |
| `AutopilotPick.job_id` | Fleet writes, Bridge reads | Replaced by `session_id`: the Session the pick started, opened in the Cockpit. A pick of an earlier build carries none |
| `Autopilot.at_work`, `spent_micros` | Fleet writes, Bridge draws | Count the Sessions it started: live, at work, and not waiting on a person, and their reported cost. Same fields, same types |
| `Autopilot.jobs_started`, `job_cap`, `AutopilotCap` `jobs` | Fleet writes, Bridge draws | Renamed in *Sessions absorb Jobs, group 2*, below |
| `AutopilotPickState` `withdrawn` | Fleet writes | A Session closed at Stop because it had never run, where it was a Job killed |
| `dispatch_set_aside` | Bridge calls | Starts a Session at once, where it drafted a Job to the approval gate. The route and the answer are unchanged |

**Skew.** A Bridge before this reads a pick with no `job_id` and cannot open it; the IDs differ, so the two are refused regardless. **The event stream is untouched** beyond the `autopilot.changed` each Autopilot already publishes. The lifeboat is untouched.

## Sessions absorb Jobs, group 1: the Job arms and piloting a Job leave the wire

Since 17 Oct 2026. `.claude/decisions/2026-10-15-sessions-absorb-jobs.md`, the retired Pilot concept and `docs/concepts/session.md`. **Removal only**: no field, operation or event kind is added, so every change below is a Bridge or Fleet that still sends or reads it. The protocol ID moves with the hash of `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and a pair that disagrees is refused. Where a wire type had a Job arm beside a Session arm, the Job arm is gone and Fleet stops producing the value.

| Where | Side | Change |
| --- | --- | --- |
| `take_over` (`POST /jobs/:job_id/take_over`), `get_handoff` (`GET /jobs/:job_id/handoff`) | Fleet removes, Bridge stops calling | Operations, routes, MCP tools and the DTOs `TakeOver`, `PilotOutcome`, `HandoffBundle`, `HandoffWorktree`, `StoppedOn`, `DroneNarrative` |
| `StartSession.pilot` (`PilotFrom`) | Bridge stops sending, Fleet stops reading | The Session started on a Job's worktree, with the Job's slot and branch handed to it |
| `SessionRow` kind `handoff` (`HandoffStep`, `HandoffPlan`) | Fleet stops writing, Bridge stops drawing | The first row of a piloted Session's thread |
| `TimelineKind` `job.created`, `job.queued`, `job.started`, `job.step`, `job.check`, `job.judged`, `job.files`, `job.read`, `job.ran`, `job.asked`, `job.answered`, `job.landed`, `job.failed` | Fleet stops writing, Bridge stops drawing | Fleet appends no entry for a Job. `slot.leased` and `slot.released` are a Session's alone |
| `TimelineSubject` `job` | Fleet stops writing, Bridge stops drawing | The subject is `session`, `pull_request` or `main` |
| `MergeLineHub.fixing`, `HubPullRequest.job`, `HubMerged.job`, `MainMerge.job`, `HubJob` | Fleet removes, Bridge stops reading | `MergeLineHub.fixing_session` and `MainMerge.session` are the owners |
| `HolderKind` `job` (`FixingElsewhere.holder`, `ClaimHolder`, `Holder`, `Ownership`) | Fleet writes `session` only, Bridge reads `session` only | A Job's claim in the central store is not drawn |
| `ClaimPullRequest.job_id`, `PullRequestClaimed.holder_kind` | Bridge stops sending, Fleet removes | `claim_pull_request` is a Session's; `holder_id` is its id |
| `ThoughtKind` `job`, `ThoughtTarget.kind` `job` (`read_thought`, `record_thought_sent`) | Fleet stops offering, Bridge stops sending | A thought routes to a Session or to none |
| `TagKind` `job`, `Mention` Job arm, `SessionAttachment.job` (`TaggedJob`) | Bridge stops sending, Fleet stops reading | The composer offers no Job after `@` |
| `SessionSummary.jobs` | Fleet removes | The Jobs a Session holds that have not finished |
| `who_owns`, `armada need`'s `mine` and `ahead` | Fleet writes Session holders only | A Job holder is omitted, not renamed |

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves. Nothing here needs a reader that tolerates the old shape.

**Left for group 2.** What this group left is removed in *Sessions absorb Jobs, group 2*, below.

**The event stream.** This change makes the open risk below no better and no worse in kind: it removes the Job producers of timeline entries, so the stream carries fewer events. It adds no bounded broadcast and no "you missed N events" message, so a minimised Bridge against several Drones still has nothing pushing back.

## Sessions absorb Jobs, group 2: the Job surface leaves the wire

Since 17 Oct 2026. `.claude/decisions/2026-10-15-sessions-absorb-jobs.md`. **One rename, and removal otherwise**: no operation, event kind or route is added. The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and a pair that disagrees is refused. `crates/ipc` and `packages/protocol` are edited together.

**The rename.** Autopilot's figures count Sessions and now say so. The store keeps the settings key it had; Fleet maps it.

| Where | Side | Change |
| --- | --- | --- |
| `Autopilot.job_cap` | Fleet writes, Bridge draws | `session_cap` |
| `Autopilot.jobs_started` | Fleet writes, Bridge draws | `sessions_started` |
| `AutopilotSettings.job_cap` | Bridge sends, Fleet reads | `session_cap` |
| `AutopilotProposal.job_cap` | Fleet writes, Bridge draws | `session_cap` |
| `AutopilotCap` `jobs` | Fleet writes, Bridge draws | `sessions` |

**Operations removed (44).** Each has no route, no MCP tool and no Bridge caller:

| Kind | Operations |
| --- | --- |
| Queries | `get_job`, `get_job_events`, `get_job_log`, `get_job_retro`, `list_jobs`, `list_job_board`, `get_diff`, `get_usage`, `get_manifest_spend`, `get_activity_feed`, `list_alerts`, `list_branches`, `list_reports`, `list_reviews`, `observe_job_log` |
| Commands | `kill_job`, `forget_job`, `file_report`, `delete_branch`, `reclaim_worktree` |
| Events | `job.created`, `job.state_changed`, `job.step_advanced`, `job.files_changed`, `job.judging`, `job.checking`, `job.dry_run`, `job.asking`, `job.command_waiting`, `job.forgotten`, `job.landed`, `job.remarks_changed`, `job.paused`, `job.resumed`, `job.plan_changed`, `job.trigger_changed`, `job.addition_changed`, `drone.spawned`, `drone.exited`, `evidence.submitted`, `proposal.moved`, `run.finished`, `commander.changed`, `commander.card` |

**Wire types removed.** The DTO files of the Job: `job`, `detail`, `work`, `work_plan`, `drones`, `history`, `journal`, `judged`, `asked`, `asking`, `approving`, `approval_ask`, `proposing`, `waiting`, `turn`, `attempt`, `added_steps`, `walk_notes`, `trigger_holds`, `piloting`, `usage`, `resources`, `report`, `reclaimed`, `breakage`, `alerts`, `capacity`, `commander`, `commander_card`, `commander_repair`, `job_issues`, `manifest_checks`, `overlap`, `raising`, `remarks`, `showing`, `explaining`; the MCP tool modules under `ipc::mcp`; and the Job vocabularies in `ipc::enums` (`JobStatus`, `StepState`, `QueuedReason`, `Resumption`, `Recourse`, `DronePresence`, `BudgetHold`, `Urgency`, `Side`, `Origin`, `TopLevelOrigin`, `TaskTier`, `JudgeVerdict`, `CriterionSource`, `DependencyDirection`, `TaskState`, `GroupState`), `JobId` and `DroneId`.

| Where | Side | Change |
| --- | --- | --- |
| `Resync.jobs` | Fleet removes, Bridge stops reading | A resync carries the ID and the cursor, and each surface re-reads its own state |
| `Event::about` | Fleet writes | `Option<String>`: an event that names no subject says so |
| `BayHolderKind` `job`, `ClashSide` `job`, `SlotHolderKind` `job`, `SlotHolding::Job` | Fleet writes the other arms only, Bridge reads them | A Job holder is not drawn. A lease record written before still reads as a holder id |
| `SessionAttachment` kind `job` | Bridge stops sending, Fleet stops reading | An old attachment is skipped |
| `ServerState.job_id`, `ServerState.for_review`, `StartServer.job_id`, `ServerOpened.job_id`, `StartedBy` `drone` | Fleet removes, Bridge stops reading | A server belongs to a checkout or a Session |
| `Requester` `job_id`, `step`, `task_id`, `drone_id`, `handle` and the `gate` and `drone` kinds | Fleet writes `merge_line`, `session`, `outside` | A record written before still reads as its `kind` |
| `WireError.job_id`, `drone_id`, `step_id` | Fleet removes, Bridge stops reading | A fault names no Job. The three were optional and skipped when empty |
| `TrackedIssue.job_id`, `ReviewPullRequest`, `ReviewDispatched` | Fleet removes, Bridge stops reading | A review is a Session's |
| `FixesMain`, `FixesMainState`, `FixMain` | Fleet removes, Bridge stops reading | A fix is a Session the hub names by `fixing_session` |
| `JobRetro` | Fleet removes, Bridge stops reading | `Retro` (`get_session_retro`) replaces it, with the Job-shaped record rows (`RecordCheck`, `RecordNotMet`, `RecordAsked`, `RecordWaited`) gone |
| `Retro.session`, `Lesson.session` | Fleet writes, Bridge reads | `RetroSession`, no longer `Option`: every retro and lesson belongs to a Session |
| `RunSheet`, `RunRecord`, `RunUnderway` and the Job runs | Fleet removes, Bridge stops reading | `CheckoutRunRecord` and `observe_checkout_run` serve a checkout's runs |
| `ChecksUnderway`, `CheckUnderway` | Fleet removes, Bridge stops reading | `Session` runs have `observe_session_check` |
| `DeclaredJudge.gaming_check`, `gaming_patterns`; `CheckRun.group`, `group_attempt` | Fleet removes, Bridge stops reading | |
| `ManifestProposals.caps` (`StatedCaps`), `WorkflowStep.may_dispatch_jobs`, `WorkflowStep.drone_per_task` | Fleet removes, Bridge stops reading | A step no longer declares what a Drone may do |
| `StudioNodeContent` `job`, `DispatchStudioDraft` | Fleet stops writing, Bridge stops drawing | A Job node on an old Studio is dropped on read. The store's CHECK keeps the word |
| `HelmScreen` `board` | Bridge stops sending, Fleet stops reading | The Board is gone. A context naming it no longer decodes; `overview` is the screen that replaced it |
| `WorktreeHeld`, `HeldReason`, `WorktreesHeld`; `reclaim_worktree`, `delete_branch` | Fleet removes, Bridge stops calling | Slot cleanup is the pool's |
| `AlwaysAllowScope`, `AnswerCommand`, `CommandAnswer`, `CommandInFlight`, `Reach`, `RemoveAllowedCommand`, `SetWhenBlocked`, `WhenBlocked`, `SetModel`, `SetCommanderModel` | Fleet removes, Bridge stops sending | A repository's allowed commands are `get_repository_allowed_commands` and `remove_repository_allowed_command` |
| `ReclaimedSaved` | Fleet writes, Bridge reads | Moves from `reclaimed.rs` to `holding.rs`, shape unchanged; `SlotReleased.saved` names it |
| `WalkTold` | Fleet writes, Bridge reads | Moves from `showing.rs` to `sessions.rs`, shape unchanged; `window_failed` answers with it |

**Left, because a surface other than a Job reads it.** The store's Job tables stay in the schema and nothing reads or writes them; no migration drops them. A `holder job <id>` already in a lease record still reads, so no slot is stranded by one. `docs/capabilities/drone-per-step.md (deleted)` describes what is gone and is for the docs pass.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves. No reader needs to tolerate the old shape. The lifeboat's `GET /v0/jobs` and `POST /v0/jobs/:id/kill` rows name Jobs and the lifeboat is not built; this change does not touch them.

**The event stream.** This change makes the open risk below **better in kind**: it removes the producers of `job.*`, `drone.*`, `commander.*`, `evidence.submitted`, `proposal.moved` and `run.finished`, the busiest kinds Fleet published, so the stream carries far fewer events and a minimised Bridge has less to fall behind on. It adds no bounded broadcast and no "you missed N events" message; `Missed` and the drop-oldest channel are as they were, and the number is still unmeasured.

## A Job's Loadout

Since 17 Oct 2026. What a Job's agent knows, was told, and used each turn, each item with where it came from. **Additive only**: two facts, one optional field on `SessionRecord`, two operations and the types they carry. No event kind or error is new. Bridge's half is `packages/protocol/src/loadout.ts` and `sessions.ts`, written by hand. The migration `20261017T1700Z-session_loadout.kept` is additive. **Names, sources and counts only** on the facts, the record and `get_session_loadout`: never a skill's body, a prompt's text, a tool's input or a file's contents. What one row says is `read_loadout_item`, read off disk where Fleet runs when a person opens that row.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionFact` `equipped` | The mod writes | `loadout`: `LoadoutItems` `{ items }`, the whole list, replacing the last. `LoadoutItem`: `kind`, `name`, `source`, `from?`, `state?`, `tokens?` |
| `SessionFact` `used` | The mod writes | `use`: `LoadoutUse`: `turn_started_at`, `kind` (`skill`, `tool` or `subagent`), `name`, `source`, `from?`, `model?`, `by?` |
| `SessionRecord.loadout_summary?` (`session.changed`, `get_session`, `list_sessions`) | Fleet writes, Bridge draws | `LoadoutSummary`: `knows`, `told`, `latest_turn?` (`started_at`, `used[]`). Absent where no `equipped` was ever reported |
| `get_session_loadout` (`GET /sessions/loadout?session_id=`) | Bridge reads | `SessionLoadout`: `items[]`, empty where none was reported, and `turns[]`, oldest first, at most fifty. 422 `fleet.session_unnamed` for a job Fleet does not know |
| `read_loadout_item` (`GET /sessions/loadout/item?session_id=&kind=&name=&source=&from=`) | Bridge reads | `LoadoutItemBody`, tagged on `what`: `read` (`path?`, `format` `markdown` or `text`, `body`, `truncated?`) or `not_read` (`why`). The query is `LoadoutItemAsked`; a use's kind is asked as the kind of that name. 422 `fleet.session_unnamed` for a job Fleet does not know |

**Every closed set is a plain enum Bridge matches on**: `LoadoutSource` (`kit`, `armada`, `repository`, `plugin`, `mcp`, `built_in`, `managed`), `LoadoutKind` (`skill`, `subagent`, `mcp_server`, `tool`, `memory_file` are known; `output_style`, `rules`, `injection` are told), `LoadoutState` and `LoadoutUseKind`. A value outside them refuses the fact.

**A turn is the instant the mod says it began**, and turns are ordered by when Fleet first heard of each. **A use repeated in its turn is kept once and counted, and publishes nothing by itself**, and an `equipped` equal to the one held publishes nothing, so a turn of a hundred MCP calls to one tool is one row and one `session.changed`. `LoadoutUse.count` is **Fleet's field**: the times it heard that use in its turn, left out when 1, and a `count` a mod sends is ignored. The store keeps it in `session_loadout_uses.count` (`20261018T1000Z-session_loadout.use_count`, additive). A count that rose rides the next record Fleet publishes anyway: a measurement that moved a figure, or the turn's end, which always publishes. There is no timer, so a Bridge shows the count as of the last of those and the final number at the turn's end.

**The event stream.** Each new list and each new distinct use publishes one `session.changed`, through the shared drop-oldest backlog. It adds no event kind and no queue, and the dedupe keeps it to what changed, and a repeat's count adds no event, so it leaves the unmeasured sink risk where it was. A Bridge that missed one reads the record again on the resync that follows a `Missed`.

**What a row reads as.** Only a row the job's Loadout lists, as an item or a use; any other reads `not_read`. Which file is `HarnessSetup::read_item`, so the harness's paths stay in `crates/adapters`:

| Kind | Read |
| --- | --- |
| `skill` | `SKILL.md` of `.claude/skills/<name>/` in the job's directory, `~/.claude/skills/<name>/` (or a folder up to three down whose name or front matter says it, where a synced set lands), or the plugin's `skills/<name>/`. A plugin's `<plugin>:<name>` is split on the colon, and `armada` is the mod's own plugin |
| `subagent` | `<name>.md` (or the file whose front matter names it) in `.claude/agents/`, `~/.claude/agents/` or the plugin's `agents/` |
| `output_style` | The same, in `output-styles/`. A built-in style is `not_read` |
| `memory_file` | The file at the path the mod reported: `~/.claude/…`, relative to the job's directory, or absolute under one of those two |
| `rules` | Armada's compiled job rules, `text`, no `path` |
| `injection` | The `armadaSession` note the mod adds, with the id as `<job id>`, `text`, no `path`. Kept by hand beside `idNote` in `plugins/armada/hooks/register.ts`, and a test fails when they differ |
| `mcp_server`, `tool`, and a `built_in`, `managed` or `mcp` source | `not_read` with why |

A plugin's folder is its `installPath` in the harness's record of installed plugins. **A name that could climb is refused before anything opens**: `..`, `.`, a separator or a NUL in a skill's, an agent's, a style's or a plugin's name, `@` in a plugin's, and a memory path with a `..` part or absolute outside the job's directory and `~/.claude`. **The file is resolved through every link and must still be under its folder or the person's home**, as Kit follows a dotfiles link, so a link to anywhere else reads nothing. The body stops at `LOADOUT_ITEM_CAP`, 256 KiB, on a whole character, with `truncated`. `path` is from the job's directory, `~/…` under the home, or from the plugin's folder.

**Skew.** A Bridge that predates this ignores a `loadout_summary` it does not read. A Fleet that predates it refuses both facts, and the mod goes on. `read_loadout_item` is a new route, so the protocol ID moves with it and a Bridge and a Fleet that differ on it never meet.

## Needs are deleted

Since 17 Oct 2026. `.claude/decisions/2026-10-15-sessions-absorb-jobs.md`, *Needs are deleted, 17 Oct 2026*. **Removal only**: no operation, event kind, route or field is added. The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and a pair that disagrees is refused. There is no number to take.

**Operations removed (2).** Each has no route, no MCP tool and no Bridge caller; `armada need` was the only caller of the first and an agent's `list_needs` the only reader of the second.

| Operation | Route |
| --- | --- |
| `act_on_need` | `POST /needs` |
| `list_needs` | `GET /needs?manifest_id=` |

`GET /clashes`, `GET /repository_areas` and `GET /slot_touches` stay and are served as before. Their handlers moved from the api's `needs` module to `clashing.rs`, and the trait Fleet implements for them is `Clashing` where it was `Needs`. Neither is wire.

| Where | Side | Change |
| --- | --- | --- |
| `NeedAct`, `NeedCall`, `NeedAnswer`, `NeedLine`, `NeedList` | Fleet removes, Bridge stops reading | `ipc::needs` and its mirror in `packages/protocol` are gone. Bridge never read them |
| `fleet.need_unnamed`, `fleet.need_not_declared` | Fleet stops raising | Both were 422s of `act_on_need` |
| `ChosenHold.kind` and `HoldMoved.kind` `need` | Fleet stops reading and writing, Bridge stops sending | `release_session_holds` acts on `slot`, `branch` and `pr`. A standing `need` row in a store is no longer acted on, so naming one is `fleet.hold_not_standing` |
| `SessionAttachment` kind `need` | Fleet stops writing, Bridge stops reading | A row an earlier build wrote still reads and is passed over. The mod no longer reports one |
| `NotMerged` kind `waiting_behind` | Fleet stops producing | No build since the Job code went produced it. `from_kind` reads a stored one as `refused` |

**Left, because a surface other than a need reads it.** The `ledger_attachments` rows of kind `need` an earlier build wrote stay in the store, unread; no migration drops them, which the Job tables' one later migration will. `Store::standing_in_order` stays: pull request owners read it.

**Left, and not this:** Fleet's single re-run of a hung CI job (`rerun_failed` in `pull_notices.rs`), the Helm `rerun_failed_checks` act, and write-clash detection (`list_write_clashes`), which is what tells two agents they are changing the same file.

**The `needs` commit status** Fleet put on each open pull request, and the forge write behind it (`Delivery::publish_status`, `CommitStatus`, `StatusState`), are gone. It was never required in the ruleset, and `checks.yml` never reported a context of that name.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves. No reader needs to tolerate the old shape. The lifeboat is not touched.

**The event stream.** Neither better nor worse in kind: no event is added or removed. A need's declaration used to move a Job's row, so Fleet publishes fewer `session.changed` events by those, a handful an agent. The bounded broadcast and the "you missed N events" message are still not built and the number is still unmeasured.

## A write clash's file is read as two patches

Since 17 Oct 2026. `docs/concepts/fleet.md`, *Write clashes*. **Additive only**: one operation, one DTO and one route, and no field of an existing type changes. The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `packages/protocol/src`, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `read_clash_file` (`GET /clashes/file?session_id=&other_kind=&other_id=&path=`, `Bridge only`) | Bridge asks, Fleet answers | `ClashFile`: `ours` and `theirs`, each the unified diff `git diff` prints for that one path from the merge base of the two sides, empty where a side left the file alone. `other_kind` is a `ClashSide`, `session` or `main`; `other_id` is the other Job's id and `main` ignores it |
| `fleet.clash_not_standing` | Fleet refuses | A 409: the pair no longer clashes, or either has left the table |
| `fleet.clash_path_not_in_clash` | Fleet refuses | A 422: `path` is not one of the clash's `paths` |
| `fleet.clash_unreadable` | Fleet refuses | A 500: git would not answer |
| `ClashFile` in `packages/protocol/src/clashes.ts` | Bridge reads | The hand-written mirror. Bridge reads the operation through one preload entry, `readClashFile`, which names no Manifest: Fleet finds the clash by the Job (`apps/desktop/src/main/clashes.ts`) |

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves. Were a pair to meet anyway, a Fleet without the route answers 404, which no refusal here is, and Bridge words it as "This Fleet cannot show the files of a clash." and draws no diff.

**The event stream.** Untouched: a request-response read, no event kind, and nothing published. The open risk below is neither better nor worse.

## Usage a turn at a time

Since 17 Oct 2026. What each of a Job's turns cost, so usage can be read by job, model and tool over a window. **Additive only**: one fact, one operation and the types they carry; no event kind, no error and no field on `SessionRecord`. Bridge's half is `packages/protocol/src/turn-usage.ts` and `sessions.ts`, written by hand. The migration `20261017T1800Z-session_turn_usage.kept` is additive. **Names and counts only**: a tool is its name, never its input.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionFact` `turn_measured` | The mod writes | `turn`: `TurnUsage` `{ message_id, at, model, input_tokens, cache_creation_tokens, cache_read_tokens, output_tokens, tools? }` |
| `get_session_usage` (`GET /sessions/usage?session_id=&since=&until=`) | Bridge reads | `SessionTurnUsage`: `turns[]` from `since` (inclusive) to `until` (exclusive), oldest first. Either bound is optional RFC3339. 422 `fleet.session_unnamed` for a job Fleet does not know |

**A turn is one row per job and `message_id`.** A harness writes a message more than once as it streams, the output count growing, so a repeat replaces the row only where it carries more output than the one held. A repeat publishes no `session.changed`: nothing on a record shows usage.

**Nothing sends the fact yet.** The mod's `session.measure` is throttled and carries only the context figure, so it cannot be the source; a mod or a hosted job's stream has to report each answered message.

**Skew.** A Fleet that predates this refuses the fact and the mod goes on; a Bridge that predates it never asks.

## A question refused for a walk is kept whole

Since 17 Oct 2026. `docs/concepts/session.md`, *Showing a window*. **Additive only**: one variant on `SessionAskState`, `kept`. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand.

| Where | Side | Change |
| --- | --- | --- |
| `SessionAskState` `kept` (`session.row`) | Fleet writes, Bridge reads | An `AskUserQuestion` refused because its job's walk is open. The row carries the whole question as any ask row does, and Bridge draws it as the question that lapsed, answerable in the Needs you list. Unlike `unanswered` and `refused`, a message from the person does not clear it unless the message answers it |

A Bridge before this reads an ask state it has no word for and draws the row as a settled tool line.

## Sessions are called Jobs: four tools listed under new names

17 Oct 2026, the owner's rename (`.claude/decisions/2026-10-15-sessions-absorb-jobs.md`, *Sessions are called Jobs*). **The wire keeps `session`.** Field names, `session_id`, event kinds, routes, operation keys and error codes are unchanged, so no Bridge field moves and no migration is owed. What moves is the text an agent reads.

| Tool an agent lists | Operation it answers for | Still answered by |
|---|---|---|
| `start_another_job` | `start_another_session` | `start_another_session` |
| `fork_this_job` | `fork_this_session` | `fork_this_session` |
| `list_jobs` | `list_sessions` | `list_sessions` |
| `move_job` | `move_session` | `move_session` |

**Aliases live in the door, not in the inventory.** `ipc::door::RENAMED` pairs each Job name with the operation, which keeps its Session name as its key in `operations/`, its route and its daemon method. `tools/list` names the Job name once; `tools/call` takes either, so a running agent that listed its tools before the rename and a mod that names the old one keep working. Permission rules, Helm's reach and `QUERIES` read the operation and need no second row.

**Descriptions say Job.** Every `description` in `operations/` and the door's parameter descriptions read Job where they read Session. Notes that discuss both the old Job and the Session were left as written, since renaming one word there would merge two concepts. `docs/concepts/session.md` is still the concept's file.

**Skew.** The protocol ID moves with `ipc/src/door.rs` and the descriptions, as it does for any wire file. There is no number to take.

## A pinned note, sent whole: `send_session_note`

Since 17 Oct 2026. `docs/concepts/session.md`. **Additive only**: one operation, one route and the types they carry; no event kind, no error and no field on an existing type. The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `packages/protocol/src`, and there is no number to take. Bridge's half is `packages/protocol/src/session-notes.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `send_session_note` (`POST /sessions/note`, `agent_access = "No"`) | Bridge sends, Fleet answers 202 | `SendSessionNote` `{ session_id, said, address, note_id?, place, state, crop? }`; answered with `NoteSent` `{ session, candidates }` |
| `NotePlace` | Bridge sends | `component?`, `owners?`, `selector`, `element` (`NoteElement` `{ tag, text, label? }`), `nearby?`, `stamp?`, `location`, `screen?`, `layer?` |
| `NoteState` | Bridge sends | `selected?`, `walk?` (`NoteWalk` `{ name, step, of }`), `take?` (`NoteTake` `{ letter, name }`), `scenario?` |
| `NoteCrop` | Bridge sends | `{ media_type, data }`, `data` base64 |
| `Candidate` | Fleet answers | `{ file, line, why, text }`, at most five, best first; `file` is relative to the checkout Fleet searched, `line` is 1-based |

**Bridge's call** is `sendSessionNote` on `window.armada` (channel `bridge:send-session-note`, `SessionsHost.sendNote` in main). It folds `NoteSent.session` into the held Sessions as a message's answer is, and hands the candidates back to the caller. An empty array on the wire is the same as an omitted one, so the mirror marks each as optional.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves. Were a pair to meet anyway, a Fleet without the route answers 404, which Bridge words as a send that failed.

**The event stream.** Untouched: the turn it takes publishes `session.changed` as a message does, and nothing new. The open risk below is neither better nor worse.

## Open questions

Naming these rather than deciding them, per this document's brief:

- **[protocol-codegen]** What generates the TypeScript from `ipc`. Hand-rolled build script,
  `ts-rs`, `specta`, something else — not decided. Whatever it is, it must not
  reach `core-model` or `adapter-traits` (their `cargo tree` is a gate rule:
  no codegen framework belongs under either).
- **[verify-protocol-task]** What checks the rest of the generated half. The
  protocol ID is held by an `xtask` rule that hashes the wire both ways; the
  DTO types are generated by nothing, so there is no candidate output to
  compare the checked-in ones against. Whether the remainder
  is a rule in `verify-foundations` or a `verify-protocol` task of its own
  follows from what `[protocol-codegen]` decides, and neither is decided.
- **[broadcast-capacity]** The bounded broadcast channel's capacity, and whether it's one number
  for all event types or tuned per event type.
- **[lifeboat-router]** Whether the lifeboat's four routes live inside the
  same `axum` `Router` as the main protocol or a separate one. Either can satisfy "no shared
  dependency with the versioned protocol"; which one hasn't been decided.
