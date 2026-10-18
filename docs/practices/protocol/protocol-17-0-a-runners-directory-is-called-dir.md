# Protocol 17.0: a runner's directory is called `dir`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1456`. `ProposedRunner.pkg` becomes `dir`, and `{pkg}` becomes `{dir}` in
every runner description's templates.

**The rename is the whole of why this is major**, on `PlanTask.detail`'s
precedent one major back: the old name silently stops arriving, which no
version of "additive" covers. Nothing else about the message moved.

`pkg` said package, and the value is a directory — the one a runner is invoked
in, which is what makes `vitest` mean one suite rather than the three others in
the same repository. It came from an illustrative example rather than from a
decision, and reached a real key without anyone asking what it meant.

**Taken at the cheapest moment there was.** The field shipped in 16.5 and no
Bridge outside this repository had yet been built against it, so the refusal a
major causes falls on a rebuild that was already owed. The longer a name that
says the wrong thing survives, the more it costs to move.
