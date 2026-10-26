# Pocket

**What it is:** The owner's phone app, a Home Screen web app named Armada, and the Phone Gateway each Mac runs inside Fleet for it, which stands in front of Fleet's own routes so they stay on loopback with no auth.

---

**Kind:** Surface.

Work stops while you are away from the Mac.
The phone buzzes once with the Job's title when it goes quiet, you tap it, and you see the Job's thread.
You answer its question, approve its plan or send it a message from there, on any of your Macs.

## What it is

```
Phone (PWA) ──https──▶ the owner's tunnel ──▶ Fleet's process
                                           ├─ Phone Gateway 127.0.0.1:8443 ──http──▶ Fleet 127.0.0.1:<port>
                                           └─ Web Push (Apple / Google) ──▶ Phone
Bridge (Settings → Phone) ──loopback /admin/*──▶ Phone Gateway
Bridge (Settings → Phone) ──GET / POST /phone──▶ Fleet
```

| Part | Where | Does |
|---|---|---|
| Phone app | `apps/pocket`, built to its `dist` folder | Shows what needs the owner on every paired Mac, signs each call with that Mac's key, receives pushes |
| Phone Gateway | `crates/pocket`, started by Fleet in `crates/armada/src/pocketing.rs` | Serves the app, checks signatures, passes the phone's calls to Fleet, sends pushes |
| Fleet | `crates/api`, `crates/fleet` | Listens on loopback, takes no auth, refuses any request carrying `Origin`. The Gateway calls it over HTTP with none, and with `x-armada-caller: phone`, so a phone act is recorded with the `phone` door ([Retro](retro.md)) |

**The Gateway runs in Fleet's process, on a listener of its own.** It works while Bridge is closed, and stops when Fleet stops.
It still calls Fleet over HTTP rather than in process, so every phone call meets Fleet's router and its `phone` door.

Setup is in [Set up the phone app](../journeys/set-up-the-phone-app.md).

## The way in

**The owner brings the tunnel.** Armada runs no relay and no service of its own; whatever reaches `127.0.0.1:8443` over https is the way in, and the choices are in [Set up the phone app](../journeys/set-up-the-phone-app.md).

| The address is | When |
|---|---|
| The line in `pocket-address` in the machine directory | The file is there. One `https://` origin, no path; anything else refuses pairing with a sentence naming the file |
| `https://<tailnet name>` from `tailscale status --json` | The file is not there |

It is read at each ask, in `pocket::machine_address`, so an edit needs no restart.

**Bridge walks the owner to Funnel.** Settings, Phone draws a row for each step, from these two routes in `crates/pocket/src/setup.rs`, and reads them again every 5 s until Funnel is on.

| Route | Does |
|---|---|
| `GET /admin/setup` | `{ tailscale, served, named, address? }`: Tailscale `missing`, `signed_out` or `running`; in front of the Gateway `none`, `tailnet` (`tailscale serve`) or `funnel`; and whether `pocket-address` names the address |
| `POST /admin/setup/funnel` | Runs `tailscale funnel --bg` in front of the Gateway. 204 once on; `{ approve }`, the tailnet's page to allow Funnel on, while the command waits for it; 502 with Tailscale's words otherwise |

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
| Loopback bind | `pocket::bind` and `pocket::bind_now` take a port and no host, so no setting opens the Gateway to a network. The tunnel the owner points at it is the only way in |
| Allowlist | `router` in `crates/pocket/src/routes.rs` names every route. Anything else is 404, and so is a listed path asked with an unlisted method |
| `/admin/*` | For Bridge's main process. Refused (403) when the request carries `Origin`, which means a web page sent it, or a header `tailscale serve`, Funnel, ngrok or `cloudflared` adds, or a `Host` other than loopback, which means it came through a tunnel |
| Signed `/api/*` | The phone sends `X-Pocket-Device`, `X-Pocket-Time` and `X-Pocket-Signature`: ECDSA P-256 over the method, path and query, time, and the body's SHA-256. The private key is made on the phone, one per Mac, and never leaves it |
| Cross-origin | `/api/*` and `POST /pair` answer a preflight and name the caller's origin when it is `https`, so the app one Mac served may call another. The signature is still the only auth. `/admin/*` never answers one |
| Clock | A request more than 60 seconds from the Mac's clock is refused |
| Replay | A signature seen inside that window is refused the second time |
| Pairing | A code lives five minutes and claims once. `/pair` answers ten requests a minute, wrong codes included. A claim is not a device until the owner confirms it in Bridge |
| Subscriptions | A push endpoint must be `https`, so a phone cannot make the Mac post to loopback |
| Shown files | A file is read only where the Job showed it with `show_file`. Fleet refuses any other path with 404 |

**A refused signature says which failure it is**, so the app can tell this Mac does not know the phone from a request that went wrong.

| Status | Means |
|---|---|
| 401 | This phone is not paired with this Mac. The app marks this Mac alone as not paired |
| 400 | No signature, a time that is not a number, a clock off by more than a minute, or a signature that does not match |
| 409 | The same signature was sent before |

