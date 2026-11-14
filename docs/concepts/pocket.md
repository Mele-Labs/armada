# Pocket

**What it is:** The owner's phone app, a Home Screen web app named Armada, and the Phone Gateway each Mac runs inside Fleet for it, which stands in front of Fleet's own routes so they stay on loopback with no auth.

---

**Kind:** Surface.

Work stops while you are away from the Mac.
The phone buzzes with the Job's title when the Job needs you, you tap it, and you see the Job's thread.
You answer its question, approve its plan or send it a message from there, on any of your Macs.

## What it is

```
Phone (PWA) ──https──▶ the owner's tunnel ──▶ Fleet's process
                                           ├─ Phone Gateway 127.0.0.1:8443 ──http──▶ Fleet 127.0.0.1:<port>
                                           └─ Web Push (Apple / Google) ──▶ Phone
                      cloudflared (Armada's own) ──outbound to Cloudflare──▶ the Mac's address
Bridge (Settings → Phone) ──loopback /admin/*──▶ Phone Gateway
Bridge (Settings → Phone) ──GET / POST /phone──▶ Fleet
```

| Part | Where | Does |
|---|---|---|
| Phone app | `apps/pocket`, built to its `dist` folder | Shows what needs the owner on every paired Mac, signs each call with that Mac's key, receives pushes |
| Phone Gateway | `crates/pocket`, started by Fleet in `crates/armada/src/pocketing.rs` | Serves the app, checks signatures, passes the phone's calls to Fleet, sends pushes |
| Fleet | `crates/api`, `crates/fleet` | Listens on loopback, takes no auth, refuses any request carrying `Origin`. The Gateway calls it over HTTP with none, and with `x-armada-caller: phone`, so a phone act is recorded with the `phone` door ([Retro](retro.md)). A linked Mac's act goes with `mac` and the Mac's id in `x-armada-mac`, the door `mac` |

**The Gateway runs in Fleet's process, on a listener of its own.** It works while Bridge is closed, and stops when Fleet stops.
It still calls Fleet over HTTP rather than in process, so every phone call meets Fleet's router and its `phone` door.

Setup is in [Set up the phone app](../journeys/set-up-the-phone-app.md).

## The way in

**The owner brings the tunnel.** Armada runs no relay and no service of its own; whatever reaches `127.0.0.1:8443` over https is the way in, and the choices are in [Set up the phone app](../journeys/set-up-the-phone-app.md).

| The address is | When |
|---|---|
| The line in `pocket-address` in the machine directory | The file is there. One `https://` origin, no path; anything else refuses pairing with a sentence naming the file. Setting up the Cloudflare tunnel writes it |
| `https://<tailnet name>` from `tailscale status --json` | The file is not there |

It is read at each ask, in `pocket::machine_address`, so an edit needs no restart.

**Bridge walks the owner to Funnel.** Settings, Phone draws a row for each step, from these two routes in `crates/pocket/src/setup.rs`, and reads them again every 5 s until Funnel is on.

| Route | Does |
|---|---|
| `GET /admin/setup` | `{ tailscale, served, named, address?, cloudflare, cloudflared? }`: Tailscale `missing`, `signed_out` or `running`; in front of the Gateway `none`, `tailnet` (`tailscale serve`) or `funnel`; whether `pocket-address` names the address; the tunnel's step, below; and the version of `cloudflared` Armada keeps |
| `POST /admin/setup/funnel` | Runs `tailscale funnel --bg` in front of the Gateway. 204 once on; `{ approve }`, the tailnet's page to allow Funnel on, while the command waits for it; 502 with Tailscale's words otherwise |
| `POST /admin/setup/cloudflare`, no body | Sets the tunnel up as far as it is not: 204 once the tunnel is made; `{ login }`, the page to sign in on, while it waits for the owner; 409 while a press is under way, 502 with the reason and what to press next otherwise |

The shapes are `GatewaySetup`, `CloudflareState`, `StartCloudflare` and `CloudflareSignIn` in `crates/ipc/src/phone.rs`.

## The Cloudflare tunnel

**Each Mac can have its own address on the owner's Cloudflare domain**, such as `studio.example.com`, with nothing installed by hand and no terminal.
`cloudflared` holds an outbound connection to Cloudflare on port 7844 and hands whatever arrives at the address to the Gateway on loopback, so a Mac behind a VPN that takes no calls in still has a way in.
Armada hosts nothing: the tunnel, its address and the domain are in the owner's account.

One press does the whole setup, from Settings, with nothing typed.
The owner picks the domain on Cloudflare's sign-in page, and Armada reads it from the sign-in.

| `cloudflare.state` | Means | The row offers |
|---|---|---|
| `missing` | Armada has not downloaded `cloudflared` | The press, which downloads it first |
| `signed_out` | Not signed in to Cloudflare on this Mac | The press, which opens the sign-in |
| `signing_in` `{ login }` | Waiting for the owner to sign in and pick the domain | A button that opens `login` |
| `no_tunnel` `{ why? }` | Signed in, no tunnel yet; `why` is the last press's failure and what to press next | The press |
| `running` `{ address }` | Connected to Cloudflare, serving the Gateway at `address` | Nothing to do |
| `down` `{ address, since, why }` | Made but not connected since `since`, in `cloudflared`'s own words where it said | Nothing to do: Fleet starts it again |

- **The download is kept only when two checks pass.** The archive's SHA-256 must be the digest its release publishes, and the binary must be signed by Cloudflare's Developer ID (team `68WVV388M8`), which does not trust the forge the release sits on.
  `crates/pocket/src/cloudflared.rs`.
