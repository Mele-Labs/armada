# Protocol 18.2: a server can be started on a checkout that is not the main one

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1577`. One optional field added to `StartServer` — `checkout`, a path. Additive: a caller that
sends none gets the main checkout, which is what every caller got before.

**It changes which span `${port.NAME}` resolves against, and never the number.** The body still
names a server and never a port, which is the field it has always refused; what this adds is which
of a repository's checkouts the number is drawn for.

**Ignored where `job_id` names a Job**, rather than refused as a conflict. A Job's worktree is the
Job's, and the two fields naming the same thing twice is a caller confusing itself rather than a
disagreement Fleet has to arbitrate — the path route refuses a Job's worktree by name anyway, and
says to name the Job.
