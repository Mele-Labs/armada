# The Job browser: seven acts held for Bridge, `answer_browser_request`, and the `browser_reminder` row

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 28 Oct 2026. A Job's agent drives a page in Bridge, the one the owner sees in the side panel and may take over.
Eight operations, eight routes, one event kind, one report kind and one row kind.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/sessions.ts`, written by hand, field for field; the row is in `hosted-sessions.ts` and the event in `events.ts`.
Nothing in Bridge's main process acts on `browser.request` yet.

## How one act travels

1. The agent calls `browser_open`, `browser_click`, `browser_type`, `browser_snapshot`, `browser_screenshot`, `browser_wait_for` or `browser_evaluate` through the door.
2. Fleet places the Job by the connection, else the body's `session_id`, and checks the act.
3. Where no client holds `/events` open, Fleet refuses at once: Bridge is not open.
4. Fleet mints a request id, `browser-<ULID>`, holds the call under it, and publishes `browser.request`.
5. Bridge does the act and posts `answer_browser_request` with the id and an `answer` or an `error`.
6. The held call answers the agent with Bridge's answer, or Fleet's words for Bridge's error.

**Held 20 seconds**, and `wait_for` 20 seconds more than its own `timeout_ms`, at most 50 seconds in all.
A hosted Job's harness ends an MCP call over HTTP at 60 seconds (spike 15), so the refusal arrives first.
A wait that runs out withdraws the id, and the agent is told "Bridge did not answer. Is the Job browser open?".
**Never written down.** A held call is an agent's open request, which a restart cuts; a Fleet asked to stop lets every one go at once.

## The event

```json
{ "kind": "browser.request", "session_id": "01JOB", "request_id": "browser-01JBX", "act": { "kind": "click", "target": { "role": "button", "name": "Save" } } }
```

| `act.kind` | Carries | Bridge answers |
| --- | --- | --- |
| `open` | `url`, `http` or `https` | `page`, once loaded |
| `click` | `target` | `page`, after the press |
| `type` | `target`, `text`, `submit?` (Enter after `text`; sent only as `true`) | `page`, after the typing |
| `snapshot` | nothing | `page` |
| `screenshot` | `path`, the PNG to write | `screenshot`, once the file is there |
| `wait_for` | `text?` or `target?`, exactly one, and `timeout_ms` as Fleet held it | `page`, once it appeared |
| `evaluate` | `expression` | `evaluated` |

A target is `WalkTarget`, the shape `show_window`'s steps take: `{ role, name?, exact?, within? }`.
**Not replayed**: a Bridge that connects after the event never sees it, and the agent's wait runs out.

## The answer

`POST /sessions/browser/answer`, `agent_access = "No"`, body `AnswerBrowserRequest` `{ request_id, answer? , error? }`, exactly one of the two.
Answered 200 with `BrowserRequestAnswered` `{ request_id, session_id }`.

| `answer.kind` | Fields Bridge sends | What the agent reads |
| --- | --- | --- |
| `page` | `url`, `title`, `snapshot`, the accessibility tree as text | The same, `snapshot` cut to 48 KiB with `truncated: true` |
| `screenshot` | `url`, `title` | The same and `path`, the act's |
| `evaluated` | `url`, `title`, `result?`, `JSON.stringify`'s text of the value, absent where it has none | The same, `result` cut to 32 KiB with `truncated: true` |

| `error.kind` | When | What the agent reads | Code and status |
| --- | --- | --- | --- |
| `owner_driving` | The owner took the browser over | "The owner is driving the Job browser. Ask him to hand it back, or wait." | `fleet.browser_owner_driving`, 409 |
| `not_found` | The act's target is not on the page | "Nothing on the page is a button named "Save". browser_snapshot shows what is there." | `fleet.browser_target_not_found`, 422 |
| `timed_out` | `wait_for` ran out, or a page did not load | ""Saved" did not appear on the page within 5000 ms.", or the address did not finish loading | `fleet.browser_timed_out`, 422 |
| `failed`, with `why` | Anything else: an expression that threw | "The Job browser could not do that: " and `why` | `fleet.browser_failed`, 422 |

Fleet's own refusals to the agent are `fleet.browser_no_bridge` (409, "Bridge is not open, so nothing can drive the Job browser. Ask the owner to open Bridge."), `fleet.browser_unanswered` (409), `fleet.browser_unplaced` (422) and `fleet.browser_refused` (422) for an act that cannot work as asked.

**An answer that does not fit is refused, and the call stays held**, so a corrected one can still land: 422 `fleet.browser_answer_refused` for neither or both of `answer` and `error`, a kind the act does not take, and a `screenshot` while no file is at the act's `path`.
An id nothing holds is refused with its reason: 404 `fleet.no_such_browser_request` for one never minted, 409 `fleet.browser_request_withdrawn` for one whose wait ran out, 409 `fleet.browser_request_answered` for one answered already.

## Where a screenshot goes

**In the Job's scratch folder**, `<scratch>/browser/<request id>.png`, which Fleet makes before it publishes and names in the act.
The agent reads the file there, and it goes when the Job ends, as the scratch folder does.
Fleet's state rather than the worktree, so a picture is never committed by accident.

## The reminder row

The Armada mod reports `say($, id, { kind: "browser_outside", tool, address? })` when the agent reaches for a browser outside Bridge.
`tool` is what the person reads, `cmux` or `Playwright`; `address` the address it opened, where the call named one.
Fleet writes one `browser_reminder` row a report, `{ kind: "browser_reminder", id, at, tool, address? }`, and publishes it as `session.row`.
**The address loses its query, fragment and sign-in**, where a token rides, and is cut to 300 characters.
A blank `tool` is refused with 422 `fleet.session_unnamed`. Reminding once a session is the mod's.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** One event a call, a few hundred bytes, never a snapshot or a picture, which go back over HTTP.
An agent calls one act at a time, so the unmeasured sink risk grows by at most one small event a call and is otherwise neither better nor worse.
