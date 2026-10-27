# What steps does a feature Job follow, and where do walks and tests fall?

**Decided 2026-10-27.** His words: *"I am running into inconsistency issues with when jobs show me walks vs plans vs tests."* He gave the order himself: Propose, Discuss the plan and scope with sketches and ideas, Approve the plan, Walk as soon as possible, Refine the implementation and tests, Delivery.

What he met: `feature` ran Plan, Implement, Land. The walk was named only in Land's `about`, so it came after the build and its tests, and Implement built *"the change and its tests"* against his mock-first rule. That rule lived in agents' memory and `session-habits`, never in the workflow, so each Job kept it or did not. Fleet observed the plan, edits, Checks, pushes and pull requests, and nothing of a sketch or a walk.

**Chosen: his six steps become `feature`, and Bug gains a short walk.** Sketched in the Job as *Feature Job: proposed steps*.

| Asked | He chose | The cost he took |
|---|---|---|
| Tests start before he has walked the mock | Fleet nudges the agent once; nothing blocks, as on 15 Oct | An agent can ignore the nudge |
| Whether a walk may come before the plan is approved | Whenever the idea is about the look, the agent may build a rough walk while discussing, and the plan then approves it | Mock work before the scope is agreed, some of it thrown away |
| Whether the agent waits after its first proposal | It stops until he answers | A Job sits idle while he is away |
| Whether a bug he would see gets the same treatment | Yes: cause, fix, a short walk straight to it, then tests and delivery; a bug he would not see skips the walk | One more stop on a visible bug |

**Decided for him, as plumbing.** Fleet learns two facts it can observe, a walk shown and a walk approved, so the rail can place the Walk step and the nudge can see a test edited before approval. A feature nobody would see passes over Walk, said in its plan.

## How he manages workflows in Bridge

His words: *"I have no good way right now to manage/iterate/update/add/modify workflows per repo or that are bundled in armada."*

What he met: System › Kit › Workflows was the Job-era creator. It edited the gate boxes, retries and passes, which Fleet no longer reads, and could not edit a step's `instructions`, the words the agent is told. There was no compare with Armada's, no reset and no delete. Armada's carried set is compiled from this repository's own `.armada/workflows/`.

| Asked | He chose | The cost he took |
|---|---|---|
| How he changes a workflow | A step's words are edited in place; a change of shape is typed to an agent, which rewrites the workflow and shows a diff he approves before it saves | Two ways in to build |
| How he changes one Armada ships | Armada's rows are read-only, with Make my copy (now, every repository on this Mac) and Propose to Armada (a Job on the Armada repository that lands it for every install) | A change for every install waits on a pull request |
| Whether a workflow's page shows where Jobs drifted | Yes: under each step, the recent Jobs Fleet nudged there, each opening its Job | Fleet keeps the nudges, and the page carries one more thing |

**Decided for him.** The page groups workflows as Armada, the repository and Yours, and each step says what ends it. The gate boxes, retries and passes leave the editor, since nothing reads them.

**Where it lands:** a mock walk first, on branch `armada/session-887f02f4`.
