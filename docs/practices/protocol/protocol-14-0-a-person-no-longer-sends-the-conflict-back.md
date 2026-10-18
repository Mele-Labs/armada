# Protocol 14.0: a person no longer sends the conflict back

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

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
