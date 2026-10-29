# The capture window

**What it is:** the security review Bridge's posture requires before any window
loads another repository's running web app, and the rules that window is built
to. **Kind:** practice.

Read before building or changing the window Studio capture opens on a server
Run. Bridge's own window is not this window: `bridge.md`, Security posture,
still governs it, unchanged.

---

## What this window is

A person starts a declared server from a Studio, presses open on its Run node,
and gets a second window showing that server's page with Studio capture over
it. Notes land on the Studio the Run belongs to, fixed at capture as every Note
is.

**The page is untrusted, and that is not a judgement about the person's code.**
A dev server renders whatever the checkout and its dependency tree produce, so a
compromised dependency is the attacker this review is written against. Bridge
authored none of those bytes, which is the whole distinction `bridge.md` draws
around `nodeIntegration: false`.

## What it may load

> **Rule.** The window loads one origin — the origin of a link the Run
> published — fixed when the window opens and never changed afterwards.
> Why: an address that can change is an address that can be wrong, and there is
> no second place this window is meant to reach.

> **Rule.** The renderer names a server id and one of that server's links. Main
> resolves the address off the live holder Fleet published, and loads no string
> a click handler composed.
> Why: `apps/desktop/src/main/servers.ts` already holds that rule for opening a
> link in the system browser, and loading one in a window is strictly more.

> **Rule.** A Run's window: the host is `127.0.0.1`, `[::1]` or `localhost`, and the scheme is
> `http:` or `https:`. Any other address opens no window at all.
> Why: a Manifest may declare a server behind a public host or a tunnel, and
> this window is for a process on this machine. A port comes from the checkout's
> own span, `crates/fleet/src/ports.rs`.

> **Rule.** A Job's window (`show_window`) may load any `http` or `https`
> address, not only a loopback one, and is pinned to that address's origin. Its
> other protections hold: its own partition, no preload, navigation off the
> origin refused.
> Why: the Job chose the page to show the person, and there is no Run's
> link to resolve it against.

> **Rule.** A Job may pass `steps` with the page it shows (`show_window`), and the window plays
> them itself: a scripted walk of any page, in any repository, that knows nothing of Armada. Main
> reads them off the Job's own record, and they are Fleet's checked shape and no other
> (`session-file.ts`, `readSteps`). *A walk in the page* has how, and what that adds to this review.
> Why: the owner reviews a quick spike by walking it, and a Job in another repository (a web app
> on a local dev server, or a deployed staging address) had no way to give him the same walk.

> **Rule.** A Job's window may also pass through the origins the Job
> names in `origins` (`show_window`): `http` or `https`, a host and an optional
> port, no path. A top-level navigation, a redirect or a subframe to one of them
> is followed, and nothing else changes: the window is pinned to the shown
> origin and those, and every other address is refused as before. *Origins a
> Job names* has the review.
> Why: a deployed page behind single sign-on redirects to a provider and back,
> and a window that refuses the first hop cannot show the page at all.

> **Rule.** A Job's window may open one popup, and only onto the shown origin or a named one
> (`origins`). It is a child window of the walk's, in the same partition, with the same navigation hold
> and no preload; it closes when the page calls `window.close()` or the walk window closes. Every other
> popup is refused, as are a second one, one a popup opens, and any popup of a Run's or a Job's window.
> *A popup a Job's page opened* has the review.
> Why: a provider that signs in through a popup cannot be walked in a window that denies every
> popup, and the sign-in is the one thing the owner cannot be handed a redirect for.

> **Rule.** A Job's window is in a cookie jar of its own repository, keyed by
> the Job's `manifest_id`. A Job with none shares one unnamed jar.
> Why: a sign-in, the person's or the Job's, has to hold for the next walk,
> and a jar shared across repositories would carry one repository's login into
> another's page.

> **Rule.** A Job may pass `sign_in`: addresses the window loads in order in
> that jar before the page, each on the shown origin or a named one. They are a
> secret. Fleet holds them in memory until Bridge takes them once, the job
> shows another window, or the job ends, and main holds them until the walk's page first comes on screen, for
> the loads, and no longer. None is
> written to a row, the ledger, the timeline, a note, an event or a log. A load of one that fails is told to
> nobody, and the page's own failure look waits until they are done.
> Why: a magic link or a token address is a login. The failure notice carries
> the address it failed on, and would put the login into the Job's thread.
> It is held past a Bridge that is not connected, because a clock would drop it
> from a Bridge that connects late, and a secret that is only ever in memory and
> only until its job ends is no wider a thing to keep.

> **Rule.** A Bridge that connects puts in the walk window each live Job's
> newest shown page that has not ended, was not opened by this Bridge and was shown
> in the last 15 minutes (`REOPEN_WITHIN_MS`, `sessions.ts`), and takes the
> Job's sign-in then. A window the person closed has ended, and is not
> opened again by this. Once the walk window is up, every walk standing on a
> Job's waiting items joins it (*One window for every walk*).
> Why: a page shown while Bridge was offline for a few minutes had no window to
> open and no way to ask for its sign-in. A walk from before a restart is stale,
> and opening it on every launch would put old windows on the screen.

> **Rule.** A Job may show several takes of one change in its window
> (`variations`). The window is pinned to the origin of the take on screen, and
> that moves only on a press of a take on the bar, which names an index: main
> reads the address off the list the Job sent, so no address the page or the
> bar composed is loaded. A refusal, and the count of refused frames, belong to
> the page left and are cleared with it.
> Why: the takes are pages the Job chose to show, as its first address is,
> and swapping among them is no wider than showing them. Navigation by the page
> itself stays held to the origin on screen.

> **Rule.** One window per Run. Opening again raises the one that is open.
> A Job's walks and a pull request's are not Runs (*One window for every walk*).

> **Rule.** The Studio is the Run's own, decided when the window opens. Capture
> here has nothing to aim.

> **Rule.** A Job's server opens with no Studio, to be walked, and what is
> captured there lands on that Job as a walk note, kept by Fleet. The bar names
> the Job. A server with no Job still needs its Studio.
> Why: a Prototype Job stops at Build for a person to look, and the look
> belongs inside Bridge rather than the system browser. The owner, 4 Oct 2026:
> the notes he makes walking the work must reach the work — so they wait on the
> Job and go to its Drone with the next Request changes, never into a Studio
> nobody opened for it.

> **Rule.** The window takes ⌥⌘A as well as ⌥⌘C before the page does.
> Why: ⌥⌘A is the annotation chord a person already presses on Bridge. Left to
> a walked mock, it opened the mock's own layer, which wrote into a worktree
> nothing reads and that is thrown away with the Job.

> **Rule.** `webSecurity` stays on, and Bridge neither adds a header to the
> page's response nor removes one. A page's own CSP is not rewritten to make it
> render.
> Why: rewriting a header would be Bridge claiming to contain a document it
> cannot contain, and turning off same-origin policy inside a foreign page is
> the opposite of this review.

## How it is held there

| What moves | Held by | What happens |
|---|---|---|
| The first load | The address main resolved | Nothing else opens a window |
| Top-level navigation | `will-navigate` | Refused, address said, unless on a named origin |
| A server redirect | `will-redirect` | Refused, address said, unless on a named origin |
| A subframe | `will-frame-navigate` | Refused, counted on the bar, unless on a named origin |
| A popup or new window | `setWindowOpenHandler` | A Job's window opens one onto the shown origin or a named one; every other is denied, address said |
| A popup's own navigation | `will-navigate`, `will-redirect`, `will-frame-navigate` on the popup | Held to the same origins as the page, refused and said on the walk's bar otherwise |
| A download | `will-download` | Refused |

