# Routing a thought: Helm reads words for where they belong

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**Since 13 Oct 2026 the `sent` row is a `SessionRow::Sent` in the Helm Job's thread, and `record_thought_sent` answers a `SessionRecord`.** The `observe_helm` message `sent` and `HelmSent` rows below are retired; see *Helm is a Job*.

`docs/concepts/helm.md`, *Routing a thought*. **Additive only**: two operations, one event kind and one message on the Helm socket, all of Bridge's (`agent_access` `No`, `Bridge only` for the event). No type that exists changes. Bridge's half is `packages/protocol/src/thought.ts`, `helm.ts` and `helm-thread.ts`, written by hand. There is no number to take: the protocol ID moves with the wire files, and `armada need` has nothing to order.

| Where | Carries | Notes |
| --- | --- | --- |
| `read_thought` (`POST /helm/read_thought`) | `ReadThought`: `text`. Answers `ThoughtReading`, tagged by `kind`: `question`, or `thought` with `target?` | One model call on the proposer's model, over every live Session and running Job in every repository. Always 200: a call that fails, runs past 20 seconds or answers what was not asked is `question`. An id the model names that was not offered is `thought` with no `target`. A message opening on a wh-word and ending in `?` is `question` with no call |
| `ThoughtTarget` | `kind` (`session` or `job`), `id`, `title`, `manifest_id?`, `slot?` | `slot` is the slot a Job holds. `manifest_id` is absent for a Job in no repository Fleet serves. A plain enum rather than a `wire_enum!`: no registry declares the set, and a third kind would be a new act Bridge has to learn |
| `record_thought_sent` (`POST /helm/thought_sent?manifest_id=`) | `ThoughtSent`: `text`, `target`. Answers 200 `HelmConversation` | Said after the delivery went through, so a refused delivery records nothing. Appends the row below and publishes `helm.thought_routed`. The manifest names the conversation, absent is the first served |
| `observe_helm` message `sent` | `HelmSent`: `ts`, `text`, `target` | A row of the thread's file, so it is in the backfill after a restart. Never delivered to Helm's job |
| `helm.thought_routed` (event) | `HelmThoughtRouted`: `manifest_id`, `to` (`session` or `job`), `id`, `at` | The audit event of *Audit trail*. Names where and never the words. Rust only: Bridge mirrors no `helm.*` audit kind |

**This reads across repositories, which `helm.md` otherwise says Helm does not do.** The owner decided it on 12 Oct 2026 for this one read. `read_thought` takes no `manifest_id`, so what it can see does not depend on where Helm points.

**The event stream.** One small event per delivery, a person's press apart, on the shared drop-oldest channel. It adds no queue and assumes no delivery is complete, so it leaves the unmeasured sink risk where it was. The `sent` message rides the conversation's own bounded channel, which is drop-oldest with a `missed` count, and the thread's file keeps what a slow viewer lost.

**`tell_session` is Helm's own delivery, and the one operation here Bridge never calls.** `POST /helm/tell_session?manifest_id=`, body `TellSession`: `session_id`, `text`. `agent_access` is `Helm only`, so the door offers it to a Helm job alone and a Drone's endpoint never lists it. It answers 200 with the `ThoughtTarget` the words went to. Fleet sends through `send_session_message`'s path, so a refusal is that send's, and only a send that was taken appends the `sent` row and publishes `helm.thought_routed`. The door names the Manifest the job stands in as `manifest_id` (`NAMES_ITS_SCOPE`), which picks the thread. No type that exists changes, and `packages/protocol` has no mirror of `TellSession` because nothing in Bridge sends it.

**Skew.** A refusal on an ID mismatch means no peer meets this change half built. A Fleet without the routes would answer 404 to a Bridge that has them, and Bridge treats a read that fails as `question`, so the words still reach Helm.
