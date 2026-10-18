# Protocol 13.25: the code a review is about, as a View

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`ipc::ViewStepRow`, additive on `JobConfidence` as `view` on an area and on a finding (#904): the files it is about, as steps in the order one change forces the next. Each step names its hunk by the patch's `@@` header, with a one-sentence summary and, except on the last, what ties it to the next.

**The hunk is named, never copied.** Bridge finds it in the patch `get_job_diff` serves, so a View cannot show code the branch no longer holds. **Left out where empty**, which is every area and finding the reviewer gave no View.