> **Rule.** A comparison is on the whole origin — scheme, host and port —
> against the origin the window opened on, or one a Job named. Nothing is
> matched by prefix, by suffix or by hostname alone.

> **Rule.** A refused address is drawn in full and never followed. One act
> offers it to the system browser, `http:` and `https:` only, on a person's
> press.
> Why: an app a person is building redirects to an auth provider, and the honest
> answer is the browser they already use rather than a second browser inside
> Armada. A Job that wants the provider in the window names it in `origins`,
> and then it is not refused.

> **Rule.** Subresources are the page's own business and are not pinned to the
> origin.
> Why: a window that blocked the app's own API and its fonts would draw a broken
> app, and a subresource cannot become a document with no rail. What the page
> could send anywhere is bounded by its holding nothing of Armada's.

> **Rule.** Capture is refused from the moment the Run stops serving, and the
> bar says the run ended.
> Why: a loopback port is not an identity. Anything on the machine may bind it
> once the server exits, and a Note captured from a replacement would be a
> record of something other than what it names.

> **Rule.** The window loads nothing further once the Run ends. What is on
> screen stays; reload and every navigation are refused.

> **Rule.** A Note captured here records the Run and the address beside
> everything a Note captured on Bridge records.

## The injected layer

**Narrowed once, deliberately, on 18 Sep 2026.** This section was reviewed
describing a layer that persists in the page: injected after the first load and
after every accepted navigation, adding document-level listeners while capture
was armed and removing them when it was not. *The layer runs inside the page*
was named as one of this design's three costs when the review was taken. **What
was built does not pay it**, and the two rules that described a persistent layer
are gone because nothing persists for them to be about. Every guarantee below is
the one that was reviewed, and *Refused outright*'s last rule binds this section
exactly as it did: putting a layer back into the page is a capability added to
this window, and that is another security review. `#1294`.

> **Rule.** The layer runs in the page's own world.
> Why: a component name is read off the `__reactFiber$` expando the page sets,
> and a contextIsolated world cannot see one —
> `apps/desktop/src/renderer/src/annotate/fiber.ts`.

> **Rule.** The source travels with each ask, and the page is left
> byte-identical: no global, no listener, nothing to remove, and nothing to
> inject again after a navigation.
> Why: the promise `executeJavaScript` answers on is then the page's only
> channel to main, and it exists only while main is holding it. A layer that
> persisted would be a thing to keep bounded; this is a thing with no lifetime
> to bound.

> **Rule.** The pointer is tracked by Bridge's own view over the page, never by
> the page.
> Why: the outline is drawn there anyway, by the two rules below it — nothing is
> written into the page, and what a person types never reaches it. So a listener
> in the page could only read what it had no way to send, which is what leaves
> the rule above nothing to trade away.

> **Rule.** It reads the page and writes nothing to it: no node, no attribute,
> no style.
> Why: an overlay drawn in the page is an overlay the page can read, restyle and
> forge — and a Note about how something looks, taken through it, would be a
> Note about Bridge's own box.

> **Rule.** It reads the fields a Note keeps and nothing wider: the shape in
> `packages/protocol/src/studio.ts`, over the declared property list in
> `apps/desktop/src/renderer/src/capture/note.ts`.

> **Rule.** It reads no cookie, no `localStorage`, no `sessionStorage` and no
> form value, and issues no request of its own.

> **Rule.** The markup a Note keeps carries no `value` attribute, and no
> contents of an `input`, a `textarea` or anything under a password type.
> Why: `outerHTML` over a form is a person's own typing, kept for the life of a
> Studio that nothing expires.

> **Rule.** What the person says is typed into Bridge's own surface, drawn above
> the page and outside it, and never reaches the page.
> Why: the page would otherwise read every note left on it. The surface a person
> types into is built from `packages/tokens` and shadcn, which a foreign
> document's stylesheet would fight.

> **Rule.** The capture card's note field takes the cursor once the card is placed.
> Why: the owner, 21 Oct 2026:
> "When I go to leave an annotation, the annotation window text box should be focused".

> **Rule.** The Studio's id, Fleet's port and every filesystem path stay in
> main. Nothing naming Armada's own records crosses into the page.

> **Rule.** The frame is taken by main, of this window's own contents, cropped
> to bounds clamped to the viewport.
> Why: main takes a Bridge Note's frame for the same reason — the capability is
> a Note with a picture, never a screenshot the page can ask for.

## What may come back

> **Rule.** One shape crosses, `StudioCapture`, and main re-applies every bound
> rather than trusting the page's: the declared style properties, the markup
> length, the text length, the selector length and each field's type.
> Why: the page can answer with anything, fifty megabytes of markup included,
> so the bound belongs on the side that is not the page.

> **Rule.** One armed press yields one capture, on a promise main created. The
> layer cannot send unasked.

> **Rule.** A page that forges a capture can put a Note on that Studio, beside
> the person's own words, and can do nothing else. That is the whole authority
> this channel carries.

> **Rule.** The Job browser's acts bring back four things and no more: the page's
> address and title, a snapshot of its accessibility tree, a screenshot written
> to a path Fleet named, and an expression's JSON result.
> Why: an agent's act is the one place a page's content is read on purpose, so
> each answer is bounded on Fleet's side. A snapshot is cut to 48 KiB and a
> result to 32 KiB, with `truncated: true`, and a screenshot is a file in the
> Job's scratch folder (`docs/practices/protocol/the-job-browser.md`).

> **Rule.** An agent's expression runs in an isolated world of the page's own,
> never in the world the finder's layer uses, and never in a Bridge document.
> Why: an isolated world shares the page's document and none of its scripts'
> globals, and its id (2001) is not the finder's (2000), so an expression cannot
> reach `ArmadaFinder`.

## The four protections

| Protection | This window | Bridge's window |
|---|---|---|
| `contextIsolation: true` | Unchanged | Unchanged |
| `nodeIntegration: false` | Unchanged | Unchanged |
| `sandbox: true` | Unchanged | Unchanged |
| `default-src 'self'` | Not applied — no Armada document | Unchanged |

> **Rule.** The CSP is not loosened, relaxed or widened. It is a `meta` in
> `apps/desktop/src/renderer/index.html`, which this window never loads, so the
> page is governed by whatever its own server sends and Bridge's promise about
> Bridge is untouched.

> **Rule.** What is loosened is none of the four: this window loads an address
> where Bridge's loads a file, and its navigation refusal is an origin
> comparison rather than a flat refusal.
> Why: that is the change this review exists to approve, and naming it as a CSP
> change would hide it behind a flag nobody moved.

> **Rule.** Bridge's own window keeps `loadFile`, denies every navigation and
> every window open, and its CSP still refuses `127.0.0.1`.

## What the page can reach back into

> **Rule.** The window is created with no preload. The page has no
> `ipcRenderer`, no exposed bridge, and no channel to main of its own.
> Why: every function on a preload is a capability that window can call, and the
> only surface that cannot be widened by accident is an absent one. See
> `apps/desktop/src/preload/index.ts` for the surface this window does not get.