- **Armada keeps it current.** A running tunnel asks once a day whether a newer `cloudflared` is out, installs it under the same two checks, and restarts on it.
- **The sign-in opens from Bridge's button, once.** `cloudflared tunnel login` runs with no way to open a browser itself, and its page is `login`.
- **Armada's sign-in is its own.** Every `cloudflared` runs with its home in `cloudflared/home` in the machine directory, so the owner's own `~/.cloudflared` and any tunnel he runs himself are never touched.
- **The domain comes from the sign-in.** The certificate `cloudflared tunnel login` writes names the zone the owner picked and holds a token that may read it.
  Armada asks Cloudflare for that zone's name, and the token is read for that one call and kept nowhere else.
  A certificate that names no zone, or one Cloudflare will not answer for, has the next press open the sign-in again.
- **The tunnel is named for this Mac's id** (`armada-<id>`), so two Macs that share a name never take each other's.
  The address is this Mac's name under the domain, and is pointed at the tunnel without overwriting, so a name already used for something else on the domain is refused with Cloudflare's words.
- **It runs while the Gateway runs.** Fleet starts it with the Gateway and stops it with it, starts a `cloudflared` that stopped again after a rest that grows to a minute, and asks it every 5 s whether it is connected.
- **Funnel stays.** The tunnel writes `pocket-address`, so new pairings get its address; a phone paired through Funnel keeps reaching the Gateway while Funnel is on.

A tunnel held open for hours is unmeasured: the 12 Nov tests served one page each way for 3 minutes.
If a network closes 7844, armada-hu76.15 is the fallback.

## This Mac and the Devices list

**Each Mac has an id minted once, a name and one colour**, kept in the store's `pocket_this_mac`.
The name is the host's name when the id was minted, and the colour one of `blue`, `teal`, `green` or `orange`, drawn at random.

| Route | Answers |
|---|---|
| `GET /admin/macs` | `[Mac]`: this Mac first, then each paired Mac in the order it was paired, then each Mac whose claim waits for Confirm here. `{ id, name, colour, address?, this_mac, build?, link?, last_heard_at?, why?, code? }`. `code` is the four digits both Macs show while an ask waits. `build` is the commit Fleet's kept build was made from. `link` is absent on this Mac |
| `GET /admin/devices` | `[PairedPhone]`: `{ id, name, created_at, last_seen_at }` |

Settings draws the two as one Devices list.
**Every state of another Mac carries its one press, and `why` says where it is when it is not here.**

| `link` | Means | The press | `why` |
|---|---|---|---|
| `linked` | Both Macs hold a key for the other, and the link is open | Forget | absent |
| `asleep { since }` | The link dropped at `since` and is being opened again | Forget | The failure, such as `Work stopped answering.` |
| `claim_waiting` | It pasted this Mac's link | Confirm, here | `Press Confirm to pair Work with this Mac` |
| `confirm_waiting` | This Mac pasted its link | Confirm, on the other Mac | `Confirm on Home` |
| `older_build` | Its build speaks another wire and was built before this one's, or says no wire at all | Update, which moves it onto this Mac's build | `Work runs an older build. Nothing crosses until it updates.` |
| `newer_build` | Its build speaks another wire and was built after this one's | Update, which moves this Mac onto its build | `Work runs a newer build. Nothing crosses until this Mac updates.` |

## Turning it on

The opt-in is the file `pocket` in Armada's machine directory, `~/Library/Application Support/Armada/`.

| Route on Fleet | Does |
|---|---|
| `GET /phone` | `{ on, port, listening, why? }`: whether the Mac is opted in, the loopback port, whether the Gateway listens, and why one that is on does not |
| `POST /phone` `{ on }` | Writes or removes the marker, and starts or stops the Gateway at once. Answers as `GET /phone` |

- **Fleet starts the Gateway at each start while the marker is there.** One that cannot bind or open the store logs one line, and Fleet carries on without it.
- **A start that fails is not a refusal.** The marker stays, `why` names the cause, and the next start tries again.
- **The app is served from `pocket-app` in the machine directory**, which `scripts/restart` copies on an opted-in Mac, else from the `dist` build of `apps/pocket` under the folder Fleet runs in.
  Neither there answers 404.

## What keeps it closed

| Control | How |
|---|---|
| Loopback bind | `pocket::bind` and `pocket::bind_now` take a port and no host, so no setting opens the Gateway to a network. The tunnel the owner points at it is the only way in for the phone |
| Wi-Fi door | A second listener on every network the Mac is on, for his other Macs nearby: TLS, and only `/pair/mac/*` and `/api/mac/*`, so `/admin/*`, the phone's routes and the app are 404 there. See [Macs nearby](#macs-nearby) |
| Allowlist | `router` in `crates/pocket/src/routes.rs` names every route. Anything else is 404, and so is a listed path asked with an unlisted method |
| `/admin/*` | For Bridge's main process. Refused (403) when the request carries `Origin`, which means a web page sent it, or a header `tailscale serve`, Funnel, ngrok or `cloudflared` adds, or a `Host` other than loopback, which means it came through a tunnel, Armada's own included |
| Signed `/api/*` | The phone sends `X-Pocket-Device`, `X-Pocket-Time` and `X-Pocket-Signature`: ECDSA P-256 over the method, path and query, time, and the body's SHA-256. The private key is made on the phone, one per Mac, and never leaves it |
| Cross-origin | `/api/*` and `POST /pair` answer a preflight and name the caller's origin when it is `https`, so the app one Mac served may call another. The signature is still the only auth. `/admin/*` never answers one |
| Clock | A request more than 60 seconds from the Mac's clock is refused |
| Replay | A signature seen inside that window is refused the second time |
| Pairing | A code lives five minutes and claims once. `/pair` answers ten requests a minute, wrong codes included. A claim is not a device until the owner confirms it in Bridge. `POST /admin/pair/decline { code }` burns the code at once, and the phone that claimed it is answered 403, `This Mac declined pairing with this phone.`, on every signed call until the code would have lapsed, so its wait says so rather than spinning |
| Subscriptions | A push endpoint must be `https`, so a phone cannot make the Mac post to loopback |
| Shown files | A file is read only where the Job showed it with `show_file`, or kept it as a revision of its document. Fleet refuses any other path with 404 |
| Shown video | `GET /shown/<ticket>` is the one route outside the signed layer. A ticket stands in for the signature, is bound to the device that asked, and names only a path Fleet vouched for: see below |

