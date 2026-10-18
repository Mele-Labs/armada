# Protocol 16.5: a proposed Check names the runner that drives it

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

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