| Reach | Answer | By |
|---|---|---|
| Armada's IPC | None | No preload on this window |
| Node, `require`, `fs` | None | `nodeIntegration: false`, `sandbox: true` |
| The Studio's records | None | No id, no path, no port in the page |
| A frame Fleet kept | None | `armada-frame:` is handled on the default job |
| Bridge's window | None | Never an opener: the only popup is the page's own, onto an origin the page may reach |
| Bridge's cookies and cache | None | Its own job partition |
| The filesystem | None | Downloads refused, `file:` never loaded |
| The screen | None | Fullscreen and pointer lock denied, and every permission but one |
| An app on this machine | Local network access, for the shown origin and the `origins` a Job names | `local-network-access` is the one permission granted, to those origins only |
| A dialog that looks like Armada's | None | `disableDialogs` |

> **Rule.** A Job's agent driving the Job browser adds nothing to this table. The
> page still has no preload, no IPC and no channel to main, every permission is
> refused (the Job browser names no origin to grant local network access to),
> and downloads stay refused.
> Why: the agent reaches into the page and nothing reaches back out. Each act is
> main's call, answered once, and the page cannot ask for one.

> **Rule.** A page runs in a job partition of its own, persistent, one
> per repository. A Job's walk is keyed by the Job's `manifest_id`.
> Why: an app under development needs a login, and a window that forgot it every
> time is a window used once. Bridge's own job holds none of it, and two
> repositories share nothing.

> **Rule.** Every permission request is denied without asking, and a certificate
> error is not overridden. The one exception is Chromium's Local Network Access
> (`local-network-access`), granted only when the requesting origin is the shown
> origin or one of the `origins` the Job passed to `show_window`. A Run's window
> names none, so for it that is its own loopback origin, where the check never
> fires. A popup the page opened gets the same rule. The person turns nothing
> on: a Job that passes the provider in `origins` is enough.
> Why: a single sign-on provider's page may verify the person through an app
> running on this machine, which it reaches over the local network. Refused, the
> sign-in fails, and the walk window has no site settings to allow it.

## What a person sees

> **Rule.** The window carries the OS frame and its title, where Bridge's own
> window is frameless and draws its own.
> Why: two windows that are not the same kind of thing do not have the same
> chrome.

> **Rule.** A bar Bridge owns sits above the page, loaded from disk with
> Bridge's preload and Bridge's CSP. It is at most two rows. A Run's bar names
> the Run, the address in full, and the Studio a Note will land on.
> Why: a person has to tell at a glance that what is below the bar is not
> Armada, and a bar composited above the page is a bar the page cannot draw
> over. The owner, 19 Oct 2026, of the walk bar: "consolidate all of this down
> to at most 2 bars".

The walk window's bar holds these, in this order:

| Row | Holds, in order |
|---|---|
| 1 | The walks dropdown, the takes dropdown (absent with no takes), the address of the walk on screen (muted and truncated, so the bar still names the address), Reload, Capture, Approve, Hide/Show Job |
| 2 | The walk's steps, with one compact status after the caption, the first that applies: run ended, address refused, a question with Show question, Loading, notes pending (*Pending notes*), or frames refused. A walk with no steps and no status draws no second row, and notes held on its Job are a status. Back leads the row and Next ends it, so no status or caption moves either |

The walks dropdown has the walk on screen selected.
Each entry carries its Job's name, the page's title, its address, its readiness and its ask marks, and who its notes go to as a hint.

> **Rule.** Reload and Capture are icons on the bar, and the keys below do the
> acts they stand for.
> Why: the owner, 19 Oct 2026, uses keys for Reload and Capture, so they take no
> more of the bar than an icon.

| Key | Does |
|---|---|
| ⌥⌘↓ | Opens the walks dropdown |
| ⌥⌘T | Opens the takes dropdown |
| ⌥⌘← and ⌥⌘→ | Step through the walks; past the last is the first |
| ⌘R | Reload |
| ⌥⌘A or ⌥⌘C | Capture |
| ⌘↵ | Approve, and not while a text field has the cursor. While the Job holds notes it asks first, as Approve does (*Pending notes*) |

> **Rule.** There is no address field, no Back, no Forward and no history.
> Reload reloads the pinned origin. A walk's Back is the walk's own, below, and not history.

> **Rule.** A refusal is said on the bar, naming the address that was refused.

> **Rule.** A Job's walk names its Job by the Job's own name, as the Jobs
> list does, never by the page's title. What opens a Job in Bridge's main window
> is read by main off the walk on screen, never off the bar.
> Why: a window shown with no title is titled by its address, and the bar said the address
> three times and named the Job by it.

> **Rule.** A window with no title of its own draws no name: the address is drawn in full in
> its place, since the full address holds the origin the window is pinned to. The OS title
> follows the same rule.

> **Rule.** The bar says whether the walk on screen is ready, and each entry of
> the walks dropdown says it for its own walk: the Job mark at its start and the
> bar's hue, and Approve reads Not ready, disabled, while the page is still
> changing. Main reads it off the Job's record and pushes it; nothing is asked
> of the page.
> Why: the owner approving a page its Job is still editing approves a page
> that is about to change under him.
> A Job that ends its turn with a subagent still at work on it is still editing: the page
> reads *Still changing* until a turn ends with none standing (`docs/concepts/session.md`,
> *Showing a window*).

> **Rule.** A page that is loading says *Loading* on the bar's status row, and the page's view is painted Bridge's ground, `--bg-base` from `packages/tokens/tokens.json`, until the page draws, never white.
> The ground is the dark theme's, since main is not told which theme the bar drew.
> Why: the owner, 21 Oct 2026:
> "I am getting a lot of white screens when walks are loading so I have no idea if its broken or if the walk is loading".

> **Rule.** A walk a Job opened offers Approve. It is a message from the
> person, `Approved: <address>`, which wakes it. **Of several
> takes** the takes dropdown names them, and Approve and every note
> name the one on screen (`Approved B, Needs you first: <address>`); approving
> one approves it alone, and another on screen offers Approve again.
> Why: the person walked the page and it is right, and the one who opened the
> window is the one waiting to hear it. A Studio's window has nobody to tell.

> **Rule.** A Job's walk that leaves the walk window is told to Fleet as
> `window_closed`, with the address it opened on, whether it left approved,
> because its waiting item was settled elsewhere, or with the window closed by
> whoever closed it (`left`, `walk-window.ts`). Closing settles nothing: the
> walk's waiting item stands until Approve.
> Why: it ends the Job's walk, and a Job refuses its own questions
> while one is open (`docs/concepts/session.md`, *Showing a window*).

> **Rule.** A page the window cannot show is told to whoever opened the walk:
> a Job as `window_failed`, which Fleet passes on as its own notice. A load that failed, an HTTP
> status of 400 or more on the page, a load still going after 30 seconds (`LOAD_MS`) or the page's process gone counts, and so
> does a loaded page that shows Vite's error overlay or draws nothing two looks
> running. Main looks every two seconds, never while the page is still
> changing, and the look answers one of three fixed words, never the page's
> text. A subframe's failure and a load this window refused do not count. Told
> once until the page is seen working: on the window's origin, under 400, and
> read fine. A Studio's window has nobody to tell, and a Run that ended is
> already known. `capture/failed.ts`.
> A page's own walk that stops on a step (`stopped`, *A walk in the page*) is told as well, once per step, as
> `step N of M: the walk in the page stopped on this step`.
> Those are fixed words and numbers main bounded, never the page's name or caption, because the text goes into the Job's prompt.
> A mock page with no walk by its name exposes a stopped walk, so it is told too. `capture/walk.ts`, `StoppedSteps`.
> Why: the person should not have to notice a dead page and carry the news
> back. Electron's events and main's own looks are read, never the page's word;
> the owner, 15 Oct 2026, chose the overlay and the blank page as well.
> The owner, 21 Oct 2026:
> "When a step in a walk fails to load, it is not notifying the job that the walk is broken. Same with when the walk fails to load".

