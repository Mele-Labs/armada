# Protocol 14.13: what is on a Studio becomes work

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

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
