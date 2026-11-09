# What should each of Armada's workflows stop on?

**Said 2026-11-09.** He had approved the model from spike 43 ([`docs/spikes/043-why-the-rail-failed.md`](../../docs/spikes/043-why-the-rail-failed.md) on its branch): a Job's plan is its workflow, each plan group ends on a gate Fleet sees for itself, and Fleet keeps only the gates. A Job then took him through the nine workflows one at a time. Partway through he asked for dry runs: a moment from one of his own Jobs, and what he expects to happen next. This file holds his answers.

| Workflow | The moment | He said | What it became |
|---|---|---|---|
| feature | Proposed: Propose, Walk, Refine, Deliver | Changed Refine | Refine folds into delivery: no stop between the wiring and the merge, ci decides |
| feature | d9b709cf: he approved one of two walks | "Wire the approved part", then one stop per walk over a written instruction | One walk group per part he would see. Fleet ties a walk to its group, so one part's approval ends that part. Cost: a Fleet change |
| feature | Same Job: he changes the look after approving both walks | Just a new walk | A new walk for a part reopens that part's stop |
| every Job | After he approved a plan, the agent changes it | Only when it asks | A later version keeps his approval unless the agent passes `ask_again`. Cost: relies on the agent's judgement |
| feature | A test fails that is also red on main | "Fleet is supposed to detect it and start a job and let other jobs know or other jobs are supposed to be able to look up to see if anyone is fixing. This is also the job of the operator to pick up on" | The Job does not fix main. Whether Fleet starts that Job itself is checked separately |
| feature, bug | Delivering | "it's okay to get a draft PR going in parallel to running checks locally" | A draft pull request goes up while the checks run |
| every walk | | "Not every project will use walks so it should be optional" | Walk groups only where the project serves a walk and the change would be seen |
| bug | d7418609: plan approved, nothing visible | "Depends. At work I expect a fix, tests and draft PR so then we can potentially test in a test environment. For armada probably just fix, tests and merge" | The delivery he picks per project decides; the last group ends at the merge either way |
| bug | At work, the draft PR is in the test environment | When the PR merges | The Job stays until the merge, so what he finds comes back to it |
| design_plan | Proposed: Draft and Present as one group ending on his approval of the page | Took it | One group, `document_approved` |
| design_plan | 1150ad0d: he approved the page | "Depends. It should ask" | After the approval the agent asks: stop, start build Jobs, or build here |
| code_review | Proposed: one stop, the review as a page | "Honestly I use the review tool in armada instead of a code review job", then delete it | Deleted. No Job had ever run it |
| orchestrate | 2a9561f8: three of four child Jobs merged, the fourth waits on his walk | Nudge, then wait | It says once in its own thread which child waits on him, then waits. A new gate, `jobs_ended`, holds it until every Job it started has merged or ended |
| prototype | 7d2b2166: the Dolt spike, nothing to see in Bridge | A walk, then a page | Build ends on a walk of it running, Write up on the page |
| prototype | After the write-up | "Some prototypes I'm curious about and don't move anywhere. It really depends" | After the page the agent asks whether to move forward now or leave it as a write-up |
| refactor | Splitting a 900-line file, nothing visible | Plan, then the diff | Plan approved, then a page summarising the diff, then the merge |
| revert | Taking #3201 back out | "Absolutely depends on the project we're in", then ask me each time | The agent asks first whether he wants a plan; with a yes the plan group holds |
| uat | 3 of 20 scenarios blocked before the run | Run 17, ask about 3 | One stop, at the results page; the four stages are the agent's own tasks |

The workflow files in `.armada/workflows/` carry each answer in their header. The old rail's step placement, visits and step tools go in a later change.
