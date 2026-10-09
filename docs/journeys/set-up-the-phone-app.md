# Set up the phone app

**Trigger:** You want to see and answer your Jobs from the phone. Read [Pocket](../concepts/pocket.md) first for what the parts are.

## You need

- Tailscale on the Mac and on the iPhone, both signed in to one tailnet.
- Armada built, with a Fleet running.
- Node and `pnpm`, once, to build the app.

## Steps

| | On | Do |
|---|---|---|
| 1 | Mac | `armada pocket`, once, then stop it. That opts this Mac in. Then restart Armada (`scripts/restart`, or the Fleet panel in Bridge): from then on every restart builds the app and keeps the Gateway running under launchd as `com.armada.pocket` |
| 2 | Mac | `tailscale serve --bg --https=443 http://127.0.0.1:8443`. Puts the Gateway on your tailnet over https, and survives reboots |
| 3 | iPhone | Open the Gateway's address in Safari, then Share, Add to Home Screen. Open Armada **from the Home Screen** |
| 4 | Bridge | Settings, Phone, Pair a phone, then **Copy link** |
| 5 | iPhone | Paste the link into Pairing link and press Pair. The phone makes its key and names itself |
| 6 | Bridge | Confirm the phone. Until then it signs nothing. Then allow notifications on the phone |

Pair from the Home Screen app, not from Safari: on iOS the two keep separate storage, so a phone paired in Safari opens the Home Screen app unpaired. Scanning the QR opens Safari, which pairs Safari alone.

Push works only in the Home Screen app: iOS grants it to nothing else.

## `armada pocket`

Runs the Phone Gateway. It does not start Fleet; it finds the running one through the runtime file on every call, so a Fleet that restarted is found at its new port.

```
armada pocket [--port <n>] [--assets <dir>]
```

| Flag | Default | Means |
|---|---|---|
| `--port` | `8443` | The loopback port. If you change it, give `tailscale serve` the same one |
| `--assets` | `apps/pocket/dist`, from the directory you run it in | The built app. A missing directory serves nothing, and the phone gets 404 |

**Needs:** the Armada store in the machine directory, and Tailscale (on the path, or inside the Mac app) for pairing and for the push sender address. Fleet may be down: `GET /admin/status` on the Gateway answers 503, or 502 when Fleet is found and does not answer.

**It fails when** the port is taken, the store cannot be opened, or the VAPID key file is unreadable. Each says which.

**Starting it again** keeps paired phones and the VAPID key. It drops unfinished pairings.

## When something is off

| You see | It means | Do |
|---|---|---|
| `Tailscale is not signed in on this Mac` or `not installed` when pairing | The Gateway could not read its tailnet name | Sign in, then start pairing again |
| The app sends you to `/pair` | This phone is not paired, or was unpaired | Pair it again |
| `The phone's clock is more than a minute off` | Skew over 60 s | Set the phone's clock to automatic |
| `This code is not good any more` | Five minutes passed, or it was used | Start pairing again in Bridge |
| 404 on the app's address | `apps/pocket/dist` is missing | Step 1 |
| No push on a Job that stopped | App not opened from the Home Screen, notifications not allowed, or the Job's stop is not Blocked | See [Pocket](../concepts/pocket.md), *What pushes* |

To remove a phone, see *Unpairing* in [Pocket](../concepts/pocket.md).
