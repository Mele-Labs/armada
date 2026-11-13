# What Bridge does with the ID it reads

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

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
