# A background act's failure: `act.failed`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 23 Oct 2026. A request that replies first and finishes in a spawned task has no answer left to refuse with, so its failure rides the stream.
**One event kind, no operation and no new route beyond the `/events` row**: `act.failed` is in `crates/ipc/operations/act.failed.toml`, `Bridge only`.
The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/act-failed.ts`, written by hand, field for field, and `onActFailed` on the BridgeApi.

| Where | Side | Carries |
| --- | --- | --- |
| `act.failed` (`Event::ActFailed`) | Fleet sends | `ActFailed` `{ act, manifest_id?, session_id?, error }` |
| `BackgroundAct` | | `checkout_to_pull_request`, `commit_to_repository`, `pull_request_press`. A spelling outside these is refused, not guessed |
| `error` | | The `WireError` a request's refusal answers with, whole |

**Fleet publishes it through `Fleet::act_failed(act, manifest, session, refusal)`**, taking the `Refusal` the failed step returned. `manifest_id` and `session_id` are left out when the act had none.
**Bridge shows it as the toast a refusal shows**, in Fleet's own words. The act and what it was for ride as the refusal's fields, so Copy debug info and a Job sent the failure carry them.
**Not replayed.** A resync does not carry it, and a failure while Bridge was disconnected is not told again.
**`Event`, `Delivered`, `StreamMessage` and `Next` are no longer `Eq`.** `WireError` holds a float in its field values, and a variant carrying one cannot be.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** One more kind on it, sent only when a background act fails, which is rare. It neither bounds the sink nor loads it, so the unmeasured risk is neither better nor worse.