## What the phone sees

| Screen | Holds | Route |
|---|---|---|
| Needs you | The live Jobs waiting on the owner, oldest wait first | `GET /api/sessions`, the ones waiting |
| Running | The live Jobs not waiting on the owner, one with a turn running marked working and the rest idle | `GET /api/sessions` |
| A Job | Its whole thread, as Bridge reads it | `GET /api/sessions/one?session_id=` |
| A file | A file the Job showed: its text, or a picture | `GET /api/sessions/shown?session_id=&path=` |
| This Mac | The name its rows are labelled with: the first label of its address, or its host name | `GET /api/machine` |

**The lists are narrowed.** `crates/pocket/src/phone_sessions.rs` builds `PhoneSession` field by field, so a field Fleet adds to the Job record does not reach a list until someone writes it there.

**A Job's thread is not narrowed.** `/api/sessions/one` is Fleet's `SessionThread` as Fleet sends it, the Job record included.

| The phone gets | The phone never gets |
|---|---|
| A Job's whole thread and its record, its folder on the Mac among it | A walk's window: its row says to open it on the Mac |
| A file the Job showed, up to 1 MiB, as text or a picture's bytes | A file the Job did not show, or one over 1 MiB (413) |
| A sketch the Job drew, from the scene its record carries | A terminal |

**`/api/sessions/shown`** is Fleet's `get_shown_file`, which reads only a path the Job's record holds as an artifact of form `file` it showed.
A picture (`png`, `jpg`, `jpeg`, `gif`, `webp`, `heic`) comes as its bytes under its type; anything else must be UTF-8 and comes as `{ path, text }`.

## What the phone can do

Each reply is passed to Fleet's route of the same name without `/api`, its body as the phone sent it, and Fleet's status and body come back as they are.
The phone sends Fleet's own body shapes.

| Phone route | Fleet route |
|---|---|
| `POST /api/sessions/ask/answer` | `answer_session_ask` |
| `POST /api/sessions/waiting/answer` | the answer to an item the Job waits on |
| `POST /api/sessions/plan/approve` | `approve_plan` |
| `POST /api/sessions/plan/changes` | `send_plan_changes` |
| `POST /api/sessions/message` | a message to the Job |

**`POST /api/sessions/answer`** is the older, checked answer, kept for the app's Needs you rows:

- **A Terminal Job carries an ask only while Fleet holds its question**: from `asked_at` for `holding_for_seconds`, by the Gateway's clock.
  After that the answer is refused with 409 and the question is answered in the terminal.
- **Whether a Job is hosted is read from Fleet's own row** when the answer arrives, never from the phone.
- **`allow_and_remember` is never offered.** It writes the owner's personal settings, so the answer type has no such value.

**One-line dispatch.** The phone sends a text and a repository label to `POST /api/sessions`.
The Gateway maps the label to Fleet's manifest id, refuses one it does not know or two repositories share, and starts a Job with `POST /sessions/start` carrying the text as `request`.
It answers 201 with the Job.

**`/api/live`** is Fleet's `/events` narrowed to `session.changed`, written as `{change: "session", session_id}`, and `{change: "resync"}` where the phone's lists may be behind.
The app keeps one per Mac and reads again on either.
While Fleet is quiet it writes a `: beat` comment line every 15 seconds, so the phone can tell a waiting stream from a dead one.

**`/api/sessions` answers the live Jobs and takes no `state`.** The phone has no list of ended ones.

**The app's files** are sent with `Content-Encoding: br` or `gzip` when the request accepts it and a `.br` or `.gz` copy sits beside the file.
Anything under `/assets/` is hashed by the build and cached for a year, `immutable`.
Every other file, `index.html` and `sw.js` included, is `no-cache`.

## Several Macs

**The phone talks to each Mac directly.** There is no relay.
It is paired once per Mac, with a key of its own for each, and shows every Mac's Jobs together, each row labelled with its Mac where there is more than one.
The app is served by whichever Mac the owner opened it from, and calls the others across origins.

A 401 from one Mac marks that Mac as not paired, and the others carry on.

## What pushes

Only a Job that has gone quiet: a live Job whose `standing` is `stuck`, a turn running with nothing happening for the quiet threshold.

| Job | Pushes |
|---|---|
| Live and `stuck` | yes, once |
| A question, a permission ask, a plan to approve, a finished or replied Job | no, it queues in the app |

- **Once per condition.** A Job pushes once when it goes quiet. It pushes again only after it has moved on and gone quiet again.
- **Payload.** `{ session_id, title, reason, origin }`: the reason is `stalled`, and `origin` is this Mac's address. `origin` is absent while the Gateway cannot say its address.
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

The VAPID `sub` is the Gateway's own address, so no personal address leaves the Mac. Where it cannot say one it falls back to a placeholder.

## Unpairing

Bridge, Settings, Phone, then the phone's row.
That calls `DELETE /admin/devices/<id>`, which removes the device and its push subscriptions in one step.
The phone's next call to that Mac answers 401, and the app shows that Mac as not paired with a way to pair it again.
