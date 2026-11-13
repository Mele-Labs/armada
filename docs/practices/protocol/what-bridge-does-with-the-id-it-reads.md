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
| The IDs differ | **Refuses.** Opens no socket. The notice says Fleet and Bridge do not match and offers Restart Fleet, or Reopen Armada where this Bridge is the stale side |
| The runtime file has no ID | **Refuses.** The same, and the notice says Fleet is out of date, since that is the one case where the side is known |

**The action on the notice is Restart Fleet, or Reopen Armada.** Which is decided by the kept build each side runs from: Fleet's runtime file carries it as `build`, and Bridge reads its own from its app folder.
A Fleet whose `build` is not the one `builds/current` names is restarted, a Bridge that is not on it is reopened, both stale or neither from a kept build is Restart Fleet.
With no Fleet running the notice offers Start Fleet instead (`armada fleet start`).
Restart Fleet is `launchctl kickstart
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
  seconds is the same code with `no_answer`. Start Fleet that fails is
  `bridge.fleet.start_failed`, with the command's own error, and Reopen Armada
  that fails is `bridge.app.reopen_failed`.

Bridge cannot say which side is stale from the IDs alone, so the sentence does
not guess, and the button is the one that names the side, where the kept builds
say. `packages/shell/src/fleet.ts` carries the sentences, and the detail
row names the first eight digits of each ID so a person can read them to
someone else.

**The refusal is not a fault of the Jobs.** Fleet is alive and dispatching, and
the screen says so. Bridge declined to read it rather than failed to, which is
why it draws as degraded and not red.
