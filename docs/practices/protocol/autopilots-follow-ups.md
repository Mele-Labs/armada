# Autopilot's follow-ups

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#2105`. **Additive only**: one operation, one answer type, three fields. Bridge's half is `packages/protocol/src/autopilot.ts`, `judged.ts` and `detail.ts`, written by hand.

| Where | Carries | Notes |
| --- | --- | --- |
| `propose_autopilot` | answers `AutopilotProposal` | The `AutopilotSettings` fields flattened in, and `quoted`: per setting, keyed as the settings are, the words of the sentence it was read from. A client that read the answer as `AutopilotSettings` still does |
| `update_autopilot` (`POST /autopilots/:repository/settings`) | `AutopilotSettings` | Answers `Autopilot` in the state it was in: it never resumes one. 409 `fleet.autopilot_stopped`, 422 `fleet.autopilot_unsettable` |
| `resume_autopilot` | unchanged | 409 `fleet.autopilot_capped` while any cap is reached, naming it. A reached cap now **pauses** with `capped_by` set, where it ended the run; Resume clears `capped_by` |
| `StepDetail.sent_back` | `JudgesSentBack`: `times`, `of` | How many times the Judges' agreed refusal sent this pass of the step back to its Drone, of `AGREED_RETRIES`. Absent where none did |
| `Judged.risky` | a string | What this member said makes the refused work risky, which is what brings an agreed refusal to a person. Absent where it said only that the work needs redoing, or was not asked |

A Drone can ask several decisions in one ask, and a person answers them all at once.
**Additive**: the single question is unchanged, and `answer_question` keeps its route.
Bridge's half is `packages/protocol/src/waiting.ts (deleted)`, written by hand.
The ID moved with `ipc`, `operations/` and the mirror; no number was bumped.

| Where | Carries | Notes |
| --- | --- | --- |
| `ask_question` (Drone tool, not a route) | `decisions`, in place of `question` and `options` | Two to six, each `{question, options}` with a single question's option rules. Both forms at once, or one decision, is refused to the Drone by name. Any step may ask this way |
| `QuestionInFlight.decisions` (on `JobDetail.asking`, `job.asking`) | `DecisionInFlight`: `decision_id`, `question`, `options` | Absent is a single question. Where present, `question` is blank and `options` empty. Fleet mints `decision_id` as `d1`, `d2`... |
| `answer_question` (`POST /jobs/:job_id/answer_question`) | `ChosenAnswer.decisions`: `ChosenDecision`, `decision_id` and `chose` | One pick per decision, all in one call, with no `chose`. `ChosenAnswer.chose` is now optional: absent on an interview. 409 `fleet.illegal_move`, the reason in `message`, for a missing, unknown or doubled decision, a label the decision did not offer, a label sent for an interview, or picks sent for a single question. Nothing is kept on a refusal |