> **Rule.** The window draws no rail and reaches two Armada surfaces: the card
> of a question its Job asked about the page, drawn on the bar, and the
> Job of the walk on screen, drawn in a panel beside the page.
> A Studio is read on Bridge's window, and a Studio's window has no panel.
> Why: the owner, 11 Oct 2026, "a question about the walk should open in the
> walk. In fact it should take me where the agent is asking me about. Highlight
> it like an annotation and show a card with the question and my options to
> answer. If the question is not related to the walk it should wait." And the
> same day: "When walks are open in the new window, I wish if they were
> connected to a session or a job that session or job somehow had a panel off
> to the side of the window. Going back and forth between the job and the
> walk is painful right now." Asked how, he chose a panel inside the walk
> window, over opening the walk inside Bridge or placing two windows side by
> side, knowing it is a capability added to this window and so another review;
> asked what it does, read and reply. *A question asked in the window* and *The
> panel beside the page* have what each adds.

## One window for every walk

Every walk a Job showed and every pull request's walk is in one OS window, oldest shown first.
The owner approved the mock walk `walkQueue`, 19 Oct 2026.

| Step | Held by | What happens |
|---|---|---|
| A walk comes on screen | `walk-window.ts` | Its page is made the first time. Only the walk on screen draws the bar and lays the views out; the bar and the panel are the window's |
| The window closes | Main | Every walk in it leaves; a Job's walk items stand, and come back with the next walk window |
| A key on the page | `before-input-event` | The keys in *What a person sees*, but ⌘↵, are taken from the page and handed to the bar. ⌘↵ reaches the page, and approves only when no field there has the cursor |

> **Rule.** Every Job's walk and every pull request review walk is in one window,
> in one queue, oldest first by when each was shown. There is one entry per walk:
> a Job with two walks is named twice. A Run's window and a Studio's window keep
> their own windows.
> Why: the owner, 19 Oct 2026, chose one window for every walk in place of one
> OS window per Job.

> **Rule.** The queue is read from Fleet's `walk:` waiting items (id `walk:<url>`,
> with its `since`), one per address a Job showed and the owner has not approved,
> and from Bridge main's own pull request review walks, which have no waiting
> item and are kept from when they are held (`queue.ts`). The window follows the
> items while it is up: a walk that stands joins it, and one whose item went
> leaves it unless its window reads working (next rule). A walk that stands opens no window (`windows.ts`).
> Why: only the owner's Approve settles a `walk:` item and closing the window
> leaves it standing, so Fleet's items already are the walks he has not approved.
> A list Bridge kept beside them would drift from them.

> **Rule.** A walk whose `walk:` item goes while its window reads working stays in the queue, on screen if it was, reading *Still changing* with Approve disabled.
> It leaves when its window reads ready with the item still gone.
> When the item comes back with a newer `since`, the walk is made afresh in its place (`windows.ts`, `follow`; `walk-window.ts`, `replace`).
> Why: Fleet drops the item while the Job works on the notes sent, and the walk going with it hid that the Job was.
> The owner, 21 Oct 2026:
> "When i send notes, it shouldn't immediately jump me to the next walk. There is a bit too much bouncing around."
> And: "Sometimes the walk disappears after i send notes so I can't actually tell that the job is working on it."

> **Rule.** The first walk opens the window behind, without taking focus, on that
> walk with every standing walk queued around it. A walk that arrives while the
> window is open joins the queue in its place, and never raises the window, takes
> focus or switches the page.
> Why: a person partway through one walk should not have the page change under
> him because another Job showed its own.

> **Rule.** Approve settles the walk on screen and moves to the next. Approving
> the last walk closes the window. A pull request's walk tells nobody, and its
> held walk closes.
> Why: the next walk is the one he reviews next.

> **Rule.** Each walk's page keeps its own partition, its own origin pin and its
> own walk step from when it first comes on screen until it leaves.
> The bar and the panel move between walks; the page never does.
> Why: the window is one, but each page is still another repository's untrusted app, and two pages share nothing.
> Returning to a walk he left should find it where he left it.

> **Rule.** Whether a field in the page has the cursor is asked in a world of its own, as the finder's is, and reads one
> word back. It writes nothing and leaves no listener.
> Why: ⌘↵ is a page's own Send in a field, and taking it there would break the page being walked.

## A question asked in the window

A Job's `ask_in_window` names a page on the origin of its window and a
selector for a place on it. `docs/concepts/session.md`, *Asking in the window*,
has the tool. This is what the window does with it, and what that adds to the
review above.

| Step | Held by | What happens |
|---|---|---|
| The question arrives | Main, off Fleet's row | The walk window comes forward on the walk asked about, opening on it if it is closed. **A walk not on screen is marked** on its entry and the page stays; going to it shows the question |
| The page moves | Main | `loadURL` of the question's address, **only on the pinned origin**, and not when the page is already there |
| The place is found | One ask, `apps/desktop/src/main/capture/locate.ts` | The element the selector finds, its box, and one scroll that brings it into view |
| It is drawn | The bar's view, over the whole frame | An outline at the box and the card beside it, as capture's are |
| It is answered | Main names the Job and the item | `answer_waiting`; the bar sends only an option's index and words |

> **Rule.** The locate ask is the injected layer's kind: its source travels with
> the ask, it leaves no global and no listener, and its answer is a box that
> main re-bounds against the viewport.
> Why: the page then holds nothing it did not hold before, which is what every
> rule under *What the page can reach back into* rests on.

> **Rule.** The one thing it moves is the page's scroll, once per question, as a
> person's own scroll would. It writes no node, no attribute and no style.
> Why: taking him where the agent is asking about is the decision. Scrolling is
> the least that does it, and the outline stays Bridge's.

> **Rule.** While the card is drawn over the page, the bar's view covers the
> frame and the page takes no press, as under armed capture. The card folds into
> the bar's second row, and the page takes presses again.
> Why: a card the page could draw over, or a press that reached the page through
> it, is the forgery *The injected layer* exists to rule out.

> **Rule.** The selector, the address and the Job's id stay in main. The bar
> gets the question, its options, who asks and a box.

**What it does not add.** No preload on the page, no channel from the page to
main, no address off the origin, and no Armada record named to the page. The
selector comes from the Job, not the page, and a selector that matches
nothing draws the card without an outline.

## A walk in the page

A page may be a walk in two ways: a mock that plays its own, or a page of any repository that a Job
gave `steps`. **A mock plays its own** with `WalkPlayer`: steps, a ringed target and a card, and a cursor and rings on a press as the bar draws them (`WalkPressMark`). When the
page offers a handle on `window.__armadaWalk` (`WalkHandle`, `packages/screens/src/fixtures/walk-window.tsx`),
the bar draws the walk's controls in a row of its own, and Back, Next, Autoplay and Replay act on the
page from there. **A page given steps plays none of it**: Bridge does, below, and a window that carries
steps does not look for the handle at all. The owner approved the mock, walk `walk-in-the-window`. This is what that adds to the
review above, and it is a capability added to this window.