**A refused signature says which failure it is**, so the app can tell this Mac does not know the phone from a request that went wrong.

| Status | Means |
|---|---|
| 401 | This phone is not paired with this Mac. The app marks this Mac alone as not paired |
| 400 | No signature, a time that is not a number, a clock off by more than a minute, or a signature that does not match |
| 409 | The same signature was sent before |

**The video address is unsigned on purpose.** A `<video>` element sends no custom header, so it cannot sign a request.
The phone signs the ask instead: `GET /api/sessions/shown/address` is an ordinary signed route, which Fleet answers only for a video the Job showed, and the Gateway answers `{ address: "/shown/<ticket>", media_type }`.

| Part | Holds |
|---|---|
| The ticket | 32 random bytes, kept in memory only |
| Its life | 15 minutes from minting, and at most 64 are live, the oldest dropped first |
| Its device | Bound to the device that asked. Every request is refused unless that device is still paired, so unpairing a phone ends its streams |
| Its path | None of its own: the ticket names the path Fleet vouched for, and the request carries no path |
| A restart | The Gateway restarting ends every ticket |
| Anything else | A bare 404: unknown, expired, an unpaired device, or a file gone since minting |

**The answer is `Cache-Control: no-store` and `Referrer-Policy: no-referrer`, and the bytes are read from disk in ranges, never held whole.**
The phone asks once more for an address when one lapses, and the video keeps its place.

## What the phone sees

| Screen | Holds | Route |
|---|---|---|
| Jobs | Every live Job, drawn as native's Jobs list: a heading per project, and in it Needs you first, then working, held and done. Each row has its lamp, state glyph, last turn and, with more than one Mac, its Mac's dot. A Job another started folds under its starter, which carries a glyph per state while folded. A scope narrows the list to Needs you, Active, Paused, Drafts or All, and a search to titles | `GET /api/sessions` |
| A row's second line | A waiting Job says what holds it, since a phone has no hover. A done Job says its summary and has an End button. Any other says its pull request, issue, workflow step and what it is doing now. Only Needs you rows pulse | `GET /api/sessions` |
| A row's state | Fleet's `mark` where the Gateway sends one, else what the phone reads off the row's fields. Without a mark a reply reads as idle, since the phone keeps no record of what the owner read | `GET /api/sessions` |
| A Job | Its whole thread, as Bridge reads it | `GET /api/sessions/one?session_id=` |
| A file | A file the Job showed: its text, or a picture, drawn in the thread | `GET /api/sessions/shown?session_id=&path=` |
| A video | A video the Job showed, played in the thread, with its place kept | `GET /api/sessions/shown/address?session_id=&path=`, signed, then `GET /shown/<ticket>`, unsigned |
| A document | A revision the Job kept of its page, as text | `GET /api/sessions/shown?session_id=&path=`, `path` one its record's `revisions` names |
| This Mac | The name its dot's tooltip and the menu give it: the first label of its address, or its host name | `GET /api/machine` |

> **Rule (12 Nov).** The phone sees only Jobs in a project he shares, and Jobs held from another Mac, which only a shared project sends.
> A Job in an unshared project, or in none, is answered on every route as a Job that does not exist, and is in no list, stream or push.
> Why: he called keeping private projects private "absolutely critical", and a phone is a paired device like his other Macs.

**Fleet rules on every route the Gateway calls as the phone, by name** (`crates/api/src/phone_share.rs`, with the rulings in `crates/fleet/src/share.rs`).
A route asked about a Job the phone does not see is asked with an id no Job has, so it answers in its own words for a missing Job.
The Jobs list, the repositories and the projects are cut to what the phone sees, `/events` drops every other Job's change, and a route with no ruling is 404.
A Job the phone starts goes into the first shared project of this Mac holding its repository, which the Gateway names, and Fleet refuses one into any other.

**Waiting follows Bridge's order.** A Job is not waiting while it is paused, failed to start, waits on another Job, sits on its start card or is held by the plan's limit, whatever it owes.
Otherwise an ask, a slot choice (unless Fleet is creating it), the delivery question, a plan, a sketch, a document or the done ask on its tasks makes it wait through a running turn; any other item waits for the turn to end.

**The lists are narrowed.** `crates/pocket/src/phone_sessions.rs` builds `PhoneSession` field by field, so a field Fleet adds to the Job record does not reach a list until someone writes it there.
A row carries what native's Jobs-list row reads: its mark, project, last turn, pull request, issue, workflow step and what it is doing now.

**A Job's thread is not narrowed.** `/api/sessions/one` is Fleet's `SessionThread` as Fleet sends it, the Job record included.

| The phone gets | The phone never gets |
|---|---|
| A Job's whole thread and its record, its folder on the Mac among it | A walk's window: its row says to open it on the Mac |
| A file the Job showed: text up to 1 MiB, or a picture's bytes up to 8 MiB | A file the Job did not show, or text over 1 MiB or a picture over 8 MiB (413) |
| A video the Job showed, of any size, in ranges from the Mac's disk | A video the Job did not show, or one through an address that has lapsed or whose phone is unpaired |
| A revision the Job kept of its document, up to 1 MiB, as text | The page in the worktree, or a revision another Job kept |
| A sketch the Job drew, from the scene its record carries | A terminal the phone opens, resizes or stops: it follows the ones the Mac opened |
| The terminals the Mac opened in a Job, what they printed so far and what they print next, and keys typed into them | A run Fleet has ended and forgotten (422), or keys the Gateway could not deliver, which are never queued |

