# A Job's delivery question

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 19 Oct 2026. `docs/concepts/session.md`, *The delivery question*.
**Additive only**: one value on `WaitingSource`, one `SessionRow` kind and one type, no operation, event kind or route.
The protocol ID moves with the files below, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `WaitingSource` `delivery` | Fleet writes, Bridge reads | Derived on every read from the thread's `delivery` row while it has no choice. Id `delivery`, text `How should this work be delivered?`, `act` `{kind: "answer", target: "delivery"}`, `options` exactly `Auto land` (choice 0), `Open PR only` (choice 1) and `Keep local` (choice 2), each with a `description` |
| `SessionRow` `delivery` | Fleet writes, Bridge reads | `id`, `at`, `choice` (`DeliveryChoice`, absent while asked) and `named`, set where the first message named it. Replaced by id when answered |
| `DeliveryChoice` | | `auto_land`, `open_pr`, `keep_local`. Bridge's half is `packages/protocol/src/hosted-sessions.ts` |
| `answer_waiting` | Bridge sends, Fleet takes | `choice` 0, 1 or 2 on that item. Fleet keeps it on the row and sends the agent the choice. Words or a mode without a `choice` are a 422 `fleet.session_waiting_empty` |
| `dismiss_waiting` | | Refused 422 `fleet.session_waiting_empty` for a `delivery` item: it is answered |

**Skew.** A Bridge before this meets `delivery` as a source and a row kind it does not know, and the IDs differ, so the two are refused regardless.
The event stream is untouched beyond the `session.row` and `session.changed` the item already rides on, which makes the unmeasured sink risk neither better nor worse: one row and one item per Job, once.
