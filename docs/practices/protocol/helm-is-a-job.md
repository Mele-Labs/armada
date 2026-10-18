# Helm is a Job

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/helm.md`, `.claude/decisions/2026-10-13-helm-is-a-session.md`.
Helm's dock moves off its own host and socket onto the hosted-Session engine.
Bridge has moved with it: the dock reads the Helm Job through the Jobs pipeline and calls only the routes below.

| Where | Side | Change |
| --- | --- | --- |
| `SessionRecord.helm` (`ipc::SessionRecord`, `SessionRecord.helm?: true`) | Fleet writes, Bridge draws the mark | `true` on a repository's Helm Job, skipped when false. Marks it in `list_sessions` and in `get_session` |
| `AskHelm.model`, `AskHelm.effort` | Bridge writes | Optional. The model and effort for the Job this message starts. Ignored once one is live: `tune_session` changes a live one. 422 `fleet.unacceptable_effort` for anything but `low`, `medium` or `high` |
| `ask_helm` (`POST /helm/ask`) | Bridge calls | Answers 200 `SessionRecord` (was 202 `HelmConversation`). Finds or starts the repository's Helm Job, sends the message with the context line ahead of the words, and returns. The reply is `session.row` |
| `get_helm_session` (`GET /helm/session?manifest_id=`), new | Bridge calls | Answers `HelmSession`: `manifest_id` and `session?`, a `SessionThread`. `session` is absent before the first message and after Start fresh |
| `start_helm_fresh` (`POST /helm/start_fresh`) | Bridge calls | Answers `HelmSession` (was `HelmConversation`). Ends the live Helm Job with `close_session`, giving back the slot it held. The next message starts a new one |
| `record_thought_sent` (`POST /helm/thought_sent`) | Bridge calls | Answers `SessionRecord`. Appends a `sent` row to the Helm Job's thread, starting the Job if the repository had none |
| `SessionRow::Sent` (`kind: "sent"`) | Fleet writes, Bridge draws | `id`, `at`, `text`, `target: ThoughtTarget`, `files?`, `tags?`. Only a Helm Job's thread has one, written by `record_thought_sent` and by `tell_session` |
| `tell_session` (`POST /helm/tell_session`) | Helm calls | Unchanged on the wire. Its `Helm only` reach and its `sent` row are what changed |
| `get_helm_debug_info` (`GET /helm/debug`), `HelmDebugInfo` | Fleet writes, Bridge draws | `brief` renamed `note`; `servers` removed. `session` is the Helm Job's id and is absent when none is live |
| `HelmDebugSaid` | Fleet writes, Bridge draws | `asked`, `said`, `called`, `refused`, and `sent` (`text`, `to`). `ended`, `fresh` and `unanswered` are gone: a Job's thread has no such rows |
| `get_session_file` | Bridge calls | Also serves the files of a `sent` row |
| `observe_commander`, `read_in_link`, `helm.call_answered`, `helm.thought_routed` | Fleet | Descriptions only. No field changed |
| `prompts.helm`, `prompts.helmReadOnly` (settings) | Machine setting | Placeholders `{manifest}`, `{folder}`, `{unasked}` and `{manifest}`, `{folder}`. The brief's texts and their placeholders are gone. `agent-prompt.md` section 5a |

**Retired from the wire.** Operations `observe_helm`, `tune_helm`, `get_helm_file` and the event `helm.changed_checkout`; types `TuneHelm`, `HelmBoxFacts`, `HelmConversation`, `HelmMessage`, `HelmOpened`, `HelmClosed`, `HelmSilence`, `HelmChangedCheckout` and `HelmSent`.
They are deleted in Rust and in `packages/protocol/src` (`helm-thread.ts` with them).
`observe_commander` and the Commander's types keep the old host.
A route typo is a runtime 500 here as everywhere (the accepted cost), so the dock should be walked against a Fleet that has this change.

**What lands in Bridge.**

| Where | Side | Change |
| --- | --- | --- |
| `BridgeState.helm` (`HelmAim`: `{state: "none"}` or `{state: "pointed", manifestId}`) | Bridge | Was `HelmThread`, the dock's whole conversation. It is only which repository Helm answers for. The thread, the turn, the model and the commands are the Helm Job's, in `BridgeState.sessions` |
| `HelmCarry.model`, `HelmCarry.effort` | Bridge | Optional. What the box held before the Job existed, sent as `AskHelm.model` and `AskHelm.effort` on the first message. Once the Job is live the box tunes it with `tune_session` |
| `SessionsDraft.helmOf(manifestId)` | Bridge | The Helm Job of the repository Helm is pointed at, which the Jobs list does not hold when the picker picks another repository |
| `Session.helm` (`sessions-wire.ts`) | Bridge | Read from `SessionRecord.helm`. The Jobs list draws Helm's mark, and opening that row opens the dock |
| `rowsOfThread`, kind `sent` | Bridge | Draws `SessionRow::Sent` as the person's words with a "Sent to" chip. `Beside.repository` names the target's repository |
| `HelmConnection` (`apps/desktop/src/main/helm.ts`) | Bridge | No socket and no thread. `ask_helm` and `record_thought_sent` answer a `SessionRecord`, which main folds into the Jobs host at once. `HelmSocket` is gone |
| `tuneHelm`, `readHelmFile` (preload and `ipc` channels) | Bridge | Removed. `tune_session` and `get_session_file` serve the same acts |

**How the dock reads a thread.** `get_helm_session` once, then `session.row` and `session.changed` on `/events` by the job's id, and `session.hosted` for `commands`, `model`, `effort` and `turn`.
A Bridge that reconnects reads `get_helm_session` again, as a Job page does: that is what the `missed` message on `/events` is for.

**The event stream, and the open risk.** Better in one respect and no worse in any other.
The Helm socket had a bounded channel of its own, per conversation, with a drop-oldest `missed` message; that is gone, and with it a second thing that could outrun a slow viewer.
Helm's thread now rides `session.row` and `session.changed` on the shared drop-oldest `/events` channel.
It adds no queue and no event kind.
**It does add rows**: a Helm Job streams what a Job streams, including its tool calls, and a Helm Job that works hard against a minimised Bridge is a source like any Job's.
The unbounded sink on the application side of axum's WebSocket stays as it was, unmeasured.
A viewer that fell behind learns it from `missed` on `/events` and resyncs with `get_helm_session`.
Nothing here makes the "you missed N events, resync" message less necessary.

**Skew.** None to meet beyond the rule: Protocol IDs equal or Bridge opens no socket and the lifeboat is the fallback.
The change touches the wire, so the ID moves; there is no order between two IDs and nothing here depends on one.
**The lifeboat is untouched**: `/v0/` is still four hand-written routes with no dependency, and none of this is on them.

**What is not on the wire.** The old Helm thread files under the records root are not read.
The Helm Job's `helm` flag is on the store's `hosted_sessions` row (migration `20261013T1700Z-helm_session.hosted_helm`), not a second table; `live_helm_of` and `latest_helm_of` find it.