**`/api/sessions/shown`** is Fleet's `get_shown_file`, which reads only a path the Job's record holds as an artifact of form `file` it showed, or one a `document` artifact's `revisions` names.
**The phone asks for a revision by its kept path**, as the record names it, oldest first, so a page has no route of its own.
A picture (`png`, `jpg`, `jpeg`, `gif`, `webp`, `heic`, `svg`) comes as its bytes under its type, up to 8 MiB, since a retina screenshot often passes 1 MiB and whatever `show_file` accepts should open.
A video (`mp4`, `mov`, `webm`) is never read: it comes as `{ path, media_type, bytes }`, whatever its size, and plays through the address above.
Anything else must be UTF-8 and comes as `{ path, text }`, up to 1 MiB.

## What the phone can do

Each reply is passed to Fleet's route of the same name without `/api`, its body as the phone sent it, and Fleet's status and body come back as they are.
The phone sends Fleet's own body shapes.

| Phone route | Fleet route |
|---|---|
| `POST /api/sessions/ask/answer` | `answer_session_ask` |
| `POST /api/sessions/waiting/answer` | the answer to an item the Job waits on |
| `POST /api/sessions/plan/approve` | `approve_plan` |
| `POST /api/sessions/plan/changes` | `send_plan_changes` |
| `POST /api/sessions/sketch/answer` | `answer_sketch` |
| `POST /api/sessions/document/answer` | `answer_document` |
| `POST /api/sessions/message` | a message to the Job |
| `POST /api/sessions/change_waiting` | `change_waiting`, for when a waiting Job starts and whether it is approved |
| `POST /api/sessions/start_waiting` | `start_waiting_now` |
| `GET /api/queued`, `POST /api/queued/take_back` | `list_queued` and `take_back_queued`, as Fleet answers them |
| `GET /api/sessions/terminals?session_id=` | `list_terminals`, answered as Fleet's `JobTerminals`: each terminal's `terminal`, `name`, `run_id` and whether a program is `running` in it |
| `POST /api/sessions/terminal/write`, body `{run_id, data}` | `write_session_run`, Fleet's status and body as it said them: 422 for a run that has ended |
| `GET /api/sessions/terminal/observe?run_id=` | `observe_session_run` as NDJSON, one `SessionRunMessage` a line and an empty line on a quiet beat, as `/api/live` |

**A terminal is followed and typed into, never opened.**
The phone lists the terminals the Mac opened in a Job, follows one from what it printed so far to `exited`, and writes keys to it as they are (Ctrl-C is the byte 3).
Nothing is queued: a key the Gateway cannot hand to Fleet is refused at once, and a stream Fleet ends or drops ends.

**A Job that waits on another opens to its waiting screen**, in place of its thread and message box.
It holds what Bridge's waiting pane holds: the brief, the Job it waits on, the condition as a select, what it needs, where it starts from, Approve and Start now.
The workflow, model and effort it starts on stay on the Mac.
The items it holds are answered under those facts, since a waiting Job does not reach Needs you.

**What a Job waits on is answered beside its thread**, each item only as Fleet takes it.
`delivery`, `slot_choice`, `task_done`, `rule_act` and `browser_ask` take a choice alone, `watch_not_ours` a choice or words, `words_address` an address or words holding `{match}`, and the agent's own items a choice or words.
A `slot_choice`'s slots to free stay on the Mac.
A `sketch` and a `document` are answered from their cards in the thread, Approve or Send back with a note, and a `draft` the same from its card.
A proposed workflow change is a draft whose card also offers Dismiss, the second option Fleet lists for it.
A document also opens at its own page, read by the same reader Bridge uses, with Approve and Send back under it.
A `held_child` is an ask that opens the Job it names, which opens as its start card, and a `fleet_call` is an ask that dismisses it.
A `walk` and a `window_ask` are a line at the thread's end saying to open it on the Mac, with no answer box, since each points at a page the phone cannot show.

**A 409 is read by its code.** Fleet's `fleet.sketch_redrawn` and `fleet.document_revised` say the page changed since he read it.
The phone reads the Job again, shows the new drawing or revision and says the Job changed it, and an answer refused with any other code shows Fleet's own message.
The Gateway's own 409 for a signature sent before carries no such code.

**`POST /api/sessions/answer`** is the older, checked answer, kept for the app's Needs you rows:

- **A Terminal Job carries an ask only while Fleet holds its question**: from `asked_at` for `holding_for_seconds`, by the Gateway's clock.
  After that the answer is refused with 409 and the question is answered in the terminal.
- **Whether a Job is hosted is read from Fleet's own row** when the answer arrives, never from the phone.
- **`allow_and_remember` is never offered.** It writes the owner's personal settings, so the answer type has no such value.

**One-line dispatch.** The phone sends a text and a repository label to `POST /api/sessions`.
The Gateway maps the label to Fleet's manifest id, refuses one it does not know or two repositories share, and starts a Job with `POST /sessions/start` carrying the text as `request`.
It answers 201 with the Job.

**Dispatch to the Operator.** `POST /api/sessions/dispatch` takes `{text, repository, kind?, mac?}` and nothing else, and sends Fleet's `POST /sessions/dispatch` as Bridge's composer does: the text as the request and as the first message, `kind` as the hint.
`mac` names another Mac by the name the phone knows it by, or its id, and is left out for the Mac the call is made to.
The phone calls the chosen Mac's own Gateway with no `mac` where it answers, and another Mac's with `mac` set only where the chosen Mac is asleep, so the send waits.
For another Mac the Gateway also names the shared project that Mac holds, since the phone knows no project, and answers 409 where the repository is in none.
It answers 200 with what the dispatch came to (`session`, `operator` or `parked`, the Job trimmed as the lists trim it), or 202 with the `Queued` act where that Mac does not answer and the send waits for it.
What the Operator then did with a held dispatch comes on `/api/live`.

