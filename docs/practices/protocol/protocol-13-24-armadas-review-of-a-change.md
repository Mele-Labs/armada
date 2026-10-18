# Protocol 13.24: Armada's review of a change

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`ipc::JobConfidence`, additive on `get_job`'s `JobDetail` as `confidence`, is the review a person reads at the stop before merging (#903): whether Armada is confident and why, the change's areas, the tests in it, and its findings sorted into needs you, small fixes and for context. `tests.opened_because` names a test removed or loosened with no reason, and that row is `flagged`.

**Not `review`.** `JobDetail.review` is the text Fleet composes for the pull request, from 10.11. **Absent is a Job with no accepted review**, which is every Job whose steps ask for none.
