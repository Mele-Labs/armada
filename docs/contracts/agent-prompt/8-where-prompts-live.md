# 8. Where prompts live

**Part of:** [Agent Prompt Contract](../agent-prompt.md).

**The invariant, which is decidable today:**

> **Every verdict must be reproducible from recorded state.** Whatever
> prompt produced a Judge refusal must be identifiable from the Evidence
> row, by version or by content hash. A prompt home that cannot satisfy
> this is not a candidate.

That eliminates at least one candidate before the question is answered —
an unversioned file under the Kit or Machine config directory — and it
applies to the baseline as much as to the Judge. Two Jobs run a week apart
against different baselines are not comparable, and nothing currently
would say so.

**What is unresolved.** `judge_check.prompt_key` is specified as a
reference into a prompt library and the library is sited nowhere. The
layer model implies homes for layers 1 to 4, and says nothing about where
a named Judge prompt is stored or how it is versioned. See Open questions
below.