**`/api/live`** is Fleet's `/events` narrowed to `session.changed`, written as `{change: "session", session_id}`, to `operator.dispatched`, written as `{change: "operator", dispatched}`, to `queued.changed`, written as `{change: "queued"}` for the phone to read `/api/queued` again, and `{change: "resync"}` where the phone's lists may be behind.
The app keeps one per Mac and reads again on a session change or a resync.
While Fleet is quiet it writes a `: beat` comment line every 15 seconds, so the phone can tell a waiting stream from a dead one.

**`/api/sessions` answers the live Jobs and takes no `state`.** The phone has no list of ended ones.

**The app's files** are sent with `Content-Encoding: br` or `gzip` when the request accepts it and a `.br` or `.gz` copy sits beside the file.
Anything under `/assets/` is hashed by the build and cached for a year, `immutable`.
Every other file, `index.html` and `sw.js` included, is `no-cache`.

## Several Macs

**The phone talks to each Mac directly.** There is no relay.
It is paired once per Mac, with a key of its own for each, and shows every Mac's Jobs together, each row with its Mac's dot where there is more than one.
The app is served by whichever Mac the owner opened it from, and calls the others across origins.

A 401 from one Mac marks that Mac as not paired, and the others carry on.

## Another Mac

**Two of the owner's Macs pair once, with a link and a Confirm, and then each Gateway calls the other's.**
Each hands what it receives to its own Fleet over loopback, and each Fleet keeps the other's rows beside its own.
What crosses has passed the share filter of the Fleet that holds the Job: `/other_mac/*` on Fleet hands out nothing else.

**Pairing.** Home shows the link and work pastes it.

| Step | Route | What happens |
|---|---|---|
| 1. Home makes a link | `POST /admin/macs/pair/start` | `MacPairLink { link, expires_at }`, the link `<home address>/pair/mac?code=<code>`. The code is single use and lives five minutes. No address answers 503 with why |
| 2. Work claims it | `POST /admin/macs/pair/claim { link }` | Work needs an address of its own, else 503 with why. It makes a P-256 key, keeps the private half in the Keychain, mints a back code and posts `{ code, mac, spki_hex, back_code }` to home's `POST /pair/mac`, unsigned and rate-limited with `/pair`. Home holds the claim in memory and answers 202 with itself. Work keeps home as `confirm_waiting` and answers it as a `Mac` |
| 3. Home lists the claim | `GET /admin/macs` | Work, `claim_waiting` |
| 4. Home confirms | `POST /admin/macs/pair/confirm { mac }` | Home makes its own key, Keychain again, and posts `{ back_code, mac, spki_hex }` to work's `POST /pair/mac/back`. Work takes it only with the back code it minted for that Mac, and links. Home links on a 2xx. If work does not answer, home keeps the claim and answers 502, `Work did not answer. Press Confirm again.` |
| 5. Either forgets | `POST /admin/macs/forget { mac }` | A signed `POST /api/mac/forget` to the other, as far as it answers, then the row and the key go here and Fleet is told with `POST /other_macs/forget`. The other side does the same on its own. Forget on a claim waiting here drops the claim |

**Pairing without a link.** Both Macs are on his Cloudflare account or on the same network, so Settings lists the other and he presses Pair.

| Step | Route | What happens |
|---|---|---|
| 0. Set up | Fleet's `POST /phone/setup`, no body | One press for the whole setup: the Gateway on, as `set_phone` turns it on, then the tunnel's press in the background. Answers `MacSetup { step, working, login?, why?, address? }`, and `GET /phone/setup` reads it with the Gateway off. `step` is `gateway`, `sign_in`, `tunnel` or `done`. `working` has nothing to press, `login` is the page to sign in on, and `why` is the setup to press again. A tunnel that is made but not connected is `working` for 30 seconds before it says why |
| 1. Work lists his other Macs | `GET /admin/macs/found` | `FoundMacs { macs, why? }`. The `armada-*` tunnels on the account other than this Mac's, from Cloudflare's tunnel list and the domain's records, read again after 4 seconds, then each Mac Bonjour sees nearby. Each `FoundMac { tunnel, name, colour?, address?, online, paired, build?, nearby }`: `online` is the tunnel connected or the Mac seen nearby, `nearby` is Bonjour seeing it now, and `name`, `colour` and `build` are what the Mac said on `GET /pair/mac/hello`, unsigned, which answers those three and nothing else. An asleep Mac keeps the name it was last read with, else its paired row's, else its address's first label. An account that cannot be read answers `why` with what was read before |
| 2. Work asks | `POST /admin/macs/pair/ask { tunnel }` | At home's Wi-Fi door while Bonjour sees it, else at its tunnel. Work makes its key and back code as for a claim, and a random half it keeps, and posts `{ mac, spki_hex, back_code, commit }` to home's `POST /pair/mac/ask`, unsigned, `commit` the SHA-256 of its half. Home answers 202 with itself and its own random half, `nonce`, and holds the ask unlisted. Work refuses a Mac that answers with an id that is not its tunnel name's, or, at a door, that names another certificate than the one it showed |
| 2b. Work reveals | `POST /pair/mac/reveal { mac, reveal }` | Home takes the half only where it hashes to `commit`, and lists the ask. Each side draws the same four digits from work's key, both doors' certificates and both halves. Work keeps home as `confirm_waiting` with `code` |
| 3. Home lists the ask | `GET /admin/macs` | Work, `claim_waiting`, with the same `code` and `why` `Work wants to pair. Confirm if it shows 4821.` |
| 4. Home confirms or declines | `POST /admin/macs/pair/confirm { mac }` or `POST /admin/macs/forget { mac }` | Confirm is step 4 above. Decline drops the ask and posts `{ mac, back_code }` to work's `POST /pair/mac/declined`, which forgets home's row and key on that code alone |

