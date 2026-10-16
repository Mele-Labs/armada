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

> **Rule.** A Session's window (`show_window`) may load any `http` or `https`
> address, not only a loopback one, and is pinned to that address's origin. Its
> other protections hold: its own partition, no preload, navigation off the
> origin refused.
> Why: the Session chose the page to show the person, and there is no Run's
> link to resolve it against.

> **Rule.** A Session's window may also pass through the origins the Session
> names in `origins` (`show_window`): `http` or `https`, a host and an optional
> port, no path. A top-level navigation, a redirect or a subframe to one of them
> is followed, and nothing else changes: the window is pinned to the shown
> origin and those, and every other address is refused as before. *Origins a
> Session names* has the review.
> Why: a deployed page behind single sign-on redirects to a provider and back,
> and a window that refuses the first hop cannot show the page at all.

> **Rule.** A Session's window is in a cookie jar of its own repository, keyed by
> the Session's `manifest_id`. A Session with none shares one unnamed jar.
> Why: a sign-in, the person's or the Session's, has to hold for the next walk,
> and a jar shared across repositories would carry one repository's login into
> another's page.

> **Rule.** A Session may pass `sign_in`: addresses the window loads in order in
> that jar before the page, each on the shown origin or a named one. They are a
> secret. Fleet holds them in memory until Bridge takes them once, main holds
> them for the loads and no longer, and none is written to a row, the ledger, the
> timeline, a note, an event or a log. A load of one that fails is told to
> nobody, and the page's own failure look waits until they are done.
> Why: a magic link or a token address is a login. The failure notice carries
> the address it failed on, and would put the login into the Session's thread.

> **Rule.** A Session may show several takes of one change in its window
> (`variations`). The window is pinned to the origin of the take on screen, and
> that moves only on a press of a take on the bar, which names an index: main
> reads the address off the list the Session sent, so no address the page or the
> bar composed is loaded. A refusal, and the count of refused frames, belong to
> the page left and are cleared with it.
> Why: the takes are pages the Session chose to show, as its first address is,
> and swapping among them is no wider than showing them. Navigation by the page
> itself stays held to the origin on screen.

> **Rule.** One window per Run. Opening again raises the one that is open.

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
| A popup or new window | `setWindowOpenHandler` | Denied, address said |
| A download | `will-download` | Refused |

> **Rule.** A comparison is on the whole origin — scheme, host and port —
> against the origin the window opened on, or one a Session named. Nothing is
> matched by prefix, by suffix or by hostname alone.

> **Rule.** A refused address is drawn in full and never followed. One act
> offers it to the system browser, `http:` and `https:` only, on a person's
> press.
> Why: an app a person is building redirects to an auth provider, and the honest
> answer is the browser they already use rather than a second browser inside
> Armada. A Session that wants the provider in the window names it in `origins`,
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
| A frame Fleet kept | None | `armada-frame:` is handled on the default session |
| Bridge's window | None | Every window open denied, so no opener |
| Bridge's cookies and cache | None | Its own session partition |
| The filesystem | None | Downloads refused, `file:` never loaded |
| The screen | None | Fullscreen, pointer lock and every permission denied |
| A dialog that looks like Armada's | None | `disableDialogs` |

> **Rule.** The window runs in a session partition of its own, persistent, one
> per repository. A Session's window is keyed by the Session's `manifest_id`.
> Why: an app under development needs a login, and a window that forgot it every
> time is a window used once. Bridge's own session holds none of it, and two
> repositories share nothing.

> **Rule.** Every permission request is denied without asking, and a certificate
> error is not overridden.

## What a person sees

> **Rule.** The window carries the OS frame and its title, where Bridge's own
> window is frameless and draws its own.
> Why: two windows that are not the same kind of thing do not have the same
> chrome.

> **Rule.** A bar Bridge owns sits above the page, loaded from disk with
> Bridge's preload and Bridge's CSP, naming the Run, the address in full, and
> the Studio a Note will land on.
> Why: a person has to tell at a glance that what is below the bar is not
> Armada, and a bar composited above the page is a bar the page cannot draw
> over.

> **Rule.** There is no address field, no Back, no Forward and no history.
> Reload reloads the pinned origin. A walk's Back is the walk's own, below, and not history.

> **Rule.** A refusal is said on the bar, naming the address that was refused.

