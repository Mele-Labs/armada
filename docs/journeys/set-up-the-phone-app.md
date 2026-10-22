# Set up the phone app

**Trigger:** You want to see and answer your Jobs from the phone, on one Mac or several. Read [Pocket](../concepts/pocket.md) first for what the parts are.

## You need

- Tailscale on each Mac and on the iPhone, all signed in to one tailnet.
- Armada built, with a Fleet running, on each Mac.
- Node and `pnpm`, once, to build the app.

## Steps, on the first Mac

| | On | Do |
|---|---|---|
| 1 | Bridge | Settings, Phone, then switch the phone on. Fleet starts the Phone Gateway on `127.0.0.1:8443` at once, and at every start after. Every `scripts/restart` builds the app and copies it where the Gateway serves it from, so a Mac restarted onto this change already has it |
| 2 | Mac | `tailscale serve --bg --https=443 http://127.0.0.1:8443`. Puts the Gateway on your tailnet over https, and survives reboots |
| 3 | iPhone | Open the Mac's tailnet address in Safari, then Share, Add to Home Screen. Open Armada **from the Home Screen** |
| 4 | Bridge | Settings, Phone, Pair a phone, then **Copy link** |
| 5 | iPhone | Paste the link into Pairing link and press Pair. The phone makes its key and names itself |
| 6 | Bridge | Confirm the phone. Until then it signs nothing. Then allow notifications on the phone |

Pair from the Home Screen app, not from Safari: on iOS the two keep separate storage, so a phone paired in Safari opens the Home Screen app unpaired.
Scanning the QR opens Safari, which pairs Safari alone.

Push works only in the Home Screen app: iOS grants it to nothing else.

## Adding another Mac

| | On | Do |
|---|---|---|
| 1 | That Mac | Steps 1 and 2 above |
| 2 | That Mac's Bridge | Settings, Phone, Pair a phone, then **Copy link** |
| 3 | iPhone | In the app you already have, add a Mac and paste the link. The app claims it on that Mac directly; you do not install a second app |
| 4 | That Mac's Bridge | Confirm the phone |

The app then shows every Mac's Jobs together, each row labelled with its Mac.
Pushes from every Mac arrive in the one app: the phone hands each Mac the push key its first Mac made.

## What you can do from the phone

| Where | You can |
|---|---|
| Needs you | See every Job waiting on you, across your Macs, oldest first |
| A Job | Read its whole thread, answer its question or ask, approve its plan or send changes, send it a message |
| A file or sketch the Job showed | Open it: text or a picture up to 1 MiB, or the sketch as drawn |
| A walk | Nothing: its row says to open it on the Mac |

## Switching it off

Bridge, Settings, Phone, then switch the phone off.
Fleet stops the Gateway at once and does not start it again; paired phones and the push key are kept for when you switch it back on.

**It fails to start when** port 8443 is taken, the store cannot be opened, or the push key file is unreadable.
Settings, Phone then shows the phone as on and not listening, with the reason, and Fleet tries again at its next start.

## When something is off

| You see | It means | Do |
|---|---|---|
| `Tailscale is not signed in on this Mac` or `not installed` when pairing | The Gateway could not read its tailnet name | Sign in, then start pairing again |
| One Mac marked not paired in the app | That Mac does not know this phone, or it was unpaired there | Pair it again from that Mac's Bridge |
| `The phone's clock is more than a minute off` | Skew over 60 s | Set the phone's clock to automatic |
| `This code is not good any more` | Five minutes passed, or it was used | Start pairing again in Bridge |
| 404 on the app's address | The built app is missing: `pocket-app` in `~/Library/Application Support/Armada/` | `scripts/restart` on that Mac |
| `Armada could not listen on 127.0.0.1:8443` in Settings, Phone | Something else holds the port, such as the old `armada pocket` job | Quit it, then switch the phone off and on |
| No push on a Job that went quiet | App not opened from the Home Screen, notifications not allowed, or the Job is not stuck | See [Pocket](../concepts/pocket.md), *What pushes* |

To remove a phone, see *Unpairing* in [Pocket](../concepts/pocket.md).
