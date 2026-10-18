# Protocol 23.7: one Check's log on a merge line

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

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