> **Rule.** A Session's window names the Session by the Session's own name, as the Sessions
> list does, never by the window's title. On a Job's or a Session's window that name opens it in
> Bridge's main window, and main reads which one off the window, never off the bar.
> Why: a window shown with no title is titled by its address, and the bar said the address
> three times and named the Session by it.

> **Rule.** A window with no title of its own draws no name: the address is drawn in full in
> its place, since the full address holds the origin the window is pinned to. The OS title
> follows the same rule.

> **Rule.** The bar says whether the page is ready: the Session mark at its
> start and the bar's hue, and Approve reads Not ready, disabled, while the page
> is still changing. Main reads it off the Session's record or the Job's status
> and pushes it; nothing is asked of the page.
> Why: the owner approving a page its Session is still editing approves a page
> that is about to change under him.

> **Rule.** A window a Job or a Session opened offers Approve. On a Job it is
> the act JobDetail's Approve sends; on a Session it is a message from the
> person, `Approved: <address>`, which wakes it. Nothing closes. **Of several
> takes** the bar draws a row of them under its own, and Approve and every note
> name the one on screen (`Approved B, Needs you first: <address>`); approving
> one approves it alone, and another on screen offers Approve again.
> Why: the person walked the page and it is right, and the one who opened the
> window is the one waiting to hear it. A Studio's window has nobody to tell.

> **Rule.** A Session's window that closes, whoever closed it, is told to
> Fleet as `window_closed` with the address it opened on.
> Why: it ends the Session's walk, and a Session refuses its own questions
> while one is open (`docs/concepts/session.md`, *Showing a window*).

> **Rule.** A page the window cannot show is told to whoever opened the walk:
> a Session as `window_failed`, which Fleet passes on as its own notice, and a
> Job as `walk_failed`, which wakes its Commander. A load that failed, an HTTP
> status of 400 or more on the page, or the page's process gone counts, and so
> does a loaded page that shows Vite's error overlay or draws nothing two looks
> running. Main looks every two seconds, never while the page is still
> changing, and the look answers one of three fixed words, never the page's
> text. A subframe's failure and a load this window refused do not count. Told
> once until the page is seen working: on the window's origin, under 400, and
> read fine. A Studio's window has nobody to tell, and a Run that ended is
> already known. `capture/failed.ts`.
> Why: the person should not have to notice a dead page and carry the news
> back. Electron's events and main's own looks are read, never the page's word;
> the owner, 15 Oct 2026, chose the overlay and the blank page as well.

> **Rule.** The window draws no rail and reaches two Armada surfaces: the card
> of a question its Session asked about the page, drawn on the bar, and the
> Session or the Job that opened the window, drawn in a panel beside the page.
> A Studio is read on Bridge's window, and a Studio's window has no panel.
> Why: the owner, 11 Oct 2026, "a question about the walk should open in the
> walk. In fact it should take me where the agent is asking me about. Highlight
> it like an annotation and show a card with the question and my options to
> answer. If the question is not related to the walk it should wait." And the
> same day: "When walks are open in the new window, I wish if they were
> connected to a session or a job that session or job somehow had a panel off
> to the side of the window. Going back and forth between the session and the
> walk is painful right now." Asked how, he chose a panel inside the walk
> window, over opening the walk inside Bridge or placing two windows side by
> side, knowing it is a capability added to this window and so another review;
> asked what it does, read and reply. *A question asked in the window* and *The
> panel beside the page* have what each adds.

## A question asked in the window

A Session's `ask_in_window` names a page on the origin of its window and a
selector for a place on it. `docs/concepts/session.md`, *Asking in the window*,
has the tool. This is what the window does with it, and what that adds to the
review above.

| Step | Held by | What happens |
|---|---|---|
| The question arrives | Main, off Fleet's row | The window comes forward, opening on the address the Session showed if it is closed |
| The page moves | Main | `loadURL` of the question's address, **only on the pinned origin**, and not when the page is already there |
| The place is found | One ask, `apps/desktop/src/main/capture/locate.ts` | The element the selector finds, its box, and one scroll that brings it into view |
| It is drawn | The bar's view, over the whole frame | An outline at the box and the card beside it, as capture's are |
| It is answered | Main names the Session and the item | `answer_waiting`; the bar sends only an option's index and words |

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