| Step | Held by | What happens |
|---|---|---|
| The walk is found | Main, `apps/desktop/src/main/capture/walk.ts`, after `dom-ready` and `did-finish-load` | One script asks whether the handle exists, every 100 ms for five seconds, and every 250 ms while it answers |
| The page leaves its card | Main | One call, `hostCard(true)`, once per document |
| It moves | Main's next read | The page answers a string; main parses it, bounds it and tells the bar only of a change |
| A press | The bar names one of five acts | `next`, `back`, `replay`, `autoplay` on, `autoplay` off, each fixed text with nothing spliced in |
| Back | The page | It loads itself again on `?step=`, on the pinned origin, so `will-navigate` allows it. The row stays through the load |

> **Rule.** Main asks and the page never tells. The channel from the page is the promise
> `executeJavaScript` answers on, as a capture's and a question's are: no listener of Bridge's is left
> in the page, no `subscribe`, no `console` line read, no global added, and nothing to remove after a
> navigation.
> Why: a callback registered on the page's handle would be a thing the page holds and can fire at any
> time. A read main made is a thing main holds the promise of. The cost is up to 250 ms before the bar
> moves, and main reads once more 50 ms after an act so a press is not felt as lag.

> **Rule.** What the page says of its walk is bounded in main before the bar sees it. One string,
> at most 4096 characters, of a JSON object with eight fields: a name and a caption (strings, cut to 80
> and 240 characters and put on one line), `at` and `of` (whole numbers, `of` from 1 to 500, `at` from 1
> to `of`), `autoplay`, `ready` and `done` (booleans) and `stopped` (a boolean, absent read as false).
> Anything else is no walk, and a field the page adds is dropped. The bar draws it as text.
> `stopped` is a walk held on its step, and the window tells the Job of it (*What a person sees*, the rule on a page the window cannot show).
> Why: the page can answer with anything, and the bar is Bridge's own document.

> **Rule.** Main may call five things on the page's walk handle, and `hostCard(true)`. The bar sends
> an act's name, never a script, and main refuses an act where the page has no walk.
> Why: the handle is the page's own code, so what an act does is what the page already could do to
> itself. Nothing is run in the page that the page did not provide, and the act list is closed.

> **Rule.** A page that never answers is let go of after a second, and a read that is still out is not
> asked again.
> Why: a page stuck in a loop must not stack a script a beat behind it, or hold the bar's row.

**What it adds.** The page can tell main one bounded string about its own walk, and nothing else. Main
can call five acts and one card toggle on the page's own handle. **What it does not add:** no
preload on the page, no channel from the page that main did not open, no address the page can make main
load, and no Armada record named to the page. A page that forges a walk can draw a row of controls on
the bar and press its own Next, and can do nothing else.

### Steps Bridge plays

A Job's `steps` are the mock walks' own steps as JSON (`packages/protocol/src/sessions.ts`, `WalkStep`):
a `say` caption and one verb aimed at a target found by role and accessible name, or by its words.
`apps/desktop/src/main/capture/stepwalk.ts` plays them. The same bar row drives them as drives a
page's handle, through the same five acts, and the keys are the same. This is a capability added to
this window, and what it adds is below.

| Step | Held by | What happens |
|---|---|---|
| The steps arrive | Main, off the Job's `window` row or its ledger artifact | Re-read to Fleet's shape; anything else is no steps. The window is pinned to the origin it opened on, as always, and the steps are the window's and not a take's: another take starts them over |
| The walk begins | Main, on the first load of the shown page | Not a `sign_in` address's load, and not a provider's page the window passed through (`origins`): the steps wait for a load on the origin the window is pinned to, and the finder is asked of no other. The page is first looked at for a walk of its own, and the steps' first act waits on that |
| The page plays its own walk | Main, on reading `window.__armadaWalk` | The page's walk wins. The steps stop and are dropped, their ring and caption go, the page's card leaves to the bar, and the bar drives the page's walk. The Job is told once through `window_failed` that its steps were set aside. Two walks on one page drew the page's card beside the bar's steps, 28 Oct 2026; `armada-iam` moves every mock walk to the steps |
| The target is found | `finder.ts`, in an isolated world | `executeJavaScriptInIsolatedWorld`, a world of Bridge's own that shares the page's DOM and nothing else. `walk-find.ts` finds it, the code the mock's walks find theirs with. The page is scrolled to it, once, as a person's scroll would, and waited on until it holds still |
| It is ringed | The bar's view over the page | Main sends the target's box and the caption; the bar draws the ring and the card, as a question's outline is drawn (`WalkGuide`). The page draws nothing |
| The person presses Next | The bar names the act | Main sends real input to the page's contents (`sendInputEvent`): the pointer moves to the target's middle, goes down and up; keys for `type` and `key`; a drag with moves between. A press is shown first: main reports its point and a count in the walk's `guide.press`, the bar glides a cursor to it and grows two rings from it, and main waits `GLIDE_MS` before the page hears the press. The report is dropped after `PRESS_MS`, so a bar drawn afresh does not play it again, and Back and Replay run presses with none of it. Reduced motion draws no cursor |
| The target is not found | Main, after the patience a step has | The walk holds on the step. The bar marks it, and the Job is told once through `window_failed`, naming the step and its target |
| The page is signing in | Main, from the page's address and its popup | While the page is on a named origin, is heading to one, or has its popup open, the step is waited on with its patience held. The bar marks it with the signing-in glyph and a tooltip, the page takes presses, and the Job is told nothing. Back on the shown origin the same step is looked for again |
| Back, Replay, Reload | Main | The pinned address loads again and the steps before the one asked for run straight through, as the mock's Back does |

> **Rule.** The finder runs in an isolated world and nowhere else. The page's own world gets no global,
> no listener, no node and no attribute from it, and the walk writes nothing to the page: not a node, an
> attribute or a style. It moves two things, as a person does: the scroll that brings a target into view,
> and the focus of the field a `type`, `key` or `paste` step names.
> Why: this is *The injected layer*'s stance kept. A world the page's scripts cannot reach has nothing
> to forge, restyle or read, and the ring is drawn by the bar over the page for the reason capture's
> outline is.

> **Rule.** The finder's script is installed in that world on the first ask of each document, and asked
> by name after. It dies with the document, and nothing is injected again after a navigation but the
> one install on the next ask.
> Why: *The injected layer* ships its source with each ask because the promise it answers on must be the
> page's only channel. Here the page has no handle on the world at all, and the finder, with its
> accessible-name code, is too large to ship on every look at a ringed box that is followed every beat.

> **Rule.** What the page's DOM answers is a box or a point, re-bounded in main against the viewport,
> and nothing else. The target a Job named is serialised into the script as JSON, as every ask's
> arguments are, and no string of the Job's is composed into code.
> Why: the page can answer with anything, and a Job's target is text another party wrote.

> **Rule.** The walk acts with the real input a person makes, to the page's contents, and never by
> dispatching an event in the page. A paste is the editing command on a field the
> walk has focused, and so the clipboard as it stands; text outside the keyboard's range is inserted as text.
> Why: it has to work on any framework, and a page can tell an event it was handed from one a person
> made. A script that clicked for the Job would be a script the page could watch for.

> **Rule.** A step acts when the person presses Next, or on autoplay he switched on. Nothing acts
> unasked, and a hover is the only act on arrival.
> Why: the Job chose the page and the steps, and the person is the one walking it.

