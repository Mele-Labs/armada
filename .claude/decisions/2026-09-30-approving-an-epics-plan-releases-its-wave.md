# What does approving an Epic Job's plan approve?

**Decided 2026-09-30.**

When an Epic's plan for its next wave waited on a person, the gate could not show the split being approved: the proposed Jobs did not exist until dispatch. That was the owner's own note again (`m6t1`, *"approve without even knowing what I am looking at"*). The Overview session pointed out that the right state already exists. `awaiting_approval` means "a person must approve the dispatch before anything runs", and `propose_from_request` already creates several Jobs at once at that gate. `proposing` (#1690) was the wrong fit, because it means a model call is out and spending money.

**Chosen:** the proposed Jobs exist as real Jobs at `awaiting_approval`, each dispatched by the Epic. The gate draws them, and each opens the same Job panel, where it can be dropped or edited. **One act, Approve the plan, releases every one of them.**

**Editing one happens in its panel.** A proposed Job's panel carries **Edit this Job**, before Open job and Hold to drop, and it edits the Job directly through Fleet without leaving the wave. Open job is not the way to edit: the Job's own approval screen also offers approving that one Job alone, which cuts across one act releasing the wave. Fleet has no route yet (#1699), so Save raises `Not implemented` naming it.

**Cost he took:** a new Fleet route for the batch approve (#1694), and one press starts several Jobs.

**Where it landed:** `plan/epic-wave`, in the mock, with #1694 and #1699.
