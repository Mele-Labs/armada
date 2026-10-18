# Protocol 13.3: the moment a Drone submits

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#813`. `evidence.submitted` is a new event kind, so the minor moves. The row
had been declared since `522dac92` with no variant behind it, and the mark that
said so came off in the same change.

**It is a pointer and the decision was that it stays one.** The stream is one
bounded drop-oldest broadcast every Job shares, which is the argument that had
kept this unbuilt — and it defeats a payload-carrying event and nothing else.
So the message names the Job, the step and what the frozen step asked the work
product to be, and `get_evidence` serves the three sentences to whoever opens
the Job. `job.step_advanced` is the shape it follows.

**Published where the fact is, which is the Evidence call.** The gate notices on
its next turn; a message sent from there would be dated wrong and would say what
`job.checking` already says one message later.