> **Rule.** Where the shown origin is not loopback (`127.0.0.1`, `[::1]`, `localhost`), Autoplay
> plays `look`, `hover` and `later` steps and stops before one that acts: `press`, `type`, `key`,
> `paste`, `drag` or `select`. Autoplay goes off there, the bar shows the Next key waiting, and the
> person presses Next to run the step. Autoplay pressed again goes on to the next step that acts.
> Back and Replay stop the same way: the steps before their target run straight
> through until one that acts, which waits for Next with the Next key waiting.
> Where the origin is loopback, Autoplay plays every step.
> Why: a deployed staging app has real buttons, and a Job's steps would press whichever one they
> name with nobody there. A local or mock walk has nothing to lose, so it feels as it did.

> **Rule.** While the ring is up the bar's view covers the page, as under armed capture, and the page
> takes no press. One press beside the card, or the Ring switch on the walk row, puts the ring away and
> the page takes presses again; the wheel is forwarded either way. A walk that is done, or held on a
> step, leaves the page to take presses.
> Why: a ring drawn where a press could land on the page is a forgery the page could read, and a walk
> the person cannot touch the page of is not one he can use.

> **Rule.** While the page is on a named origin the walk is in a signing-in state, not a held one.
> The state is the page's address (navigated to, redirected to, or about to be, off `will-navigate`
> and `will-redirect`) or its popup being open, and it ends when both are gone. A step held before the
> page left is looked for again on its return. The first load follows the same line as `beginsOn`:
> the steps begin on the shown origin.
> Why: a provider's page is not one the steps were written for, so a target missing there is not news.

> **Rule.** A step whose target is not found within the patience a step has, the mock's, is held on, and
> told to the Job through the path `window_failed` uses: once per step until it is found, in a line
> naming the step and its target, never the page's text. The person is shown the bar's glyph and its
> tooltip, no sentence.
> Why: the Job wrote the steps, so it is the one to fix them. The person is not asked to carry the
> news back.

**What it adds.** A Job's steps can make the person's own pointer and keyboard act on the page it
chose, one step at a time and on his press, with the cookies of the repository's partition. The page
can see only that a person acted. **What it does not add:** no preload on the page, no channel from the
page to main, no address the page can make main load, no global in the page's world, and no Armada
record named to the page. A step that follows a link off the origin is refused by the same hold as a
person's press. **It is not an allowlist**: a Job that shows a page it should not can have steps
press on it, which showing it already exposed him to.

## The panel beside the page

The walk window draws the Job of the walk on screen in a third view, on
the window's right edge at full height, and swaps it as the walk changes.
The Job panel is the Cockpit's own `SessionView` for that Job.
A Studio's window has no panel, and a pull request's walk has no Job, so the
panel is not drawn beside it.

> **Rule.** The panel draws the Cockpit's own `SessionView`, not a smaller view
> of a Job (`apps/desktop/src/renderer/src/walk-panel/WalkSession.tsx`).
> Why: the owner, 19 Oct 2026: "the same Job view as the Cockpit, not a
> stripped-down one". `SessionMini` and the `WalkPanel` component are deleted.

**The Cockpit never moves for the walk window: the panel is where the walk's Job is shown.**
The panel exists because of the owner, 11 Oct 2026: *"Right now I see a window with a walk in it with no context to the
job/session that walk stemmed from."*

| Step | Held by | What happens |
|---|---|---|
| The window opens | Main, `apps/desktop/src/main/capture/layout.ts` | A walk window gets a Job panel, never the other kind |
| A walk comes on screen or the window takes focus | Main, `apps/desktop/src/main/capture/walk-window.ts` | Swaps the panel's Job; Bridge's window is told nothing but to dim behind a focused one |
| The panel loads | `apps/desktop/src/main/capture/views.ts` | `walk-panel.html` from disk, with Bridge's preload, Bridge's CSP and the default job; its first subject is the query it is loaded with |
| It is told its Job | `walk-window.ts` | Main pushes the Job of the walk on screen to the panel (`walkPanel.subject`), and again when the walk changes |
| It reads | Main's publish | The state every Bridge window is sent, with no repository picked |
| It acts | Bridge's preload | A message, an answer, a decision, a merge or a stop, on the channels Bridge's window sends each on |
| It navigates | Bridge's preload | Nothing in the panel navigates: every act that would goes to Bridge's window through `walkPanel.openInBridge` or `walkPanel.goInBridge` |
| It folds | The bar | One press hides it and the page takes the width; the next window of its kind opens as this one was left, while Bridge runs |

> **Rule.** The panel is a Bridge document in a view of its own, never the
> page's: loaded from disk, Bridge's preload, Bridge's CSP, Bridge's default
> job. It navigates nowhere and opens no window, as the bar.
> Why: everything under *What the page can reach back into* rests on the page
> holding nothing of Armada's, and a panel in the page's view or the page's
> partition would end that.

> **Rule.** Main places the panel beside the page and never over it. Armed
> capture and a question's card cover the page's column, not the panel.
> An open menu or tooltip of the bar's covers the column as far down as it draws,
> and main puts the bar's view back to its strip once none is open.
> The bar says how far with `captureWindow.overhang`, a height or nothing, and main reads the window off the sender.
> While it covers, the page takes no press there.
> Why: the two views share no pixels, so a press on the panel cannot reach the
> page and nothing the page draws can sit over the panel; and the thread stays
> answerable while a question is drawn over the page.
> The bar is a document of its own at the strip's height, so a menu drawn inside it was cut at the strip's edge, under the page.

> **Rule.** The panel's preload is Bridge's whole surface, as the bar's is.
> These entries are answered for the walk window and no other document: the bar's
> `captureWindow.panel`, `captureWindow.walkTo` and `captureWindow.sendNotes`, and the panel's
> `walkPanel.openInBridge`, `walkPanel.subject`, `walkPanel.goInBridge`,
> `walkPanel.held`, `walkPanel.editNote`, `walkPanel.removeNote` and `walkPanel.sendNotes`
> (`apps/desktop/src/preload/index.ts`, `apps/desktop/src/main/studio-channels.ts`,
> `apps/desktop/src/main/held-notes-channels.ts`).
> None names the panel's Job. `panel` takes whether it is open, `walkTo` takes a
> place in the queue, and `openInBridge`, `subject`, `held` and both `sendNotes` take nothing: main reads
> the walk, and its Job, off its own queue and the view the call came from.
> `editNote` and `removeNote` take a note's id, and reach it only on that Job.
> `goInBridge` takes where a press in the Job view lands, one of five kinds that
> main validates (`panelGoOf`), and fills the Job in for every kind but
> `session`, which names another Job the panel's view opened.
> Why: a panel that named its subject could name another. The panel navigates
> nowhere itself, so each act that would is a request to Bridge's window, and
> main decides which Job it concerns.

> **Rule.** The Cockpit never moves for the walk window. A walk coming on
> screen, the window taking focus, a note saved and Approve moving on leave
> Bridge's open Job where it was. The window's `focused` hook tells Bridge's
> window only to dim (`walk-window.ts`, `WalkWindowBoard`), and its `shown` hook
> tells it nothing. The walk's Job is shown in the panel.
> Why: the owner, 19 Oct 2026: *"When I leave an annotation, it shouldnt take me
> anywhere."* Asked when the Cockpit should move to the walk's Job, he answered
> **Never**. This replaces his 11 Oct 2026 choice of one open Job that followed
> the walk in front of him.