**Work commits to its half before it sees home's**, so a Mac between them cannot try halves until the digits match: it gets one guess in ten thousand.

**Home's address is public, so an ask is held three ways.** `/pair/mac/ask`, `/pair/mac/reveal` and `/pair/mac/declined` count against the same minute's share as `/pair`.
At most three asks wait at once, and a fourth answers 429; a Mac asking again replaces its own.
An ask nobody confirms is gone from both sides after five minutes, as a link is.

**A private key never leaves the Mac that made it.** Each side makes its own and hands over only the public half, so the one link and one Confirm give a key each way.
A link is `https`, or `http` only to a loopback host, so two Gateways in a test can pair.
A claim stays until it is confirmed or forgotten, or the Gateway restarts.
While it waits it calls him: the Gateway hands Fleet every claim waiting with `hold_mac_claims` each time they change, and Fleet stands one `mac_pairing` call for each ([What calls the person](../practices/protocol/a-jobs-calls.md)).

**Signing.** A Mac signs exactly as the phone does, with `x-pocket-device` its own minted id.
The signed layer finds the device among the phones, then among the linked Macs.

| Caller | May call | Elsewhere |
|---|---|---|
| A paired phone | Every signed `/api/*` route but `/api/mac/*` | 404 |
| A linked Mac | Only `/api/mac/*` | 404 |

The wrong side gets 404 and never 401, so it reads as a route that is not there.
**This line keeps the other Mac off the phone's routes**, which show Jobs held from a third Mac that the other Mac was never sent.

| Route, for a linked Mac | Answers |
|---|---|
| `GET /api/mac/hello` | This Mac: `{ id, name, colour, address?, build?, wire?, built_at?, source? }`. `wire` is the protocol ID its build speaks |
| `GET /api/mac/sessions` | Fleet's `GET /other_mac/sessions`, a `SessionList` |
| `GET /api/mac/bug_reports` | Fleet's `GET /other_mac/bug_reports`, `BugReports` |
| `GET /api/mac/thread?session_id=&before=` | Fleet's `GET /other_mac/thread`, a `JobThread` |
| `POST /api/mac/act` | Fleet's `POST /other_mac/act`, as the Mac the signature names: an act on one of this Mac's Jobs, or a Job started here. Fleet's status and body come back as they were said |
| `GET /api/mac/page?session_id=&port=` | Fleet's `GET /other_mac/page`: 204 where the Job is shared and shows that loopback port, else Fleet's 404 as it came |
| `GET`, `POST` and any method of `/api/mac/page/<job>/<port>/<path>` | The page itself: Fleet's `/other_mac/page` asked again, then the request sent on to `127.0.0.1:<port>/<path>` with its query, and the page's status, type and body handed back whole. Uncompressed. A Job not shared is Fleet's 404, worded as for a Job that does not exist |
| `GET /api/mac/live` | Fleet's `/other_mac/events` as NDJSON: each `StreamMessage` on a line as Fleet wrote it, and an empty line after 15 seconds of quiet. It ends when the Gateway stops |
| `POST /api/mac/update` | Fleet's `POST /macs/update`, as the Mac the signature names: this Mac restarts onto the build it sends. Taken while the link is frozen |
| `POST /api/mac/terminal` | Fleet's `POST /other_mac/terminal`, as the Mac the signature names: open, list, type into, resize or stop a terminal of a shared Job. Fleet's status and body come back as they were said |
| `GET /api/mac/terminal/observe?session_id=&run_id=` | Fleet's `/other_mac/terminal/observe` as NDJSON, one `SessionRunMessage` a line and an empty line on a quiet beat, as `live`. Fleet's refusal comes back before anything streams |
| `POST /api/mac/forget` | Forgets the caller on this side |

**The link.** For each linked Mac the Gateway holds one task.

| When | It does |
|---|---|
| The link opens | Reads `hello` first and keeps the name, colour, address, build, wire, build time and source. Where the wire is not this build's, it stops there: see *Builds that differ*. Otherwise it opens the stream, reads the sessions and the bug reports and hands them to Fleet with `PUT /other_macs/sessions` and `PUT /other_macs/bug_reports`. Sets `last_heard_at` and clears asleep |
| Each event | `POST /other_macs/event { mac, event }`. On `projects.changed` it reads the sessions again, which is how a project that stops being shared leaves this side |
| A resync or a missed count | Reads both lists again |
| A drop, or 45 seconds without a line | Keeps `asleep_since`, the moment it dropped, and `why`, the failure as a sentence. Opens it again after 1 second, doubling to a minute |

Pairing starts a task and forgetting stops it. Fleet keeps the rows while the Mac is asleep.
Fleet's `read_other_mac` reaches the other Mac only for a `/other_mac/thread?…` or `/other_mac/conversation?…` path, read as `/api/mac/thread?…` or `/api/mac/conversation?…`; any other is refused.
Its `send_other_mac` posts only `/other_mac/project_act`, `/other_mac/act`, `/other_mac/update` and `/other_mac/terminal`.
Its `open_other_mac_page` asks `/api/mac/page` and, where that answers 204, opens one loopback port on this Mac for that Job and page port, which signs every request through to `/api/mac/page/<job>/<port>/<path>`.
A Mac with `asleep_since` is answered asleep without a call, and a frozen link with its why; a page port already open answers 503 with that why while the link is frozen, as the other Mac's `/api/mac/page` routes do.
*Pages across Macs* in `docs/practices/protocol/his-other-macs-fleets-half.md` says the rest.
Its `follow_other_mac` follows only `/other_mac/terminal/observe?…`, as `/api/mac/terminal/observe?…`, and refuses it on a frozen link.

**A terminal of a Job on the other Mac** opens here with the calls Bridge makes at home, and its shell runs on the Mac that holds the Job, in that Job's slot.
Typing goes at once, one keystroke after another, and is never queued: a keystroke the link cannot carry is refused, and is not typed when the Mac wakes.
The stream ends when the link drops, the Mac sleeps or 45 seconds pass without a line, and the terminal here shows it ended.

