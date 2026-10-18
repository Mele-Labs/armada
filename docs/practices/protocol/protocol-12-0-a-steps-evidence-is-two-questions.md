# Protocol 12.0: a step's evidence is two questions

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#777`. `ipc::EvidenceType` loses `shown`, and removing a value from a set the
wire carries is a major bump by this document's own table — so the major moves
and the minor resets.

`shown` was never a claim a Drone handed in. Every other value names a work
product the gate measures against the step's own declaration; that one meant
*Fleet, run the repository's harness*, which is an instruction. Holding both in
one field is why a step could not hand in a patch **and** be captured, and a
WorkflowDef step now says the two separately: `evidence.submitted.type` is the
claim, `evidence.captured` is the instruction, and it gates nothing.

**No DTO gains or loses a field.** `ipc::Submitted.evidence_type` still carries
what a submission was recorded as, and that is still the workflow's word rather
than the Drone's. Being captured is a fact about the frozen step and not about a
submission, so nothing on this seam had to carry it — what a capture produced
already reaches Bridge as `frames` on the step, unchanged. The break is the
narrower set alone, which is why the refusal is worth the major: a Bridge built
before this looks `shown` up in the generated vocabulary and finds a word Fleet
can no longer send.
