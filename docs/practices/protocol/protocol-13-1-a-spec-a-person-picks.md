# Protocol 13.1: a spec a person picks

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

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
