# Protocol 13.33: a Job's review model

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`JobDetail.review_model_override` and `JobDetail.review_step`, additive, and the command `set_review_model`, which takes `set_model`'s body (#903).
A person chooses the model the step that writes Armada's review runs on, and on that step it beats `model_override`.
`review_step` is that step's label, **absent on a workflow with no review step**, which is where Bridge draws no review model at all.
