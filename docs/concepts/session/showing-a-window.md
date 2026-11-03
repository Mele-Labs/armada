# Showing a window

**Part of:** [Job](../session.md).

**A job shows the person a web page with `show_window`** (`url`, optional `title`, optional `variations`, optional `steps`, optional `origins`, optional `sign_in`, optional `brief`), a tool on the agent's door, and Bridge opens it as a tab of the job by itself.
**No window opens and nothing takes focus.**
The walk is a tab of the job in the Cockpit, beside the thread, named after its page.
The job's row and the tab pulse until it has been seen.
Each walk has its own tab, so a job with several walks has several tabs, and several things to approve are several `show_window` calls, one each.
Any `http` or `https` address; any other scheme is refused with its reason.
Fleet places a hosted job by its connection, as for its asks, so the call names no job; a terminal job names its own id, which the mod puts in the model's first-message context.
It writes one `window` artifact on the ledger however often the address is shown, and a quiet `window` row on the thread each time, which is what makes Bridge open the tab.
Pressing the ledger row or the thread row opens it again, only for an address the ledger shows as a window.
**A note taken on the page is held on the job**, with the element it pointed at, and shown at the end of the thread.
While any is held, Approve becomes Send notes, a split button, and sending them goes to the job as one turn from the person, which wakes it (*Pending notes*, `docs/practices/capture-window.md`).
The window is pinned to the origin it opened on, as a server's is, and to the `origins` the call names (*A deployed page*).
**`variations`, a list of `{name, url}`, offers alternatives of one change in the one tab**, `url` the first's and each held to the same `http`/`https` rule.
The person picks one: the bar draws them under *Pick one* on its first row, a press swaps the page, and the pin moves to the origin of the take chosen and to no address the job did not send.
A note names the take it was taken on (`On B, Needs you first: …`), Approve names the one on screen (`Approved B, Needs you first: <url>`), and the window row and the ledger's `window` artifact keep the takes so a reopened tab brings them back.
The walk ends on an Approve of any take, so several things to approve are never takes of one.
**`brief`, at most 400 characters, is what the job wants looked at**, and the person reads it behind *What to look at* on the bar.
A longer one is refused with 422 and nothing written.
**`steps` walks the person through any page, on any repository's app**: a local dev server or a deployed staging address.
Each step is a `say` caption and one verb (`press`, `look`, `hover`, `later`, `type`, `key`, `select`, `paste` or `drag`) aimed at a target found by role and accessible name, or by the words it draws: the mock walks' own shape, as JSON, so a page needs nothing of Armada's in it.
Bridge plays them from outside the page.
It rings each target and draws the caption over the page, and acts on the ringed target with real mouse and keyboard input when the person presses Next, so the page hears what it would from him.
The bar's walk row (Back, Next, Autoplay, Replay and the keys) drives them as it drives a mock's walk.
**A target Bridge cannot find holds the walk on that step**, the bar marks it, and Fleet tells the job in its own voice which step and which target, once (`window_failed`), so it fixes its steps and shows the window again.
Fleet refuses steps of the wrong shape with the first one that is wrong.
They are kept on the `window` artifact and row beside the takes, and a window shown with other steps is another window.
A page that plays its own walk (Armada's mock) is passed none and plays as it did.
**Where a job passes steps to one anyway, the page's own walk wins**: Bridge stops the steps, the bar drives the page's walk, the page's card leaves to the bar, and Fleet tells the job once (`window_failed`).
The steps hold their first act until the page has been looked at for a walk of its own.
Since 28 Oct 2026, when a mock shown with steps drew its own card beside the bar's steps; playing every mock walk from the window is `armada-iam`.
`docs/practices/capture-window.md`, *A walk in the page*.
**The bar says whether the page is ready to walk**, a Job mark at its start (*Ready to walk*, or *Still changing*) and the bar's hue, and Approve reads *Not ready* while it changes.
**It is detected, not declared:** `show_window` says ready, the mod's report of a file the job itself wrote (`wrote`; a subagent's does not count) or a note from the window marks every window the job showed as working (`detail.working` on its artifact), and showing the page again or the end of the turn (`turn_completed`) marks it ready.
**A second mark, `detail.changed`, is set with `working` and stays when the page is ready.**
Showing the page again replaces the artifact and clears it, and nothing else does.
A window with it raises no `walk:` item, so a page the job is still fixing does not call him each time the job pauses between fixes, and holds a pending plan back while it is the newest window or one he has not approved ([The plan](the-plan.md)).
The bar's ready mark reads `working` only.
**A turn that ends while a `subagent` of the job still stands leaves the windows working**, since a job that hands a note to a background subagent and ends its turn has not finished changing the page; the subagent settling marks nothing, and the turn that takes its report does.
A subagent whose own transcript shows it finished is settled spent at the turn end, since the mod never settles one and one left standing kept every window working for a day.
Only subagents count: a server or a shell left running holds no window.
**The tab sits beside the job's thread**, so the walk is read and answered without leaving the job.
A pull request's walk keeps the walk window and its panel beside the page: `docs/practices/capture-window.md`, *The panel beside the page*.

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
It is held for as long as Bridge is away, so a Bridge that connects after the page was shown opens the tab of each live job whose newest shown page it never opened and was shown in the last 15 minutes, and takes the sign-in then.
Bridge holds it for as long as the window takes to load it.
A load of one that fails is told to nobody.
A tab reopened from the ledger or the thread signs in with none, so it relies on the jar.
A popup onto the shown origin or a named one opens as a child of the window and closes on `window.close()`; every other popup is refused.
The window's user agent is Chrome's, without the Electron token.
While the page is at a provider or its popup is open, a walk with `steps` waits on its step and tells the job nothing (`docs/practices/capture-window.md`, *A popup a Job's page opened*).

**A question asked while the person walks a page is put to him as at any other time.**
A walk is a tab beside the thread and can stay open for hours, so a job that could not ask for that long could not ask at all.
`ask_in_window` asks about the page itself (*Asking in the window*).

| | |
|---|---|
| A walk opens | At `show_window`: the newest `window` row on the thread opens as a tab of the job, beside its thread, named after its page. The job's row and the tab pulse until it has been seen. No window opens and nothing takes focus or switches the page. Each walk has its own tab. A Bridge that was not connected opens the tabs when it connects |
| A walk ends | The person approves the page (`Approved: <url>`, or of several takes `Approved B, Needs you first: <url>`), Bridge says the tab closed (`window_closed`), or a newer `show_window` replaces it. The row gets `ended_at` and is published again. Only his Approve settles the `walk:` waiting item, or a newer window that shows one of its pages, compared without the port. Approving closes its tab |
| × on the tab | The walk is still owed. The row keeps pulsing, and pressing it brings the tab back |
| A note on the page | Ends nothing and settles nothing: the `walk:` waiting item stays until his Approve. It is held on the job and sent with Send notes |
| The window cannot show the page | Ends nothing. Bridge says so (`window_failed`) and Fleet tells the job in its own voice, with the address and what went wrong: a failed load, a status of 400 or more, a crashed page, Vite's error overlay or a page with nothing drawn. Not while the page is still changing, and once until it is seen working. A failed load, where nothing answered on the address, also spends the window's artifact, so its `walk:` item leaves Needs you until the job shows it again; a page that answered broken still waits |
| A step cannot be played | Ends nothing. The walk holds on the step, and Fleet tells the job in its own voice which step and which target Bridge could not find (`window_failed` again, with a reason that begins `step`), once until the step is found |
| A question meanwhile | Put to the person as at any other time, through a hosted job's permission door or a terminal job's `ask_from_terminal`. Nothing is refused and the job is not told to end its turn. A question about the page is asked in the tab with `ask_in_window` |

A thread that holds an ask row settled `kept` still answers it from Needs you (`docs/practices/protocol/a-question-refused-for-a-walk-is-kept-whole.md`).
`show_window`'s description tells the agent the same.
Closing the tab ends the walk and does not settle its `walk:` item, which still waits until his Approve, as does the item of every older page he has not approved, and the tabs are read from those items.
A newer window of the same page replaces it, though: a dev server started again often takes another port, and 19 Oct 2026 the owner was left a walk on a server long stopped, after approving the same walks on its new port.
A Bridge that quits with a tab open may not reach Fleet to say so.
The walk then reads open until it is approved or replaced.