> **Rule.** The selector, the address and the Session's id stay in main. The bar
> gets the question, its options, who asks and a box.

**What it does not add.** No preload on the page, no channel from the page to
main, no address off the origin, and no Armada record named to the page. The
selector comes from the Session, not the page, and a selector that matches
nothing draws the card without an outline.

## A walk in the page

A page may be a walk: a mock played by `WalkPlayer`, with steps, a ringed target and a card. When the
page offers a handle on `window.__armadaWalk` (`WalkHandle`, `packages/surfaces/jobs/src/fake/walk-window.tsx`),
the bar draws the walk's controls in a row of its own, and Back, Next, Autoplay and Replay act on the
page from there. The owner approved the mock, walk `walk-in-the-window`. This is what that adds to the
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
> at most 4096 characters, of a JSON object with seven fields: a name and a caption (strings, cut to 80
> and 240 characters and put on one line), `at` and `of` (whole numbers, `of` from 1 to 500, `at` from 1
> to `of`) and `autoplay`, `ready` and `done` (booleans). Anything else is no walk, and a field the page
> adds is dropped. The bar draws it as text.
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

## The panel beside the page

A window a Session or a Job opened draws that Session or Job in a third view, on
the window's right edge at full height. A Session's panel is its thread, the
cards it waits on and the box to answer it. A Job's is JobDetail's own reading of
it, live: its state, its brief, and the verdict at the gate with its evidence,
its pull request, the comments on it, Merge, and the decision with this walk's
notes — or a redirect to a Drone still working. A Studio's window has no panel.

**Focusing a Job's walk window makes its Job Bridge's open Job.** The owner,
11 Oct 2026: *"Right now I see a window with a walk in it with no context to the
job/session that walk stemmed from."*

| Step | Held by | What happens |
|---|---|---|
| The window opens | Main, `apps/desktop/src/main/capture/layout.ts` | A Session's window gets a Session panel and a Job's a Job panel, never the other kind |
| A Job's window opens or takes focus | Main, `apps/desktop/src/main/capture/follow.ts` | Main watches the Job and tells Bridge's window to open it, raising nothing |
| The panel loads | `apps/desktop/src/main/capture/views.ts` | `walk-panel.html` from disk, with Bridge's preload, Bridge's CSP and the default session; its subject is the query it is loaded with |
| It reads | Main's publish | The state every Bridge window is sent, with no repository picked; a Job panel draws Bridge's open Job, its claims and its comments from it |
| It acts | Bridge's preload | A message, an answer, a decision, a merge or a redirect, on the channels Bridge's window sends each on |
| It folds | The bar | One press hides it and the page takes the width; the next window of its kind opens as this one was left, while Bridge runs |

> **Rule.** The panel is a Bridge document in a view of its own, never the
> page's: loaded from disk, Bridge's preload, Bridge's CSP, Bridge's default
> session. It navigates nowhere and opens no window, as the bar.
> Why: everything under *What the page can reach back into* rests on the page
> holding nothing of Armada's, and a panel in the page's view or the page's
> partition would end that.

> **Rule.** Main places the panel beside the page and never over it. Armed
> capture and a question's card cover the page's column, not the panel.
> Why: the two views share no pixels, so a press on the panel cannot reach the
> page and nothing the page draws can sit over the panel; and the thread stays
> answerable while a question is drawn over the page.

> **Rule.** The panel's preload is Bridge's whole surface, as the bar's is.
> Two entries are answered for a walk window and no other document: the bar's
> `captureWindow.panel` and the panel's `walkPanel.openInBridge`. Neither takes
> an id. Main reads the Job or the Session off the view the call came from.
> Why: a panel that named its subject could name another.

> **Rule.** A Job's walk window opening or taking focus makes its Job Bridge's
> open Job. Main watches it, then sends Bridge's window a summons marked
> `follow`, which opens the Job without raising the window. With the composer
> up, Bridge's window keeps it and opens nothing. A Studio's window and a
> Session's move nothing.
> Why: the owner chose one open Job that follows the walk in front of him over
> a second open Job per window (11 Oct 2026). The summons keeps Bridge's own
> open Job the same one, so main never swaps the watch out from under it.