**A Job moves to the other Mac** when he presses Move on it, from either Mac, where it is in a shared project.
It finishes its turn, its work is committed, its branch is pushed and its slot freed.
The other Mac reads its conversation across the link and resumes it under the same id in a fresh slot on that branch.
The row keeps its place in both lists and changes its Mac.
A push that fails, or a Mac that refuses or cannot be reached, leaves the Job where it was.
*Moving a Job between Macs* in `docs/practices/protocol/his-other-macs-fleets-half.md` says the rest.

**Builds that differ.** Two Macs whose builds speak another wire do not talk half matched: the link freezes until one updates.
The wire is the protocol ID, a hash of what the builds send each other, so two builds on different commits that speak the same wire stay linked.

| While the link is frozen | What happens |
|---|---|
| The other Mac's rows | Keep their last state on this Mac. Nothing is read from it |
| Acts, messages and project edits | Wait in the queue, with take-back, as for a Mac asleep. Nothing drains |
| The other Mac calls this one | Answered 503 with why on every `/api/mac/*` route but `hello` and `update`. A build from before the freeze keeps its own queue waiting on that 503 |
| The link | Reads `hello` again on the retry's pace, up to a minute, and opens once the wires match. What waited then goes, oldest first |

**Updating.** `POST /macs/update` moves this Mac, the other Mac, or both onto one build, always keeping running Jobs with `--adopt`.
The other Mac is asked over the link, the one call a frozen link carries.

| He presses | What moves |
|---|---|
| Update on an `older_build` row | The other Mac, onto this Mac's build: main, or this Mac's preview at its pushed commit |
| Update on a `newer_build` row | This Mac, onto the build the other Mac last said it runs |
| Refresh preview, this Mac and the other | Both. This Mac merges its preview, pushes the commit to `preview/<this Mac>` on origin, asks the other Mac to build exactly that commit, and restarts onto it |
| Restart onto main, this Mac and the other | Both, each fetching main for itself |

**A preview is pushed so both Macs build the same commit.** A preview refresh on a Mac with another linked pushes its merged commit to `preview/<this Mac>`, and the other Mac fetches that branch and builds the commit, merging nothing of its own.
The `ci` workflow runs on pull requests and `main` alone, so a push there starts nothing.
The body and its refusals are [update_mac](../../crates/ipc/operations/update_mac.toml).

**Acting across Macs.** The phone's routes cover what the phone does. A Mac can also message, answer, approve a plan, stop and merge a Job the other Mac holds, and start a Job there.

| He does, on this Mac | What happens |
|---|---|
| Presses any of those on a Job held from the other Mac | The route finds no Job of this Mac's by the id, and sends the act through the link as a `MacAct { id, act, at }` to the Mac that holds it. That Mac runs it on its own checkout and slots, as the door `mac`, and its answer comes back as it was said. A merge goes to the Mac whose Job owns the pull request, and nothing is merged from this one |
| Names the other Mac on a start or a dispatch (`mac` in the body, absent for this Mac) | The same, as a start there. The new Job's row is kept here at once with that Mac's id |
| Acts while that Mac is asleep | The act is answered 202 with a `Queued`, kept in the queue the project edits use, and sent when the Mac next speaks, oldest first, each with its `id` |
| Takes it back before then | `POST /queued/take_back`. An act the other Mac refused when its turn came stays there with its why, and the acts after it still go, until he takes it back |

