# What does Overview show when the plan is what waits on a person?

**Decided 2026-09-30.**

On a Job whose plan step waits on a person (`arc/plan-review`), Overview drew the ordinary work review: a "What should change" box with Approve the work, Request changes and Reject the work. It said "Plan the change is waiting on you" and showed no plan and no way to it. The owner: *"Instead its asking me to provide notes and approve without even knowing what I am looking at."*

**Chosen:** Overview shows the plan and its gate: the plan's groups and tasks, with Approve the plan and Propose a change, on Overview itself.

**Cost he took, stated in the question:** a second copy of Plan's review on Overview, and every review act has to stay the same in both places. That means building it from Plan's own pieces rather than beside them.

**Then, the same evening:** *"When showing the plan on the overview, just like the workflow we should try to reuse the graph/list components from the plan tab."* The plan at the gate is Plan's own Graph and List, not a new drawing. The gate selects it when the waiting step's `evidence_type` is `plan`, the discriminator already on the wire, which the Overview session is building the gate's other shapes against (#1680).

**The fold changes at a plan gate.** Since 29 Sep the gate's record sits folded under Overview's lead (`2026-09-29-the-review-gate-sits-under-the-lead.md`). At a gate whose waiting step claimed a plan, the plan replaces that folded record: Plan's review is drawn open, in its place, with its Graph and List and its two acts. A folded plan would be the annotation's problem again, a decision asked for with the thing being decided shut. Every other gate keeps the folded record as it was. (Since 2 Oct 2026 that record is open, not folded — the owner's reversal, recorded in the 29 Sep decision.)

**Where it landed:** `packages/screens/src/plan-review.tsx`, one region the Plan tab and the gate both mount. `verdictSlotAtGate` in `verdict.tsx` draws it when `claimed.evidence_type` is `plan`; no claim, or any other claim, still draws the work review. The Graph/List choice is one per viewer, shared with the Plan tab.
