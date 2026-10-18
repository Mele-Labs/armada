# Protocol 13.36: what a review finding became

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`JobConfidence.followed`, additive: each For context finding a person turned into a Job queued behind this one, by its id, or into an issue, by its address (#906). Two commands: `queue_after_finding` takes the finding and proposes a Job created waiting on this one, and `file_finding_issue` takes the finding with the title and body a person confirmed and files it on the forge. **Neither posts anything on the pull request.**
