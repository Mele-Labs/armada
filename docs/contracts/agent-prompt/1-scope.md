# 1. Scope

**Part of:** [Agent Prompt Contract](../agent-prompt.md).

**Every prompt Armada assembles and puts in front of a model.** That is
the membership test, and it is deliberately different from the one on
[System Architecture](../system-architecture.md) section 9, which asks
whether an invocation carries a toolset. That is a taxonomy question with a
different answer: a clarification round is not a separate Agent, but it is
unambiguously a separate assembled prompt.

Every entry catalogued here passes this test — the invocations in section
2, plus the injected turns in section 4a.

**Not in scope:** what a model writes back — [Agent Copy
Contract](../agent-copy.md). The task text itself, which comes from the Job.
Model selection and budget, which are per-step configuration.
