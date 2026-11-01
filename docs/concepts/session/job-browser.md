# The Job browser

**Part of:** [Job](../session.md).

**A Job has one web page that its agent drives and the person watches, and may take over.**
It is a Browser tab in a pane of the Job, beside the thread or any other tab, so the page and the Job are read and answered together.
`+` on a pane's tab row offers Browser, and each Browser tab has a page of its own, named by its address.
It is the page the Job's browser tools act on, so it is how a Job opens its own work and looks at it without a second browser the person cannot see.
It is not the walk window: [Showing a window](showing-a-window.md) shows the person a page to walk and approve, and this is the page the agent works in.

**Each tab's page is one view for the tab's life in Bridge**, kept while another tab is in front of it and when it moves to a window, so it holds its history, its sign-in and what was typed.
It exists for a Job Fleet holds, once the tab opens or the agent opens a page, and goes when the tab is closed, the Job ends or Bridge quits.
**The page the agent drives is the Job's page**: the first Browser tab in pane order that is showing, else the one shown last, else the first.
An agent's open on the Job on screen, with no Browser tab, makes one, in the pane that holds a Browser, else in a new pane on the right; for a Job not on screen no tab opens until the Job is.
It runs in its repository's own session as the walk window's page does, with no preload and every permission refused (`docs/practices/capture-window.md`, *What the page can reach back into*).
Its code is `apps/desktop/src/main/job-browsers.ts`, `apps/desktop/src/main/job-browser-acts.ts` and `apps/desktop/src/renderer/src/job-browser/`.

## In the tab

**The toolbar is, left to right:**

| Control | Does |
|---|---|
| Driver glyph | Says who drives, and a press changes it (*Who drives*) |
| Back, Forward, Reload | Move along the page's history and load it again. `⌘[`, `⌘]` and `⌘R` do the same with the pane focused |
| Address | The page's address, which the person edits only while he drives |
| Site settings | The lock before the address: what the page may use, for this site and for every site |
| Capture | Pick a part of the page and send the Job a note on it (*Capture*). `⌥⌘C` |

The page sizes to its pane, and a tab behind another hides its page with everything it held.
Closing the tab destroys the page.

## In a window

**Move to window, on the tab's menu, moves the page to a window of its own, with the Job's thread beside it**, as the walk window draws its Job.
The page is the same one, so nothing it held is lost, and the tab stays in its pane and says where the page is.
**Move to the pane**, on the tab or in the window's toolbar, closes the window and shows the page in the tab again.
Closing the window puts the page back in its tab.

## Who drives

**The driver glyph is the toggle**: an agent mark reading *Agent driving. Take over*, a person mark reading *You're driving. Hand back to the agent*.
While the person drives, the address takes an `http` or `https` address and Enter loads it, and every act of the agent's is refused with `owner_driving`, which the agent reads as "The owner is driving the Job browser.
Ask him to hand it back, or wait." (`docs/practices/protocol/the-job-browser.md`, *The answer*).
Handing it back lets the agent's next act through.
**The glyph decides what the agent may do, not what the person may press**: the person's own presses reach the page while the agent drives too.

## Capture

**Capture sends the Job a note on a part of the page**, as the walk window's note control does, and is the same control: its glyph, its words and `⌥⌘C`, which the page hears while it holds the keyboard.
Pressed, it freezes the page to a picture of it, because nothing in Bridge's document can take a press over a native view.
A press on the picture picks the part under it, by the walk window's own layer, which reads that part in the page, and a card under the part takes the note.
**Send (`⌘↵`) sends the note to the Job as the person's** (`send_session_note`), with the part named, the address, and the whole picture with the part ringed and tagged 1, and wakes the Job; Cancel (`esc`) puts the picture away and nothing is sent.
Sent or cancelled, the page is live again.

## What the agent does with it

**The agent drives the page with seven tools on its door**, and each answers in the same call with the page as it stands after the act.
Fleet holds the call until Bridge does the act and answers it, so Bridge must be open; the wire, the holds and the refusals are in [The Job browser](../../practices/protocol/the-job-browser.md).

| Tool | Does |
|---|---|
| `browser_open` | Loads an `http` or `https` address and answers when it has loaded |
| `browser_click` | Presses a target found by role and name, with real mouse input |
| `browser_type` | Types into a target, over what it held, with Enter after if asked |
| `browser_snapshot` | Reads the page as an accessibility tree in text |
| `browser_screenshot` | Writes the page as a PNG into the Job's scratch folder and names the file |
| `browser_wait_for` | Waits for text or a target to be on the page, or times out |
| `browser_evaluate` | Runs an expression in the page and answers its value as JSON |

A target is found as a walk's steps find theirs, by role and accessible name, so a page needs nothing of Armada's in it.
**`browser_open` on the Job on screen with no Browser tab opens one**, on the address.
Any other act on one is refused with *The Job browser is closed. Call browser_open first.*

## Where it is in the Cockpit

**Browser is a tab kind**, offered by `+` on any pane's tab row.
A Job on screen that has none opens one when its agent opens a page, or when the person presses `+` and Browser, and the tab is named by its page's address.
Flight control has no Browser tile.

## When a Job reaches for another browser

**A Job that drives a browser outside Bridge is reminded of this one, and never stopped.**
The Armada mod recognises three reaches: a Bash call that runs `cmux browser`, the `cmux-browser` skill, and a Playwright browser tool over MCP (`mcp__…playwright…__browser_…`).
A call that is none of them goes unremarked, among them `cmux notify` and a `playwright test` run in a shell.
The call is allowed and runs, and **the first one in a session** carries a reminder beside its result that names the seven tools.
The mod reports the reach to Fleet once a session.

**The thread draws a row where it happened**: *Opened a browser outside Bridge with cmux. This job has a Job browser.*, with the tool Playwright or cmux.
Its **Open Job browser** brings the Browser tab forward, or opens one, and sends the Job a message from the person, "Use the Job browser for this. You were on `<address>`.", once.
The address is left off where the call named none, and the row reads *Sent to the job* from the person's message on, so it cannot go twice.
The row keeps the address without its query, fragment or sign-in (`docs/practices/protocol/the-job-browser.md`, *The reminder row*).

## What it does not do

- **It lives until its tab closes or Bridge quits.** The page, its history and its sign-in are not written down, and the tab opens empty the next time Bridge opens.
- **It does not stop the person's presses while the agent drives.** The toggle decides whether the agent's acts are refused.
- **It draws no ring over the real page.** A note is a picture and a named part sent to the Job; only the mock's fixture page draws pins.
- **It grants the page nothing.** Downloads and every permission are refused, a popup opens in the page itself, and only `http` and `https` load.
- **It is not replayed.** An act asked while Bridge was not connected is refused, and one asked as Bridge quits is not answered, so the agent's wait runs out.
- **It is not the walk.** It writes no `window` artifact, holds no `walk:` item and waits on no Approve.
