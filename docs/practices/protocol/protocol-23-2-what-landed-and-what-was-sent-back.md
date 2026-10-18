# Protocol 23.2: what landed, and what was sent back

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

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