**The Mac that holds the Job answers for it.** An act on a Job in a project that is not shared is refused 404, as one on a Job that does not exist, and nothing is written there.
An act that arrives again under an `id` it ran is answered with what it answered then, and is not run again.
The Fleet half is [His other Macs, Fleet's half](../practices/protocol/his-other-macs-fleets-half.md).

**Messages and alerts.** A Job on one Mac writes to a Job on the other by its address, and alerts cross the same way.

| What happens | Where it shows |
|---|---|
| A Job writes to a Job the other Mac holds | The receiver's thread draws it From the sender, naming its Mac, and the receiver wakes as for a Job on its own Mac. The sender's thread draws the call To the receiver, whose row names its Mac. A Mac asleep keeps it in the queue above, with take-back |
| Either Job is in a project that is not shared | Nothing crosses. The writer is refused as for an address no Job has, and learns nothing of the other Mac's projects |
| A shared Job on the other Mac needs him | It sounds, posts its notification, joins Needs you and counts on the Dock, with its Mac named, as a Job of this Mac does |
| The Macs first link, or a Mac wakes with nothing changed | Nothing rings. What already waited stands in Needs you |

## Macs nearby

**Two of his Macs on the same network find each other with no Cloudflare sign-in.** Only the Gateway must be on, the first step of Set up.
Each Gateway opens its Wi-Fi door and says so over Bonjour, and Settings, Macs lists the other within a few seconds with Pair.
Pair keeps Confirm and the four digits, because nothing has proved the two Macs are his.

| Piece | What it does |
|---|---|
| The Wi-Fi door | `pocket::Door`: a listener on every IPv4 network the Mac is on, at a port the system picks. TLS with the certificate in `pocket-door.key`, self-signed, since no CA signs for an address on a home network. It serves the Gateway's own router behind a filter that lets through `/pair/mac/*` and `/api/mac/*` and nothing else |
| Bonjour | `_armada._tcp`, named `armada-<id's first 12>`, at the door's port, with `id`, `name`, `colour` and `build` in its TXT record. The Gateway browses for the same type and keeps each Mac it resolves, with a routable IPv4 address where it has one. What Bonjour says only says where to knock |
| The found list | A Mac seen nearby is marked `nearby` on its row, which is the row of its tunnel name, so a Mac found on the account and nearby is one row. One found only nearby has no `address` |
| Pairing | As without a link, at the door. The asking Mac's client takes any certificate and notes the one it met, and the four digits cover both doors' certificates as the asking Mac met them. A Mac between them shows its own, so the two sides draw different digits. Each paired row keeps the other's certificate as `pin` |
| Every later call | `mac_client::signed`: at the door while Bonjour sees the Mac and its `pin` is kept, with a client that refuses any other certificate, else at its tunnel. A door that cannot be connected to falls back to the tunnel, since a request that never connected was never counted as a seen signature |
| Apart | A Mac paired nearby keeps the address its hello names, its tunnel, where it has one, and is reached there while Bonjour does not see it. A Mac with no tunnel is asleep until they share a network again |

**macOS asks once for Local Network access.** It asks for the process that runs Fleet, `armada`, when the Gateway first starts Bonjour: at Fleet's start on a Mac with the Gateway on, or at Set up's first press.
A Mac where it is denied finds no Mac nearby, and the Cloudflare list still works.
A network that drops multicast, as some work networks and VPNs do, hides a Mac the same way.

**A phone cannot use this.** A web app cannot browse the network.

## What pushes

Only Needs you: a Job whose `waiting_for` gains an item id it did not have, and that item is still open 60 seconds later.
Only a Job the phone sees pushes: the watcher reads `/events` as the phone, cut as *What the phone sees* says.

| Job | Pushes |
|---|---|
| A new item in `waiting_for`: a question, a permission ask, a plan to approve, a walk | yes, once per item, 60 seconds after it is raised if it is still open |
| Nothing pushed for the Job is open any more | a clear, which closes the Job's notification and shows nothing |
| `stuck` | no, Fleet repairs it without the person |

- **Settle first.** A new item is held for `SETTLE` (60 seconds, `crates/pocket/src/watch.rs`) and pushed only if it is still open then.
  A permission ask that the Mac or an agent's mode answers within seconds never reaches the phone.
  Items one Job raises inside the window go out as one push, saying the newest that is still open.
- **Once per item.** Pocket keeps the item ids it has pushed, per Job.
  An id leaves when its item does, so an item raised again pushes again.
  The first update Pocket sees for a Job only records what is open, so a restart pushes nothing for items already waiting.
- **Taken back.** When none of the items pushed for a Job is open, Pocket sends `{ session_id, clear: true }` once.
  The phone's service worker closes the notification tagged with that Job and shows nothing.
  An iPhone is sent no clear: iOS removes a push subscription after a few pushes that show nothing, so on an iPhone the notification stays until it is dismissed.
- **Payload.** `{ session_id, title, reason, origin }`: the title is the Job's title, the reason is the item's text, or `A walk is ready` for a walk item whose text is an address, and `origin` is this Mac's address.
  `origin` is absent while the Gateway cannot say its address.
- **Tapping it** opens `/jobs/<origin>/<session_id>`, the Job on that Mac, or `/jobs/_/<session_id>` without an origin. The Gateway serves the app for any `/jobs/` path.
- **Retries.** A failed send is tried again after 5 s, 30 s and 120 s, then dropped. Each phone sends on its own task, so a failing one holds up no other.
- **Dead subscriptions.** A 404 or 410 from the push service removes the subscription.
- **Expiry.** The push service drops an undelivered push after 24 hours.
- **iOS.** Push works only from the app opened from the Home Screen.

**Every Mac pushes to the one subscription the phone holds.** The phone subscribes once, with its first Mac's VAPID public key, and a push service refuses a push signed with any other key.

| Step | Route |
|---|---|
| The first Mac gives its own paired phone its VAPID private key | `GET /api/push/vapid_private`, answering `{ vapid_private }` |
| The phone passes it as it subscribes on another Mac | `POST /api/push/subscribe` with `vapid_private` beside `endpoint` and `keys` |
| That Mac signs every push to that subscription with it | `crates/pocket/src/watch.rs` |

The key is base64url of the raw 32-byte P-256 scalar, the form `pocket-vapid.key` holds.
A subscription without one is pushed to under the Mac's own key.

## Keys and where they live

| What | Where |
|---|---|
| VAPID key | `pocket-vapid.key` in Armada's machine directory, made on first start, mode 600 |
| Devices, push subscriptions | The store, tables `pocket_devices` and `pocket_push_subscriptions`. A subscription made with another Mac's VAPID key keeps it in `vapid_private`, read into a `Secret` so it cannot be printed |
| Pairing codes, claims, seen signatures | Memory. A Gateway that stops drops them, and a half-finished pairing starts again |
| This Mac's id, name and colour | The store, table `pocket_this_mac`, one row |
| Other Macs | The store, table `pocket_macs`: id, name, colour, address, the other's public key, build, when paired, state, the back code while waiting, `last_heard_at`, `asleep_since`, `why` and `pin`, its Wi-Fi door's certificate |
| The Wi-Fi door's certificate | `pocket-door.key` in Armada's machine directory: the certificate and its private key, made on first start, mode 600 |
| This Mac's key toward each other Mac | The Keychain, a generic password under `armada-mac-key` with the other Mac's id as its account, written through `security -i` so it is never in a process's arguments. A test keeps them in memory |
| A Mac's pairing codes and claims | Memory, as a phone's are |
| `cloudflared`, its sign-in and the tunnel's credentials | `cloudflared/` in the machine directory: `bin/`, `home/.cloudflared/cert.pem`, `tunnel.json`, and `tunnel`, the tunnel's name and host |

The VAPID `sub` is the Gateway's own address, so no personal address leaves the Mac. Where it cannot say one it falls back to a placeholder.

## Unpairing

Bridge, Settings, Phone, then the phone's row.
That calls `DELETE /admin/devices/<id>`, which removes the device and its push subscriptions in one step.
The phone's next call to that Mac answers 401, and the app shows that Mac as not paired with a way to pair it again.
