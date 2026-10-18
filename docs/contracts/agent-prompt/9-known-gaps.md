# 9. Known gaps

**Part of:** [Agent Prompt Contract](../agent-prompt.md).

**Raised by adversarial review, Aug 2026**, and not resolved here.

| Gap | Section |
| --- | --- |
| **Tool documentation.** How a Drone is shown its Commands and MCP tools, and with what descriptions. Commands, MCP and Sub agents are frozen for the Job and this contract names no wording for any of them. Tracked separately | 2, 3 |
| **No token budget** for the assembled prompt. Six layers plus a corpus, unbounded | 3 |
| **Wording for the injected turns.** Every turn Fleet authors and injects is drafted rather than sanctioned. The one entry with sanctioned wording, the refusal reprompt, is injected into nothing — it rides a brief | 4a |
| **The `Commit/PR message template` collision.** A Manifest template is layer-3 prompt content and it is a shape rule. Section 5 forbids shape rules in the baseline and says nothing about layers 2 to 6 | 3, 5 |
| **Prior-iteration context on loop workflows.** Evidence Scope appends it, which is a further source of prompt content — and for the Judge it sits oddly against "stateless, one-shot, no memory between calls" | 2, 3 |
| **Helm has no frozen/live story.** Jobs are persistent and per-Manifest; Voice and authority are configurable; Helm has no respawn path. What happens to a live job when the Kit or Machine config changes is unstated | 4 |
| **Criterion independence** — may a Judge grading criterion 2 see criterion 1's verdict? | 2 |
| **What a Job summary is and who writes it** | 2 |

---

## Open questions
- Where does the prompt library live, and is a prompt version recorded on the verdict?

Also bearing on this document, and written where each belongs: `[manifest-scanner-agent-or-model-call]` in `system-architecture.md`.
A question has one home — answering it in two places is how one of them goes stale.