> **Rule.** A Job panel reads only what Bridge's window reads: the open Job,
> its claims and its pull request's comments. It asks for the claims and the
> comments by Bridge's own calls, and never lets either go.
> Why: Bridge's window holds the same reads for the same Job, and a panel
> closing with its window would drop them from under that window.

> **Rule.** Nothing from the page reaches the panel but what capture already
> sends. A note taken here reaches the Session as a message, or waits on the
> Job as a walk note, and the panel shows it because it reads Fleet's record of
> it. Saving a walk note re-reads the open Job, which is this Job.

| Reach | The page | The panel |
|---|---|---|
| Armada's IPC | None, no preload | Bridge's preload, as the bar |
| Session partition | Its own, per repository | Bridge's default |
| CSP | Whatever its server sends | `default-src 'self'; img-src 'self' blob:` |
| The other view | None | None: no channel, no shared document, no shared origin |
| A press | Taken by the bar while covered | Its own, beside the page |

**What it does not add.** No preload on the page, no channel between the page
and the panel, no address the page can make the panel load, and no Armada
record named to the page. The page cannot read the panel: they are two
documents in two sessions on two origins, and main passes nothing from one to
the other. **The page's posture is unchanged by this section**; what changed
is that a second Bridge document sits beside it, with the bar's grant.

## Refused outright

| Refused | Rather than |
|---|---|
| Any origin but the Run's, or the one a Session's window opened on and the ones that Session named for this window | An allowlist, a setting, or a trusted-host list |
| A non-loopback address a Manifest declares | Capture against staging or production |
| A non-http(s) address a Session shows | `file:`, `javascript:` or any scheme but the web's |
| Typing or pasting an address | A general browser inside Armada |
| Capture once the Run is not serving | Trusting a port to still be the Run's |
| Rewriting the page's CSP, or `webSecurity: false` | Making an awkward app render |
| Any preload capability on this window | One small exception, then a second |
| A page's `window.open`, downloads and dialogs | Filtering them |
| Following a refused address in this window | A second browser nobody asked for |
| A popup, even a sign-in provider's | Letting a page open a window of its own |
| A `sign_in` address off the shown origin and the named ones | A Session loading a page it did not let the window reach |

> **Rule.** A capability added to this window later is another security review,
> on the same terms as the four protections.
> Why: every guarantee above rests on the page holding nothing and reaching
> nothing, which one addition ends.

## Origins a Session names

`show_window`'s `origins` widens where a Session's window may go, by named origins and nothing wider. This is the review of that allowance, and it is a capability added to this window.

| Question | Answer |
|---|---|
| Who names an origin | The Session, in its call. Never the page, the bar or a redirect |
| What may be named | `http` or `https`, a host and an optional port. Fleet refuses a path, query, fragment, sign-in of its own or another scheme with 422 and writes nothing. Main drops what is more than an origin again (`passThrough`), so a ledger read back is held to it too |
| What it allows | A top-level navigation, a redirect or a subframe to that origin, compared on scheme, host and port whole |
| What it does not allow | Any other origin. A redirect off the named ones is refused and said on the bar, as before |
| Where main still runs a script | The shown origin only. Capture, a question's locate, the failure look and a walk's handle all wait for the page to be on it, so a provider's page is never asked anything |
| What the named page reaches | What the shown page does: no preload, no channel to main, every permission denied, downloads and popups refused, `webSecurity` on. It holds its own origin's cookies in the repository's jar and no other's |
| What the Session can do with it | Reach a page it could already show with `show_window`. Naming an origin makes no page the window loads trusted; it lets a redirect there complete |

> **Rule.** A named origin is allowed to be navigated to and nothing more.
> Why: the allowance is the redirect to a provider and back. Everything the
> window does to a page for the person's sake (capture, a question, a look,
> a walk) stays on the origin the Session showed, so the injected layer's
> guarantees are the ones already reviewed.

> **Rule.** A `sign_in` address is loaded only if it is on the shown origin or a
> named one, checked by Fleet when it is named and by main when it is loaded.
> Why: a sign-in is a page load with a login in its address, and it should reach
> no further than the window does.

**Costs.** A provider that signs in through a popup, or one that refuses a
browser whose user agent says Electron, cannot be walked in this window. The
redirect form works, and the owner signs in on the provider's page once. A
Bridge that is not connected when the page is shown opens no window, and
`sign_in` is lost with the call.

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
