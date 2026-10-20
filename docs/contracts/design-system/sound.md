# Sound

**Part of:** [Design System — UI & Voice](../design-system.md).

**Armada makes two kinds of sound: a tone on a notification, and a quiet tick under the hand.**
The tone says which kind of wait, to a person who is not looking.
The tick confirms a move or a press, to a person who is.

## Notification tones

macOS gives every notification the same chime, so work that has stopped and a
review that will keep sound identical. Armada gives the two their own tone.

| Tone | Plays when | Notes |
|---|---|---|
| **Blocked** | A notification tells of a job held on the owner: a question, a permission, a question on a walk, a plan, a full slot pool or the delivery choice | G4 → D4, falling, 170ms apart |
| **Waiting** | A notification tells of anything else a job waits on him for | One A4 |

Each note is a sine of 200ms, rising to full in 12ms and decaying
exponentially. Both tones sit at one level, because loudness is macOS's
setting and not Armada's.

**A tone is the notification's own sound, never a second channel.** It rides
on the banner, so macOS's permission, Focus and per-app sound settings govern
it exactly as they govern the banner, and a refused notification is a silent
one. A packaged Bridge carries both tones in its bundle; a development Bridge
runs Electron's own, so it installs them into `~/Library/Sounds`, which macOS
searches for the same names.

**A banner goes out only while Bridge is not in front**, and only while
`features.notifications` is on. In front, the job's row already pulses. The
dock count shows how many jobs wait on him either way.

**One notification, one tone.** Items that arrive within two seconds of each
other go out as one banner, and a batch that holds a blocked item plays
Blocked, because the most urgent item decides.

**The default model's step down takes the Waiting tone.** The weekly limit's offer, and its restore after the window resets, are decisions that will keep, so they sound as a job waiting for review does and never as work that stopped.
They go no further than an OS notification.

## Interface sounds

**Bridge plays its own interface sounds, behind `features.uiSounds`.** They are
synthesised in the renderer, carry no file, and sit at one low level well
under a notification tone.

| Sound | Plays when |
|---|---|
| **Move** | The keyboard moves the selection in a list, a rail or a grid |
| **Press** | A button, a switch or a tab is pressed |

Typing never plays, and nothing else makes a sound: a job that lands, a Check
that passes and a panel that opens are silent. A run of moves faster than the
ear can part plays once.
