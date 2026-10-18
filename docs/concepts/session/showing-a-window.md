# Showing a window

**Part of:** [Job](../session.md).

**A job shows the person a web page with `show_window`** (`url`, optional `title`, optional `variations`, optional `steps`, optional `origins`, optional `sign_in`), a tool on the agent's door, and Bridge opens it in a capture window by itself.
Any `http` or `https` address; any other scheme is refused with its reason.
Fleet places a hosted job by its connection, as for its asks, so the call names no job; a terminal job names its own id, which the mod puts in the model's first-message context.
It writes one `window` artifact on the ledger however often the address is shown, and a quiet `window` row on the thread each time, which is what makes Bridge open (or raise) the window.
Pressing the ledger row or the thread row opens it again, only for an address the ledger shows as a window.
**A note taken in the window goes to the job as a message from the person**, with the element it pointed at, and wakes it.
The window is pinned to the origin it opened on, as a server's is, and to the `origins` the call names (*A deployed page*).
**`variations`, a list of `{name, url}`, offers several takes of one change in the one window**, `url` the first's and each held to the same `http`/`https` rule.
The bar draws a letter and name per take, a press swaps the page, and the pin moves to the origin of the take chosen and to no address the job did not send.
A note names the take it was taken on (`On B, Needs you first: …`), Approve names the one on screen (`Approved B, Needs you first: <url>`) and is per take, and the window row and the ledger's `window` artifact keep the takes so a reopened window brings them back.
**`steps` walks the person through any page, on any repository's app**: a local dev server or a deployed staging address.
Each step is a `say` caption and one verb (`press`, `look`, `hover`, `later`, `type`, `key`, `select`, `paste` or `drag`) aimed at a target found by role and accessible name, or by the words it draws: the mock walks' own shape, as JSON, so a page needs nothing of Armada's in it.
Bridge plays them from outside the page.
It rings each target and draws the caption over the page, and acts on the ringed target with real mouse and keyboard input when the person presses Next, so the page hears what it would from him.
The bar's walk row (Back, Next, Autoplay, Replay and the keys) drives them as it drives a mock's walk.
**A target Bridge cannot find holds the walk on that step**, the bar marks it, and Fleet tells the job in its own voice which step and which target, once (`window_failed`), so it fixes its steps and shows the window again.
Fleet refuses steps of the wrong shape with the first one that is wrong.
They are kept on the `window` artifact and row beside the takes, and a window shown with other steps is another window.
A page that plays its own walk (Armada's mock) is passed none and plays as it did.
`docs/practices/capture-window.md`, *A walk in the page*.
**The bar says whether the page is ready to walk**, a Job mark at its start (*Ready to walk*, or *Still changing*) and the bar's hue, and Approve reads *Not ready* while it changes.
**It is detected, not declared:** `show_window` says ready, the mod's report of a file the job itself wrote (`wrote`; a subagent's does not count) or a note from the window marks every window the job showed as working (`detail.working` on its artifact), and showing the page again or the end of the turn (`turn_completed`) marks it ready.
**A turn that ends while a `subagent` of the job still stands leaves the windows working**, since a job that hands a note to a background subagent and ends its turn has not finished changing the page; the subagent settling marks nothing, and the turn that takes its report does.
Only subagents count: a server or a shell left running holds no window.
**The window draws the job beside the page**, its thread and the box to answer it, so the walk is read and answered without going back to Bridge's window.
`docs/practices/capture-window.md`, *The panel beside the page*.

**A deployed page** can be walked as a local one is.
A staging environment behind single sign-on redirects to a provider and back, so `show_window` takes `origins`, a list of other `http` or `https` origins (a host and an optional port, no path) the window may pass through.
A navigation or redirect to one of them is followed; anything else is refused as before, and the page stays pinned to the shown origin and those.
A call whose `origins` or `sign_in` break the rule is refused with 422 and nothing written.
The window keeps **one cookie jar per repository**, read off the Job's `manifest_id`, and Jobs in no repository share one unnamed jar.
So a sign-in holds for later walks of that repository, and two repositories share none.
How the window gets signed in depends on the repository:

| The repository | The job passes | Who signs in |
|---|---|---|
| Gives no way in | `url` and `origins` | The person, on the provider's page in the window, once. The jar keeps it |
| Has a dev magic link or a token login address | `url`, `origins` where it redirects, and `sign_in`, a list of addresses on the shown origin or a named one | The window, before the page loads: each address in order, in the repository's jar |

**`sign_in` is a secret.** Fleet holds it in memory from the call until Bridge takes it (`take_window_sign_in`, once), the job shows another window, or the job ends, and writes it nowhere: not the thread, the ledger, a `window` row, the timeline, a note, the receipt or an event.
It is held for as long as Bridge is away, so a Bridge that connects after the page was shown opens the window of each live job whose newest shown page it never opened and was shown in the last 15 minutes, and takes the sign-in then.
Bridge holds it for as long as the window takes to load it.
A load of one that fails is told to nobody.
A window reopened from the ledger or the thread signs in with none, so it relies on the jar.
A popup onto the shown origin or a named one opens as a child of the window and closes on `window.close()`; every other popup is refused.
The window's user agent is Chrome's, without the Electron token.
While the page is at a provider or its popup is open, a walk with `steps` waits on its step and tells the job nothing (`docs/practices/capture-window.md`, *A popup a Job's page opened*).

**A question asked while the person walks a page is refused, not held.** A message starts a turn only once the current one ends, and an open `AskUserQuestion` keeps it open, so every note and the Approve would queue behind the question until he answered it. [Spike 27](../../spikes/027-how-does-fleet-deliver-text-to-a-live-terminal-session.md).

| | |
|---|---|
| A walk opens | At `show_window`: the newest `window` row on the thread. A Bridge that was not connected opens it when it connects, if the walk has not ended and was shown in the last 15 minutes |
| A walk ends | The person approves the page (`Approved: <url>`, or of several takes `Approved B, Needs you first: <url>`), Bridge says its window closed (`window_closed`), or a newer `show_window` replaces it. The row gets `ended_at` and is published again |
| A note in the window | Ends nothing, though it settles the `walk:` waiting item |
| The window cannot show the page | Ends nothing. Bridge says so (`window_failed`) and Fleet tells the job in its own voice, with the address and what went wrong: a failed load, a status of 400 or more, a crashed page, Vite's error overlay or a page with nothing drawn. Not while the page is still changing, and once until it is seen working |
| A step cannot be played | Ends nothing. The walk holds on the step, and Fleet tells the job in its own voice which step and which target Bridge could not find (`window_failed` again, with a reason that begins `step`), once until the step is found |
| A question meanwhile | Refused with a reason the agent reads, and **kept whole**: Fleet writes the ask row with every question, header, option and description, settled `kept`, so it draws in the Needs you list as the question that lapsed does, to be answered after the walk (the answer goes to the agent as his message). The agent is told not to add it to `waiting_for`, and to end the turn. A question about the page is asked in the window with `ask_in_window` instead (*Asking in the window*). Since 17 Oct 2026, the owner: "why can't we keep all of the options and everything about the question? Turning it into a reply box makes no sense." A `kept` question, unlike an `unanswered` one, is not cleared by his notes or his Approve, which are what he sends during a walk; a newer question or his answer to it clears it A hosted job's is refused at its permission door; a terminal job's at `ask_from_terminal`, which the mod asks before the terminal's own prompt opens |

The mod's first-message note and `show_window`'s description tell the agent the same.
Closing the window ends the walk and does not settle its `walk:` item, which still waits.
A Bridge that quits with a window open may not reach Fleet to say so.
The walk then reads open until it is approved or replaced, and a question meanwhile is kept as above, which blocks nothing.
