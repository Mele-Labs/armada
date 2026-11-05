# The Operator takes Helm's place

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Helm, the dock and its hosted Job, is gone from the wire, the store and the settings.
What it did that was work is a Job started on your words, and what it did that was routing is the Operator's.
This page lists what left the wire and each rename.
The older pages that name Helm are records of what was.

## Operations gone

`ask_helm`, `get_helm_session`, `start_helm_fresh`, `get_helm_debug_info` and `record_thought_sent`.

## Types gone

`AskHelm`, `Blank`, `Freshness`, `HelmAsked`, `HelmContext`, `HelmFresh`, `HelmScreen`, `HelmSession`, `HelmText` and `HelmUnanswered`.
`HelmDebugInfo`, `HelmDebugLine`, `HelmDebugSaid` and `HelmDebugText`.
`ThoughtSent`.
The mirror files `helm.ts` and `helm-debug.ts` in `packages/protocol/src` are gone with them.

## Gone with the dock

The dock was the only place a call from a process Fleet does not host could be answered, and it is gone.
A hosted Job's call is put to the person on the Job's own thread and answered with `answer_session_ask`.
A call from any other process is denied at once, so nothing waits for a person who cannot see it.

| Operations and events | Types | Routes | Error codes |
|---|---|---|---|
| `list_calls`, `answer_call`, `call.asking_to_run`, `call.answered` | `CallsWaiting`, `CallAskingToRun`, `CallAnswered`, `CallSettled`, `AnswerCall` | `/calls`, `/calls/answer` | `fleet.call_not_waiting`, `fleet.answer_not_offered` |

They were named `list_helm_calls`, `answer_helm_call`, `helm.asking_to_run` and `helm.call_answered` before this page's renames.
`ask_the_person` and its route `/calls/permission` stay.

## Renames

Calls, in Fleet's `ipc` and in `packages/protocol/src/calls.ts`, which was `helm-calls.ts`:

| Was | Is |
|---|---|
| `HelmCallAnswer` | `CallAnswer` |
| `HelmCallInFlight` | `CallInFlight` |

Events, sent by Fleet and read by Bridge:

| Was | Is |
|---|---|
| `helm.thought_routed` (`HelmThoughtRouted`) | `operator.words_sent` (`WordsSent`) |
| `studio.helm_acted` | `studio.job_acted` (`StudioJobActed`) |

Other types and fields:

| Was | Is |
|---|---|
| `HelmStudioAct` | `StudioJobAct` |
| `FleetHealth.helm_action_authority` | `FleetHealth.action_authority` |
| `SessionRecord.helm` | `SessionRecord.on_your_words` |

## New field

`StartSession.on_your_words`, optional, set by Bridge.
A start through the door reads it as absent, so an agent cannot start a Job on your words.
`SessionRecord.on_your_words` is skipped when false.

## Values that changed

`StudioAuthor`, `Via` and `Actor` write `job` where they wrote `helm`.
The `agent_access` value `Helm only` is `On your words`.
Call ids are `call-N`, where they were `helm-N`.

## Routes

| Was | Is |
|---|---|
| `/helm/permission` | `/calls/permission` |
| `/helm/read_thought` | `/operator/read_thought` |
| `/helm/tell_session` | `/operator/tell_session` |

## Error codes

`fleet.studio_node_not_helms` is `fleet.studio_node_not_a_jobs`.

## Settings

| Was | Is |
|---|---|
| `features.helmCanAct` | `features.onYourWordsCanAct` |
| `timeouts.helmAskHoldSeconds` | `timeouts.askHoldSeconds` |
| `prompts.helmRoute` | `prompts.thoughtRoute` |

`prompts.helm`, `prompts.helmReadOnly`, `prompts.helmVoice`, `effort.helm` and `timeouts.helmReplySeconds` are retired.
A settings file holding an old key is read, written back with the new one, and not refused.

## The store

The migration `20261105T2300Z-on_your_words.renamed.sql` renames `hosted_sessions.helm` to `on_your_words` and clears it.
It ends a live on-your-words Job that holds no slot.
It renames `by_helm` to `by_job` and rebuilds the Studio author tables with `job` for `helm`.
It drops `helm_tuning` and `helm_sessions`.

## What stayed

`SessionRow::Sent` stays on the wire.
The event stream is unchanged in volume: the renamed events have the emitters they had.
The WebSocket sink is no better and no worse bounded than before.
