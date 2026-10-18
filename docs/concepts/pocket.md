# Pocket

**What it is:** The owner's phone app, a Home Screen web app named Armada, and the Phone Gateway it talks to, which stands in front of Fleet so Fleet itself stays on loopback with no auth.

---

**Kind:** Surface.

Work stops while you are away from the Mac. The phone buzzes once with the Job's title when it goes quiet, you tap it, and you see which Jobs need you. A question a Job is holding you answer in a tap or two.

## What it is

```
Phone (PWA) ──https──▶ tailscale serve ──▶ Phone Gateway 127.0.0.1:8443 ──▶ Fleet 127.0.0.1
                                                │
                                                └──▶ Web Push (Apple / Google) ──▶ Phone
Bridge (Settings → Phone) ──loopback /admin/*──▶ Phone Gateway
```

| Part | Where | Does |
|---|---|---|
| Phone app | `apps/pocket`, built to its `dist` folder | Shows what needs the owner, signs each call, receives pushes |
| Phone Gateway | `crates/pocket`, run by `armada pocket` | Serves the app, checks signatures, trims what it reads from Fleet, sends pushes |
| Fleet | `crates/api`, `crates/fleet` | Listens on loopback, takes no auth, refuses any request carrying `Origin`. The Gateway calls it with none, and with `x-armada-caller: phone`, so a phone act is recorded with the `phone` door ([Retro](retro.md)) |

Setup is in [Set up the phone app](../journeys/set-up-the-phone-app.md).

## What keeps it closed

| Control | How |
|---|---|
| Loopback bind | `pocket::bind` takes a port and no host, so no setting opens the Gateway to a network. `tailscale serve` is the only way in |
| Allowlist | `router` in `crates/pocket/src/routes.rs` names every route. Anything else is 404, and so is a listed path asked with an unlisted method |
| `/admin/*` | For Bridge's main process. Refused (403) when the request carries `Origin`, which means a web page sent it, or any header `tailscale serve` adds, which means it came over the tailnet |
| Signed `/api/*` | The phone sends `X-Pocket-Device`, `X-Pocket-Time` and `X-Pocket-Signature`: ECDSA P-256 over the method, path and query, time, and the body's SHA-256. The private key is made on the phone and never leaves it |
| Clock | A request more than 60 seconds from the Mac's clock is refused |
| Replay | A signature seen inside that window is refused the second time |
| Pairing | A code lives five minutes and claims once. `/pair` answers ten requests a minute, wrong codes included. A claim is not a device until the owner confirms it in Bridge |
| Subscriptions | A push endpoint must be `https`, so a phone cannot make the Mac post to loopback |

## What the phone sees

`crates/pocket/src/phone_sessions.rs` builds `PhoneSession` field by field. Fleet's Job record is never serialised and trimmed, so a field Fleet adds does not reach the phone until someone writes it there. The app has three lists, each read from the routes Bridge's Cockpit reads.

| List | Holds | Route |
|---|---|---|
| Needs you | The live Jobs waiting on the owner, oldest wait first | `GET /api/needs` |
| Running | The live Jobs with a turn running and no question held | `GET /api/sessions` |
| Done | The 50 Jobs that ended last, newest first | `GET /api/sessions?state=ended` |

| The phone gets | The phone never gets |
|---|---|
| Id, title, repository directory name, whether it is hosted or a Terminal one | A transcript, a file, a message |
| Whether it is waiting and since when, and whether a turn is running | A working directory or an environment value |
| When it started and ended | Any path on the Mac |
| The held ask: its question text and options, or the tool and its one-line detail | Any ask but the one it is held on |

| Job | The phone shows | The phone can do |
|---|---|---|
| Hosted | The held ask | Answer a question with the options chosen, or a permission ask with `allow_once` or `refuse` |
| Terminal | That it is waiting. While Fleet holds its question, the ask as a hosted one shows it | While Fleet holds the question, the same answers as a hosted Job. After the hold, nothing: the answer is refused with a 409 and the question is answered in the terminal |

- **A Terminal Job carries an ask only while Fleet holds its question**: from `asked_at` for `holding_for_seconds`, by the Gateway's clock. It falls back to the terminal after that. The answer goes to Fleet's `answer_session_ask`, as a hosted one does. Whether a Job is hosted is read from Fleet's own row when the answer arrives, never from the phone.
- **`allow_and_remember` is never offered.** It writes the owner's personal settings, so the answer type has no such value.
- **Job asks do not push.** They queue in Needs you.
- **`/api/live`** is Fleet's `/events` narrowed to `session.changed`, written as `{change: "session", session_id}`, and `{change: "resync"}` where the phone's lists may be behind. The app reads its three lists again on either.

**One-line dispatch.** The phone sends a text and a repository label. The Gateway offers the labels it derives from Fleet's manifests, maps the one chosen to Fleet's manifest id and refuses a label it does not know or two repositories share. It starts a Job with `POST /sessions/start` carrying the text as `request`, so Fleet's proposer names it as it does for Bridge's Dispatch. The phone answers 201 with the Job.

## What pushes

Only a Job that has gone quiet: a live Job whose `standing` is `stuck`, a turn running with nothing happening for the quiet threshold.

| Job | Pushes |
|---|---|
| Live and `stuck` | yes, once |
| A question, a permission ask, a plan to approve, a finished or replied Job | no, it queues in the app |

- **Once per condition.** A Job pushes once when it goes quiet. It pushes again only after it has moved on and gone quiet again.
- **Payload.** Job id, title and the word `stalled`. Tapping it opens the app's home.
- **Retries.** A failed send is tried again after 5 s, 30 s and 120 s, then dropped. Each phone sends on its own task, so a failing one holds up no other.
- **Dead subscriptions.** A 404 or 410 from the push service removes the subscription.
- **Expiry.** The push service drops an undelivered push after 24 hours.
- **iOS.** Push works only from the app opened from the Home Screen.

The Gateway follows Fleet's `/events` and finds Fleet again after a restart. It pushes only while it is running.

## Keys and where they live

| What | Where |
|---|---|
| VAPID key | `pocket-vapid.key` in Armada's machine directory, made on first run, mode 600 |
| Devices, push subscriptions | The store, tables `pocket_devices` and `pocket_push_subscriptions` |
| Pairing codes, claims, seen signatures | Memory. A Gateway restart drops them, and a half-finished pairing starts again |

The VAPID `sub` is the Gateway's own tailnet address, so no personal address leaves the Mac. With Tailscale down it falls back to a placeholder.

## Unpairing

Bridge, Settings, Phone, then the phone's row. That calls `DELETE /admin/devices/<id>`, which removes the device and its push subscriptions in one step. The phone's next call fails the signature check and the app returns to `/pair`.
