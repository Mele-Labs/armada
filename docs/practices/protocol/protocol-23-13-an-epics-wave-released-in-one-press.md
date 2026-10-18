# Protocol 23.13: an Epic's wave, released in one press

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Spike 022, slice 6 (#1767, carrying #1694, #1699's rest and #1692).

**A route, a body and optional fields, all additive.** An Epic's plan proposes its wave as real
Jobs at `awaiting_approval`, each `dispatched_by` the Epic, and one press releases them.
`JobSummary` gains `dispatched_pass`, the pass of the parent's plan that proposed it, counted from
one, and `merged_at`, when its pull request merged; `JobDelivery` gains `merged_at`; and
`LandingRule` gains `complete_when`. Each is absent where empty, so a 23.12 Bridge connects behind
the banner and reads a wave as Jobs at their gate.

`POST /jobs/:job_id/approve_wave` is retired; see *The wave routes: retired*.

**One act releases the wave.** Each member takes `awaiting_approval -> queued` as a person's act,
as `edit_job` left it, and the parent's plan step advances without the step after it being
entered: the parent waits on its members as it waited on a dispatch before. A member dropped
before the press (`kill_job`) is not in the wave. Three acts that would cut across the press are
refused with the same code: `approve_review` on a plan holding a wave, `approve_dispatch` on one
member, and the last approval of a parent with `complete_when: all_members_landed` while a member
has not merged.

**`all_members_landed` is honoured**, so the approval keeps it rather than refusing it. Under it
the parent's wait after its plan also counts a finished member whose pull request has not merged,
and its roll-up is written about what landed. **`merged_at`** is the forge's own instant, read on
the call that said it merged, or Fleet's clock where Fleet pressed the merge; absent beside a
`merged` noticed before 23.13.

**The shipped epic changes shape, and the parser holds it.** `may_dispatch_jobs` moves from
`dispatch` to `plan`, the step a person answers, and `dispatch` goes: a step that creates Jobs
must be `human_always` now, since what it creates waits for that gate. A plan that runs again
withdraws, as Fleet, what its last attempt proposed and nobody released. A Job frozen on the old
three-step epic runs as it was frozen.

**Store V101** keeps the pass on the Job as `DispatchOrigin`'s third column, when a pull request
merged, and `complete_when` with the landing. A member with a pass rebuilds at
`awaiting_approval`; a child with none is one made before V101, which entered `queued`.