> **Rule.** A Job panel reads only what Bridge's window reads for the Cockpit's
> `SessionView`, and wants nothing of its own: the held pool it is given has no
> want.
> Why: Bridge's window holds the same reads for the same Job, and a panel
> closing with its window would drop them from under that window.

> **Rule.** Nothing from the page reaches the panel but what capture already
> takes. A note taken here is held in main on the walk's Job (*Pending notes*), and
> main tells the panel the Job's held notes. Once sent, the panel shows the turn
> because it reads Fleet's record of it.

| Reach | The page | The panel |
|---|---|---|
| Armada's IPC | None, no preload | Bridge's preload, as the bar |
| Job partition | Its own, per repository | Bridge's default |
| CSP | Whatever its server sends | `default-src 'self'; img-src 'self' blob:` |
| The other view | None | None: no channel, no shared document, no shared origin |
| A press | Taken by the bar while covered | Its own, beside the page |

**What it does not add.** No preload on the page, no channel between the page
and the panel, no address the page can make the panel load, and no Armada
record named to the page. The page cannot read the panel: they are two
documents in two jobs on two origins, and main passes nothing from one to
the other. **The page's posture is unchanged by this section**; what changed
is that a second Bridge document sits beside it, with the bar's grant.

## Pending notes

**A note taken on a Job's walk is held, not sent.** The owner, 19 Oct 2026: *"I would love to be able to leave batch annotations on a walk and then submit them back to the agent all at once."*
He approved holding them, listing them, and sending them once.

| Step | Where | What happens |
|---|---|---|
| Save | Main, `apps/desktop/src/main/capture/window.ts` | The body `send_session_note` would carry, crop included, is held on the Job instead (`heldOn`). The bar says the note is pending, as the status rule below has it |
| Hold | Main, `apps/desktop/src/main/capture/held-notes.ts` | Kept per Job, each note with its own id and time, in `held-notes.json` under Bridge's `userData`, so a restart keeps them |
| List | The panel, `apps/desktop/src/renderer/src/walk-panel/HeldNotes.tsx` | The last entry of the Job's thread, a turn of the owner's that is not sent and grows as notes are taken: each note's crop, what it is on and its words. The words are edited in place and saved as the field is left; each note has a Remove. Send notes sits at its foot |
| Send notes | Main, `sendHeld` in `apps/desktop/src/main/held-notes-channels.ts` | Every note the Job holds goes as one turn through `send_session_notes` (`docs/practices/protocol/a-walks-notes-sent-as-one-turn.md`). The notes that were in the turn are dropped once Fleet takes it. A refusal keeps them all and the panel says why |
| Approve | The bar | While the Job holds notes, Approve and ⌘↵ ask first: send them and approve, approve without them, or go back |

> **Rule.** A held note belongs to the Job, not to the walk it was taken on.
> Approving without them, closing the window or restarting Bridge leaves them held.
> Why: the owner's notes are about the Job's work, and dropping one he has not sent loses it.

> **Rule.** A Job's walk, and a pull request's walk a Job holds, hold their notes on that Job.
> A Studio's capture and a pull request's walk no Job holds are unchanged: the first lands on the Studio, the second stays in Armada.
> A pull request's walk has no panel, so its held notes are listed on the Job's own walk, and the bar's ask is how they are sent from the review.

> **Rule.** The pending notes are the thread's trailing entry, drawn after its newest row and nowhere above the view.
> The panel passes them through `SessionView` to `SessionThread`'s `trailing` slot, which the Cockpit's own thread never fills, and the thread follows the entry as it grows, as it follows a new row.
> The turn is marked with `circle-dashed` and the tooltip `Pending`. When the notes are sent, the turn Fleet writes takes its place in the thread and nothing is drawn where they were.
> Why: the owner, 19 Oct 2026: *"I think this should show as the last item in the job thread. So it keeps building until I hit send on it. Instead of the top."*

> **Rule.** The bar says notes are pending in its status.
> With the Job panel hidden it reads *Note pending* or *Notes pending* with a Show note or Show notes button that opens the panel; with it open, *Notes pending in the Job panel*.
> Notes held take the queued bar's second row.
> Why: the owner, 21 Oct 2026:
> "When I have pending notes, and the job panel is hidden there should be a better way for me to see the pending notes instead of a little message in the upper right."

> **Rule.** Every send leaves through `sendHeld`, and nothing else sends a held note.
> A Job holding none sends nothing, since Fleet refuses an empty turn.

## What a Job's note carries

A note pinned on a Job's page goes to the Job whole, and not as one line of words.
Bridge's own layer sends it through `send_session_note`; the walk window holds it and sends it with the Job's other held notes through `send_session_notes` (*Pending notes*).
The page is a Job's mock walk, another repository's app, staging, or
Bridge's own layer (⌥⌘A). The operation's fields are in `crates/ipc/operations/send_session_note.toml`.

| Carried | Read from | Why the agent wants it |
|---|---|---|
| The words | What the person typed, trimmed | The thread keeps only these |
| The element | Tag, label and text (cut to 200 characters) | What he pointed at |
| The stamp | The closest `data-armada-source`, `path:line` | Where the mock's build says the element was drawn. Only the mock build stamps it |
| Nearby | Names of the enclosing landmarks, lists, sections and dialogs, and the nearest heading above, innermost first, at most 8 | A figure built from data has no words of its own; its list and heading do |
| Selected | Elements marked selected, current, pressed, checked, expanded or open, each as `role "name"`, at most 12 | The tab or option the page was on |
| Walk step and take | The walk's name, step and length; the take's letter and name | The state to reproduce |
| The crop | A PNG of the element, taken by main | What he saw, to confirm a candidate by |
| Component chain, selector, location, scenario | The capture, as every Note keeps it | Where else to look |

The stamp, nearby and selected are read in the same ask as the capture
(`apps/desktop/src/main/capture/layer.ts`), which writes nothing to the page, and main bounds them again
(`apps/desktop/src/main/capture/bounds.ts`, `readOf`). Bridge's own layer reads the same in
`apps/desktop/src/renderer/src/annotate/capture.ts` and sends the note's id with them, so
`mark_annotation_done` still finds the file. The window builds the body in `apps/desktop/src/main/capture/pinned-note.ts`,
the layer in `apps/desktop/src/shared/pinned-note.ts`. A note in a pull request's walk is held the same
way where a Job holds the pull request, its address the one the change runs at, and stays in Armada where none does.
A whole-window ⌥⌘A note stays a message: what Bridge held has no field in the body.

**Fleet locates before it delivers.** It searches the checkout that served the page for the words on screen,
numbers treated as wildcards, and ranks up to five `file:line` candidates, each with why it matched. A stamp is
believed only where its line, or the fifteen after it, holds a word from the screen; otherwise it is dropped,
and the delivered note says so. The Job reads the words first, then `Pointed at:`, `Candidates:`, `State:`,
`Page:` and `Inside:` lines, with the crop as a picture (`crates/fleet/src/session_host/noting.rs`).

**Where the search cannot reach.** The words on screen are the one anchor every page has, so these are what it
does not find:

| The page shows | The note gives |
|---|---|
| A figure built from data | The nearby labels and the component names; the number itself is no anchor |
| A canvas or an image | The crop and the place, since there are no words in the page to search |
| A native app | Nothing; the capture window loads a web page and a native app cannot be pinned |
| Words that live in more than one file | Several candidates; the crop and the owner's words choose |

## Refused outright

