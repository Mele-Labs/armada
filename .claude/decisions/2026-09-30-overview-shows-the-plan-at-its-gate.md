# What does Overview show when the plan is what waits on a person?

**Decided 2026-09-30.**

On a Job whose plan step waits on a person (`arc/plan-review`), Overview drew the ordinary work review: a "What should change" box with Approve the work, Request changes and Reject the work. It said "Plan the change is waiting on you" and showed no plan and no way to it. The owner: *"Instead its asking me to provide notes and approve without even knowing what I am looking at."*

**Chosen:** Overview shows the plan and its gate: the plan's groups and tasks, with Approve the plan and Propose a change, on Overview itself.

**Cost he took, stated in the question:** a second copy of Plan's review on Overview, and every review act has to stay the same in both places. That means building it from Plan's own pieces rather than beside them.

**Then, the same evening:** *"When showing the plan on the overview, just like the workflow we should try to reuse the graph/list components from the plan tab."* The plan at the gate is Plan's own Graph and List, not a new drawing. The gate selects it when the waiting step's `evidence_type` is `plan`, the discriminator already on the wire, which the Overview session is building the gate's other shapes against (#1680).

**Where it landed:** open, pending who builds it (Overview's screen).
