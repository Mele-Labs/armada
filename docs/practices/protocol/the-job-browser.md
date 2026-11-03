# The Job browser: seven acts held for Bridge, `answer_browser_request`, a page's ask in Needs you, and the `browser_reminder` row

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 28 Oct 2026. A Job's agent drives a page in Bridge, the Job's page, in a Browser tab the owner sees and uses at any time too.
Ten operations, ten routes, two event kinds, one report kind and one row kind.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/sessions.ts`, written by hand, field for field; the row is in `hosted-sessions.ts` and the event in `events.ts`.
Bridge's main process acts on `browser.request` in `apps/desktop/src/main/job-browsers.ts` and `job-browser-acts.ts`.

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

## A page's ask

A page in the Job's browser may ask the owner for a permission.
**The ask is in the tab and in the Job's Needs you**, and answering either settles both.
Bridge tells Fleet with `raise_browser_ask` and `end_browser_ask`, both `agent_access = "Bridge only"`.

`POST /sessions/browser/ask`, body `RaiseBrowserAsk` `{ session_id, ask_id, text, options }`, `options` a list of `WaitingOption`.
`ask_id` is Bridge's tab id, and a tab has one ask at a time, so the same `ask_id` again replaces it and keeps the `since` of the first.
Answered 200 with `BrowserAskNoted` `{ session_id, ask_id }`.
A blank `ask_id` or `text` is refused with 422 `fleet.browser_ask_refused`, and a Job Fleet does not know with 404 under the same code.

**The words are Bridge's**, from `apps/desktop/src/shared/browser-ask.ts`: `text` is `Allow {Label} for {host}?`, and `options` are Allow and Block, in that order.
Local network's Allow carries the description "It can reach apps on this Mac and devices on your network."
Fleet keeps what it is given and reads none of it.

Fleet lists it in the Job's `waiting_for` as `{ id: "browser_ask:<ask_id>", text, since, source: "browser_ask", act: { kind: "answer", target: <the id> }, options }`.
It holds no slot and changes nothing about the Job's standing.
Bridge's Needs you list calls the owner for it as it does for a permission.

`POST /sessions/browser/ask/end`, body `EndBrowserAsk` `{ session_id, ask_id }`, is Bridge saying the ask is over.
**Bridge calls it on every end but Fleet's own answer**: the person answered or dismissed it in the tab, the tab closed, or the Job is gone.
Fleet takes the item away and publishes no event.
Answered 200 with `BrowserAskNoted` also where Fleet holds none, since either side may end first.

```json
{ "kind": "browser.ask_answered", "session_id": "01JOB", "ask_id": "tab-7", "choice": 1 }
```

`browser.ask_answered` goes out on the channel `browser.request` does.
`answer_waiting` with a `choice` on a `browser_ask` item publishes it with that index, and `dismiss_waiting` publishes it with `choice: null`.
**`0` is Allow, `1` is Block and `null` is dismissed**, which the page's ask takes as a refusal now that keeps nothing.
Either way Fleet has taken the item off the list first, and Bridge settles the page's ask the way it was answered.
An event for an ask the tab no longer holds is ignored, so a late one does nothing.
**Not replayed**, as `browser.request` is not.

**One ask at a time a tab.**
Bridge refuses a second ask in a tab as a dismissed one, so Fleet holds at most one item a tab.
An ask raised during an agent's act waits on the person, and the agent's held call may run out meanwhile, as *How one act travels* says.

**The agent is told nothing and is not woken** by raising, answering, dismissing or ending.
No message goes to it, no row is written on its thread, and nothing is `Re:`.
An answer in words is refused with 422 `fleet.session_waiting_empty`, as for the other items that take none, and an index past the options with the same code.
The agent's own `waiting_for` replaces its items and no others, so it cannot hold or drop one.
A dismissal is not kept as a closed item, since the tab owns the ask and there is nothing to reopen.

**Held in memory only.**
A Fleet that restarts loses the items, and Bridge still has its bubble.
Where no client holds `/events` open, Fleet drops every ask on the next read, since nothing can act on the answer, and a Bridge that opens later starts with none.
That is the same count `fleet.browser_no_bridge` reads.
A Fleet that has no reader between a Bridge closing and another opening keeps the old asks until the next read of any Job.

**The event stream.** One small event an answer, from a person's press, so the unmeasured sink risk is neither better nor worse.

## What stays in Bridge

**Three things cross no seam to Fleet**: `JobBrowserState.acting`, which names the act in progress for the agent mark, the route that reads the Mac's site defaults, and the `defaultsChanged` notice.
They are Bridge's own channels, in `apps/desktop/src/shared/api/job-browser.ts`, between its main process and its pane.
The person's choices in Site settings, which answer a page's permission, download, popup or fullscreen request, stay in Bridge too: Fleet holds only the ask that a row set to Ask raises.

## The reminder row

The Armada mod reports `say($, id, { kind: "browser_outside", tool, address? })` when the agent reaches for a browser outside Bridge.
`tool` is what the person reads, `cmux` or `Playwright`; `address` the address it opened, where the call named one.
Fleet writes one `browser_reminder` row a report, `{ kind: "browser_reminder", id, at, tool, address? }`, and publishes it as `session.row`.
**The address loses its query, fragment and sign-in**, where a token rides, and is cut to 300 characters.
A blank `tool` is refused with 422 `fleet.session_unnamed`. Reminding once a session is the mod's.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** One event a call, a few hundred bytes, never a snapshot or a picture, which go back over HTTP.
An agent calls one act at a time, so the unmeasured sink risk grows by at most one small event a call and is otherwise neither better nor worse.
