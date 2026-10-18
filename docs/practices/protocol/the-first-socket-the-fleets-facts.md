# The first socket: the fleet's facts

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

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

## A kind exists when something produces it

**An event kind that never fires reads as a stream that is working**, so a kind
`crates/ipc/operations/` names is not stubbed until a record exists for it
to carry. The kinds still waiting describe records this workspace has no type
for.

## A row appearing is a kind, not a state change

When a record is created while a client is connected, the event carries the
whole row and no `from`. A `StateChanged` with a `from` the edge table has no
row for would tell every client folding the stream to apply a move that did not
happen. **A creation is a row appearing, not a row moving**, and a deletion is
the opposite message with the opposite payload: only the id, because by the time
it is published there is no row left to carry.

## A fact about the fleet is a reading, not only an event

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
