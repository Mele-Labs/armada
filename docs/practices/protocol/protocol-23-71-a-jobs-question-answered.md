# Protocol 23.71: a Job's question answered

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *A job Fleet hosts*. **Additive only**: optional fields on two existing DTOs.

| Where | Carries | Notes |
| --- | --- | --- |
| `HelmCallInFlight.questions` (`hosted.asked`, `session.row` ask) | `AskedQuestion`: `question`, `header`, `multi_select`, `options` (`label`, `description`, and `preview` where the agent sent one) | Present where the call is the agent's `AskUserQuestion`; absent for every other call. `offers` is `allow_once`, `refuse` and nothing else |
| `answer_session_ask` (`POST /sessions/ask/answer`) | `answers?`: `QuestionAnswer` `question`, `chosen[]` | One entry per question. `allow_once` without every question answered is a 422 `fleet.session_answer_incomplete`. Free text is an entry in `chosen` |

Bridge's half is in `packages/protocol/src/helm-calls.ts` and `hosted-sessions.ts`, written by hand like the rest.
