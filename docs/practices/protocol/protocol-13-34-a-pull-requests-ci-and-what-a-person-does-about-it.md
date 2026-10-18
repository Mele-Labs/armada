# Protocol 13.34: a pull request's CI, and what a person does about it

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`PullRequestDetail.checks`, additive: what the forge's own CI came to on the pull request, as the sweep last read it, with the names of the checks that failed (#905).
Two commands take no body: `rerun_failed_checks` asks the forge to start the failed runs again, and `investigate_failed_checks` sends the Job back to the step before the one that delivers with the failed checks as the next Drone's note.
**Neither posts anything on the pull request.**
