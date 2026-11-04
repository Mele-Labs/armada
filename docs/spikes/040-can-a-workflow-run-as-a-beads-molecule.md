# Spike 40 — Can a workflow run as a beads molecule?

**Answer: yes, if Fleet keeps the gates.**
`feature` poured as a molecule of beads, and every step and gate behaved as written: a step blocked until its gate closed, `bd ready --gated` named the step to resume, and the molecule closed itself after its last step.
**Beads does not enforce the gates.**
An agent can close any of them, and `bd mol current` steers an agent towards one, so only Fleet may resolve a gate, and Fleet must check that nobody else has.

Measured 4 Nov 2026 with bd 1.3.2, on this Mac, against a throwaway beads database under `/tmp`, never the owner's.
The question is the owner's, 4 Nov 2026: rebuild the workflow rail on beads molecules without losing his view of what every agent is doing.
No dev Fleet was started, because Fleet has no code that pours or reads a molecule yet, so it would have run the same `bd` commands.

## How it was run

`040-feature.formula.json` is the `feature` workflow as a beads formula: Propose, Walk, Refine and Deliver as steps, then a Landed step.
Each gate sits on the step it holds back: a human gate on Walk (plan approved), on Refine (walk approved) and on Deliver (judge passed), and a `gh:pr` gate on Landed.
Walk carries `condition: "{{seen}} == true"`.
It was poured for two test Jobs, one seen and one not, and driven with `BEADS_ACTOR` set to `agent-j1` for the agent's acts and `fleet` for Fleet's.
`040-transcript.txt` is every command and its output, with the owner's address replaced.

## What it measured

| Question | What bd 1.3.2 did |
|---|---|
| Can the formula stay at `.armada/workflows/feature.json` | No. `bd mol pour <path>` answers `not found as formula or proto ID`. It reads only `<name>.formula.json` under `.beads/formulas/` or `~/.beads/formulas/` |
| What a gate becomes | A bead of its own, type `gate`, blocking its step. Four steps with gates poured as 10 beads |
| When a gate is ready | At the pour. A gate waits on nothing before it, so the Refine gate could be closed before Propose was |
| What `bd mol current` tells an agent | It lists every open gate as `[ready]` and offers one as the next step: `Next ready: spk-mol-fdz - Gate: human. Start with: bd update spk-mol-fdz --claim`. With Propose done it picked Refine's gate, not Walk's. `bd ready --mol` does the same. Plain `bd ready` hides gates |
| Can an agent close a gate | Yes. `bd close` on a human gate as `agent-j1` printed `✓ Closed`. Nothing checks who closes |
| Can an agent claim a step its gate still holds | Yes. `bd update --claim` on Refine with its gate open printed `✓ Updated`, and the step read `in_progress` |
| Can an agent close a step its gate still holds | No. `cannot close blocked issue: spk-mol-clt is blocked by [spk-mol-2e6 spk-mol-0x0] (use --force to override)` |
| What a claim carries | A lease of 5 minutes (`lease_expires_at`). Only the holder can `bd heartbeat` it, so a step worked for longer than that is reaped by `bd reclaim` unless it is kept alive |
| Who closed it, afterwards | `bd history` names the author as `beads` or `root`. Only the events journal names the actor (`"op":"close","actor":"agent-j1"`), and it is off until `bd config set events-journal true` |
| A step dropped by `condition` | Its edges go with it. With `seen=false`, Refine lost its need on Propose as well as on Walk, and was held only by its gate |
| `gh:pr`, merged (#3277) | `bd gate check` resolved it |
| `gh:pr`, closed unmerged (#2990) | Printed `ESCALATE`, left the gate open, and wrote nothing to the bead: no label, status or comment |
| `gh:pr`, open (#3279) | `pending`, whatever its checks say. Red `ci`, a conflict and a queue drop never move it |
| `gh:pr` with no number | `pending - no PR number specified`. `bd update <gate> --await-id <n>` sets it after the pour, and the formula's `repo` field was used |
| Human gate past its timeout | Never looked at. `bd gate check --type=human` checked 0 gates; a 1-second human gate stayed open |
| Timer gate | Resolved once its timeout passed |
| Bead gate | Resolved once the bead it names closed |
| After Fleet resolves a gate | `bd ready --gated` named the molecule, the gate and the step now ready: `Ready step: spk-mol-7oy - Walk` |
| The molecule's own bead | Closed by itself when its last step closed. A plain epic does not: `bd mol stale` lists it with `Close with: bd close` |
| A plan with two groups side by side and one after both | `bd swarm validate` read two waves, `max parallelism: 2`. `bd swarm create` made a swarm molecule, an open bead of its own |
| Leases | `bd heartbeat` refused anyone but the holder. Leases are kept per machine and not synced, and `bd reclaim` reaps only this machine's |
| Merge slot | A second `acquire` was refused, naming the holder. A `release` by someone who did not hold it succeeded |
| Wisp | Not in `bd list`, not synced. `bd mol squash` left one closed `Digest:` bead and deleted the 7 steps |
| Pour | Its steps are ordinary beads and sync. Fleet's own open-issue query lists molecules, gates and steps beside the backlog |
| Time per read | 2.5 s for `bd show`, 3.5 s for `bd gate list`, 4.5 s for `bd ready --mol`, 4.9 s for `bd mol current` |

## What it means for Armada

- **Fleet resolves every gate, from what it sees**, with `bd gate resolve`.
  `bd gate check` alone would never resolve a human gate, never fire on red `ci`, a conflict or a queue drop, and leave an escalation unrecorded.
- **Fleet reopens a gate anyone else closed.** The events journal names the closer, so Fleet turns it on for a repository it runs a rail in and reads it after each sync.
- **An agent is not pointed at `bd mol current`.** It recommends gates. Fleet tells the agent its step, and the agent closes only its own step bead.
- **A gate needs the step before it as well**, written as a `needs` on the gated step, because the gate itself waits on nothing.
- **A step dropped by `condition` needs its edges written again**, or the step after it loses its order.
- **Fleet reads the molecule from its own record and the journal, never `bd` per notice.** One read takes seconds.
- **Pour or wisp is a choice between seeing it on another machine and keeping the backlog clean.** A poured Job syncs, which multi-machine work needs, and its beads then have to be filtered out of the Issues list.
- **The merge slot does not hold a lock**, since anyone can release it, and the merge queue already orders merges. It is not used.