| Refused | Rather than |
|---|---|
| Any origin but the Run's, or the one a Job's window opened on and the ones that Job named for this window | An allowlist, a setting, or a trusted-host list |
| A non-loopback address a Manifest declares | Capture against staging or production |
| A non-http(s) address a Job shows | `file:`, `javascript:` or any scheme but the web's |
| Typing or pasting an address | A general browser inside Armada |
| Capture once the Run is not serving | Trusting a port to still be the Run's |
| Rewriting the page's CSP, or `webSecurity: false` | Making an awkward app render |
| Any preload capability on this window | One small exception, then a second |
| A page's `window.open` onto any other address, downloads and dialogs | Filtering them |
| Following a refused address in this window | A second browser nobody asked for |
| A popup onto an address the window may not reach, a second popup, a popup's popup, and any popup of a Run's or a Job's window | Letting a page open a window of its own |
| A `sign_in` address off the shown origin and the named ones | A Job loading a page it did not let the window reach |

> **Rule.** A capability added to this window later is another security review,
> on the same terms as the four protections.
> Why: every guarantee above rests on the page holding nothing and reaching
> nothing, which one addition ends.

## Origins a Job names

`show_window`'s `origins` widens where a Job's window may go, by named origins and nothing wider. This is the review of that allowance, and it is a capability added to this window.

| Question | Answer |
|---|---|
| Who names an origin | The Job, in its call. Never the page, the bar or a redirect |
| What may be named | `http` or `https`, a host and an optional port. Fleet refuses a path, query, fragment, sign-in of its own or another scheme with 422 and writes nothing. Main drops what is more than an origin again (`passThrough`), so a ledger read back is held to it too |
| What it allows | A top-level navigation, a redirect or a subframe to that origin, compared on scheme, host and port whole |
| What it does not allow | Any other origin. A redirect off the named ones is refused and said on the bar, as before |
| Where main still runs a script | The shown origin only. Capture, a question's locate, the failure look and a walk's handle all wait for the page to be on it, so a provider's page is never asked anything |
| What the named page reaches | What the shown page does: no preload, no channel to main, every permission denied but local network access, downloads refused, popups held as *A popup a Job's page opened* says, `webSecurity` on. It holds its own origin's cookies in the repository's jar and no other's |
| What the Job can do with it | Reach a page it could already show with `show_window`. Naming an origin makes no page the window loads trusted; it lets a redirect there complete |

> **Rule.** A named origin is allowed to be navigated to and to reach this
> machine's local network, and nothing more.
> Why: the allowance is the redirect to a provider and back, and the provider's
> check with an app on this machine on the way. Everything the
> window does to a page for the person's sake (capture, a question, a look,
> a walk) stays on the origin the Job showed, so the injected layer's
> guarantees are the ones already reviewed.

> **Rule.** A `sign_in` address is loaded only if it is on the shown origin or a
> named one, checked by Fleet when it is named and by main when it is loaded.
> Why: a sign-in is a page load with a login in its address, and it should reach
> no further than the window does.

> **Rule.** A walk whose page is on a named origin waits there. The steps hold on the step being
> looked for in a signing-in state, the bar draws a glyph with a tooltip and not the not-found one,
> and the Job is told nothing. Back on the shown origin the same step is looked for again, with
> its whole patience. A page that has its popup open is in the same state.
> The signing-in glyph wins over Autoplay's waiting key: no Next key waits while the page is off the
> shown origin, and Autoplay's stop before an acting step resumes with the step.
> Why: a step that sends the page to a provider has a target the provider's page cannot hold, and
> reading that as missing told the Job to fix steps that were right. The person's time at a
> provider is not the step's patience. The state is read off the page's address and the popup, which
> main already knows, and nothing is asked of the page.

> **Rule.** The capture partitions' user agent is Electron's with the `Electron/x.y.z` token and the
> app's own name token removed (`chromeAgent`, `address.ts`), for the page and its popup. Bridge's own
> views keep Electron's.
> Why: a provider that refuses a browser it can place as Electron reads Chrome instead. The agent is
> a header and a string the page reads, and the page learns nothing of Armada from it. It makes the
> page no more trusted and the window no more capable; a page that reads it learns only less about
> where it runs.

**Costs.** A bot check that reads more than the user agent still refuses this window, and the
owner signs in on the provider's page in a browser first. A popup that opens `about:blank` and
navigates it afterwards is refused, since its first address is on no origin.

## A popup a Job's page opened

A provider's sign-in often opens a popup and posts its answer back to the page. This is the review of
letting that one window open, and it is a capability added to this window.

| Question | Answer |
|---|---|
| Whose window | A Job's. A Run's refuses every popup, as before |
| Which address | The popup's own, `HandlerDetails.url`, on the shown origin or a named one, compared whole. Anything else is refused, said on the bar |
| How many | One. A second `window.open` while it is open is refused, and so is one the popup makes |
| What it is | A child window of the walk's around the contents Electron made for it (`createWindow`), in the page's partition, sandboxed, `contextIsolation` on, `nodeIntegration` off, no preload |
| What holds it | The page's own hold: `will-navigate`, `will-redirect` and `will-frame-navigate` against the same origins. A download, a dialog and every permission but local network access meet the partition's refusals; that one is granted on the same origins as the page's |
| What it can reach | The page that opened it, as any popup reaches its opener, and the repository's cookie jar. Nothing of Bridge's: it is not Bridge's window's opener, and the page has no preload for it to find |
| When it closes | The page calls `window.close()`, the person closes it, or the walk window closes |
| What a failure does | Nothing reaches main uncaught. A popup that cannot be built is closed and refused as every popup was |

> **Rule.** The popup is built on the contents Electron hands `createWindow`, never on a view of
> Bridge's own making.
> Why: Electron requires the child window to be connected to the contents it created, and a popup
> that is not is an uncaught exception in main. The preferences are the opener's, which are the
> page's: no preload and sandboxed.

> **Rule.** The popup runs no script of Bridge's. Capture, a question's locate, the failure look and
> the walk's finder stay on the page, and the page's own `window.opener` is the only line between the two.
> Why: the popup is a provider's page, and everything the window does to a page for the person's sake
> stays on the origin the Job showed.

**What it adds.** One more document that can exist beside the page, on an origin the Job already
let the window reach, able to talk to the page that opened it. **What it does not add:** no origin
the window could not already navigate to, no preload, no channel to main, no run of Bridge's script
in it, and no popup for a Run's page or a Job's. A page that opens a popup onto its own origin gets a
window of itself, which it could already draw.

## What this does not claim

**The origin is not an identity.** Loopback is not a trust boundary between
processes: whatever binds the port owns the origin, and Armada cannot prove the
page came from the Run. What bounds it is that capture ends with the Run and the
window stops loading — not a check that would read as proof.

**A page hostile to its own data is not contained.** The window holds nothing of
Armada's, and it holds everything the page already had. Nothing here protects
the person's app from its own dependency tree.

**A Note keeps what was on screen.** Markup, styles and a frame taken over a
logged-in app carry that app's data into a Studio nothing expires, on the
person's own machine.

**Fleet's loopback surface was reachable from any browser on this machine, and
is not now.** `#1460` refuses any request or WebSocket upgrade carrying an
`Origin` header, which is what a browser puts on one and a page cannot forge —
`docs/contracts/system-architecture.md`, *Fleet answers no page in a browser*.
This window neither widened that reach nor narrowed it; closing it was Fleet's
work, and it is done. The page loaded here is still a page, so it is refused by
the same rule as any other.
