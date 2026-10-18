# Sound

**Part of:** [Design System — UI & Voice](../design-system.md).

**A sound says which kind of wait, to a person who is not looking.** macOS
gives every notification the same chime, so work that has stopped and a
review that will keep sound identical. Armada gives the two their own tone.

| Tone | Plays when | Notes |
|---|---|---|
| **Blocked** | A notification tells of a Job that has stopped: it entered Escalated, or ran out of retries and awaits repair | G4 → D4, falling, 170ms apart |
| **Waiting** | A notification tells of any other Job that started waiting on a person | One A4 |

Each note is a sine of 200ms, rising to full in 12ms and decaying
exponentially. Both tones sit at one level, because loudness is macOS's
setting and not Armada's.

**A tone is the notification's own sound, never a second channel.** It rides
on the banner, so macOS's permission, Focus and per-app sound settings govern
it exactly as they govern the banner. Bridge never plays audio itself, and a
refused notification is a silent one. A packaged Bridge carries both tones in
its bundle; a development Bridge runs Electron's own, so it installs them into
`~/Library/Sounds`, which macOS searches for the same names.

**The default model's step down takes the Waiting tone.** The weekly limit's offer, and its restore after the window resets, are decisions that will keep, so they sound as a Job waiting for review does and never as work that stopped.
They go no further than an OS notification.

**One notification, one tone.** A batch that holds a stopped Job plays
Blocked, because the most urgent Job in it decides. Nothing else makes a
sound: a Job that lands, a Check that passes and a press are silent, which
keeps the loudness order in Behaviour rules intact.
