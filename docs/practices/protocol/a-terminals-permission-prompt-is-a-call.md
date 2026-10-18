# A terminal's permission prompt is a call

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Oct 2026. `docs/concepts/session.md`, *A terminal's permission prompt*; measured in `docs/spikes/031-can-bridge-answer-a-terminal-permission-prompt.md`.
**Additive only**: two `SessionFact` variants a terminal's mod reports, an optional `tool` on the mod's `asks`, and a `WaitingItem` with the existing `permission` source.
No operation, event kind or error is new.
Bridge's mirror of the facts and of `asks` is the mod's `plugins/armada/hooks/fleet.ts`; `packages/protocol` is unchanged, because the item and the open ask are existing shapes.

| Where | Carries | Notes |
| --- | --- | --- |
| `SessionFact::PermissionAsked` (`report_session`) | `tool`, `detail?`: the command's first line, the path or the address, cut at 300 characters | The harness opened its own permission prompt (Claude Code's `PermissionRequest` hook, where no hook beneath decided it). Ignored for a hosted Job, whose permission is Fleet's own ask |
| `SessionFact::PermissionSettled` | nothing | The tool call that raised the prompt ended, whichever way it was answered |
| `TerminalAsk::Asks` (`ask_from_terminal`) | `tool?`, since mod `0.3.22` | The mod puts the prompt to Fleet as it puts a question, and polls for the answer. Absent is `AskUserQuestion`. A permission offers Allow once and Refuse, never Allow and remember, draws no row in the thread, and is not refused while a walk is open |
| `WaitingItem` (`source: permission`) on a terminal Job | `id` `perm:<call>`, `text` the tool and its detail, `act` `answer`, `options` Allow once and Refuse | The open ask, as a hosted Job's is: `answer_session` and `answer_waiting` reach the polling mod as `answered` or `refused`. The terminal answering first settles it as answered there |
| `WaitingItem` from the fact alone | `id` `perm:<when Fleet heard it>`, `act` `message` naming the Job, no `options` | Until the ask arrives, and for a mod that reports the fact but cannot put it. `answer_waiting` on it is refused as `fleet.session_waiting_unheld`: only the terminal answers it |

**Held, not stuck.** While a prompt is open the terminal's turn is waiting on the person, so the stuck tick leaves it out, as it does a hosted Job's open ask.
A turn starting or ending, and the Job ending, closes the prompt in Fleet without a `permission_settled`.
It is kept in memory only; the mod tells it again when a Fleet that was out of reach answers.

**Skew.** A mod before `0.3.22` reports neither fact and puts no permission, so its Job's prompt raises no call and reads stuck after ten minutes, as before.
A Fleet before this refuses both facts as unreadable and reads a permission's `asks` as a question; the mod's report is not awaited, so that costs the job nothing.
