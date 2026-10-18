# Protocol 21.11: a kept brief is read back

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

One route and one DTO: `GET /jobs/:job_id/briefs/:name` (`get_brief`) answers `BriefContents`,
which is `path`, `lines`, `from_line`, `total_lines`, `bytes` and `whole`. Additive. It is what Pulse's log
panel reads when a person presses a brief row, so a Judge's or a gaming check's brief opens inside
Bridge the way a transcript does (the owner's decision, 2 Oct 2026).

**`:name` is the last part of a `brief_path`**, the same file `JobResources.logs` lists as kind
`brief`. Fleet resolves it inside that Job's briefs directory. A name that would leave it (`..`, a
separator, another Job's brief, a link out) gets the 422 a name it never kept gets. A missing Job
is a 404.

**The window is `get_check_output`'s**: the tail, 2,000 lines and 256 KiB. A brief ends on what
it asks, after the diff, so a cut brief keeps its question. `whole` is stated, never inferred.
