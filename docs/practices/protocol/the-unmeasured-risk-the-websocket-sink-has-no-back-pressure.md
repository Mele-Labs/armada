# The unmeasured risk: the WebSocket sink has no back-pressure

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

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
