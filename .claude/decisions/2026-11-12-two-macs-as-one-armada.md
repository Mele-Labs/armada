# How should his two Macs work together?

**Said 2026-11-12.** He works on a home Mac and a work Mac, and until now each was a separate Armada: to watch, answer or merge a Job he had to be at the Mac running it.
He approved the page "Two Macs as one Armada", revision 55, on 12 Nov.
The work Mac holds private projects, and on whether those could ever be seen from home he said it was "absolutely critical" that they could not.

## What he chose

| Asked | He chose | The cost he took |
|---|---|---|
| Which work crosses between the Macs | Only projects he marks shared. Off by default. The Mac holding a Job enforces it before anything leaves. Private Jobs, their acts, messages and bug reports never cross, and an act on an unshared Job is answered "not found" | Every project he wants on both Macs has to be marked, one by one |
| How the Macs reach each other | A Cloudflare named tunnel per Mac, on his own Cloudflare account and his domain armada.codes, bought at Cloudflare. Armada hosts nothing for anyone. Measured 12 Nov: a named tunnel on armada.codes on each Mac was opened from the other Mac, both ways, through the work VPN | The domain, about $10 a year. A tunnel needs outbound port 7844, which his IT could close later, and a tunnel held open for hours is not measured |
| How the Macs pair | A one-time link from one Mac, pasted on the other, then Confirm. One pairing gives each Mac a key for the other | None named |
| Whether the phone is a separate system | One system: phones and Macs are paired devices on one Gateway, one address per Mac, one Devices list. The share switch limits the phone too | A phone sees no Job in a project he has not marked shared |
| What he sees | A Mac chip on every Jobs row, each Mac one fixed colour. A Mac chip in the dispatch sheet. Bug reports in one list with Mac chips. A message between Jobs shows in both threads | The other Mac's Jobs join the core Jobs list, not a mod view |
| A Mac that is asleep | A send waits on the sending Mac and can be taken back | A new queue table in the store |
| Updates | Updating one Mac offers to update the other | A route that runs an update on its own Mac, which does not exist today |
| What the other Mac's Jobs do here | They alert like local ones: sound, toast, Needs you and the Dock count. Their walks, terminals and dev servers open here through the signed Gateway, never open to the internet | A page and a terminal stream through the link |
| A running Job | It can move to the other Mac: it finishes its turn, its branch is pushed, and the other Mac resumes the same conversation in a fresh slot | How the conversation travels is not decided |
| Projects and the Operator | Shared projects and their tasks sync both ways. The Operator warns a new Job about overlapping work on the other Mac | Two edits made while a Mac sleeps have no merge rule yet |
| How it is set up and tested | From native Bridge only. No step asks him to run a terminal command. Armada downloads and updates cloudflared itself, and Cloudflare sign-in opens from a Bridge button | Setup is built as Bridge screens, not a script |

Left out: the per-Mac pages stay on their Mac. These are slots, cleanup, Checks logs, time analytics, settings and mods.

## What it costs

Five stages of build, and the domain at about $10 a year.

| Stage | Build |
|---|---|
| 1 | Share switch, tunnel, Devices list, pairing, both Jobs lists, shared projects synced, Operator overlap. Five Fleet Jobs and one native |
| 2 | Message, approve, stop, merge and dispatch across, the queue, alerts. Two Fleet Jobs and one native |
| 3 | Update both Macs. One Fleet Job and one small native |
| 4 | Walks, terminals and dev servers across. Two Fleet Jobs and one native |
| 5 | Move a running Job to the other Mac. Two or three Fleet Jobs and one small native |

The estimates are low confidence, sized from the phone's client and `crates/pocket/src/routes.rs`, not from a plan.
Native Bridge is built by a separate owner Job, so each issue lists the screens it needs and builds none.

## Still open

When the Macs' builds differ and he declines the update, does the link freeze or keep working?
Freezing is the reference design's rule and stops anything running half matched.
Keeping it working costs more: the Macs speak a narrower language of their own that only ever grows, and a Mac he forgot stays useful.
It is asked in `armada-hu76.11`.

## Where it landed

The epic `armada-hu76` now reads as this decision. Its children:

| Issue | What |
|---|---|
| `armada-hu76.7` | Stage 1: share switch and filter |
| `armada-hu76.8` | Stage 1: tunnel and Devices |
| `armada-hu76.5` | Stage 1: pairing and both Jobs lists |
| `armada-hu76.3` | Stage 1: shared projects sync |
| `armada-hu76.2` | Stage 1: Operator overlap |
| `armada-hu76.9` | Stage 2: acting across Macs |
| `armada-hu76.10` | Stage 2: messages and alerts |
| `armada-hu76.11` | Stage 3: update both Macs |
| `armada-hu76.12` | Stage 4: walks and dev servers across |
| `armada-hu76.13` | Stage 4: terminals across |
| `armada-hu76.14` | Stage 5: move a Job between Macs |
| `armada-hu76.15` | P3: if port 7844 closes, the work Mac falls back to home's Funnel address |

The share switch blocks every stage issue except the tunnel, the tunnel blocks pairing, pairing blocks everything that reads or acts across a link, and acting across Macs blocks the move.

## What this supersedes

- **The 4 Nov carrier.** Reports and projects no longer travel through a private armada-home, and "Operators cannot send words to a Job on another machine in this version" is reversed: messages across Macs are the point. The GitHub sign-in is no longer a wait, since pairing is by link.
- **The 6 Nov mod view.** Another Mac's Jobs sit in the core Jobs list. `armada-hu76.4`, the browser try, and `armada-hu76.6`, the work Mac dialling home, are closed.
- **The 30 Oct drop of cross-machine Jobs.** Seeing and acting on the other Mac's Jobs is back, narrowed to shared projects.

`armada-hu76.1`, the per-repository setting of where a repository's data sits, stands. The per-project share switch governs what crosses between Macs.
