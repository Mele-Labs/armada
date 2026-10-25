# An adversarial review of a plan

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 25 Oct 2026. `docs/concepts/adversarial-review.md`.
**Additive only**: three operations, one field on a record, and the types they carry.
The protocol ID is derived from the wire files and needs no bump.

| Where | Carries | Notes |
| --- | --- | --- |
| `SessionPlan` | `adversarial?` | The latest `AdversarialReview` of this Job's plan, whatever version it read. Bridge marks it out of date when `subject.version` is not the plan's `version`. It rides every `session.changed`, so no event of its own |
| `AdversarialReview` | `subject`, `state`, `started_at`, `finished_at?`, `failed?`, `findings`, `cost_micros?` | `state` is `running`, `done` or `failed`. `failed` is Fleet's words and a failed review is never a result. `findings` empty on `done` is an answer: it read the plan and found nothing |
| `ReviewSubject` | `{ kind: "plan", session_id, version }` | The only kind so far. A pull request or a document is a new variant, and nothing else here changes |
| `AdversarialFinding` | `id`, `about`, `consequence`, `expected`, `found`, `cites?`, `settled?` | `cites` is `path` or `path:line`. `settled` is `{ how: "sent" or "dismissed", at }`, absent while the finding waits |
| `FindingAnchor` | `{ kind: "plan" }`, `{ kind: "group", id }`, `{ kind: "task", id }` | The ids are the plan's own group and task ids |
| `start_adversarial_review` (`POST /adversarial_review`) | `StartAdversarialReview`: `subject`, `again?`. Answers 200 `AdversarialReview` | Answers at once with the review `running`, or with the one kept for the subject when `again` is not set. 404 `fleet.review_subject_unknown` for no such Job or no plan. 409 `fleet.review_subject_stale` for a plan version that is not the current one. `agent_access: No` |
| `send_finding` (`POST /adversarial_review/send`) | `SendFinding`: `subject`, `finding_id`. Answers 200 `AdversarialReview` | The finding becomes the person's message to the agent, sent before it is settled. A finding already settled is answered as it stands. The same two refusals, and 404 `fleet.finding_unknown`. `agent_access: No` |
| `dismiss_finding` (`POST /adversarial_review/dismiss`) | `DismissFinding`: `subject`, `finding_id`. Answers 200 `AdversarialReview` | Settled; the agent is told nothing. The same refusals |

**The event stream.**
This adds no event kind.
A review starting, finishing and each finding settled is one more `session.changed`, so a review costs the stream a handful of frames, each carrying the record whole as every `session.changed` does.
That is a small load on the sink [that has back-pressure built and no measurement](the-unmeasured-risk-the-websocket-sink-has-no-back-pressure.md), and no change to what a reconnecting Bridge must resync.

**Skew.** A Fleet before this answers 404 for the three routes, and a Bridge before it ignores `adversarial`.
The IDs differ, so the two are refused regardless and the lifeboat serves.
Bridge's half is `packages/protocol/src/adversarial-review.ts` and `session-plan.ts`, written by hand like the rest.
