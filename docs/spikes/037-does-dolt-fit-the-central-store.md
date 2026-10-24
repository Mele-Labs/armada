# Spike 37 — Does Dolt fit the central store, or Armada's work tracking?

**Answer for the store: no.**
Dolt solves merging copies that drifted apart, and the store's hard part is the opposite: one answer, at once, to "who fixes this red".
Two machines that claimed the same red against a Dolt remote both committed their claim, and the second learnt it had lost only at its push, as a merge conflict.
The one real fault in the store today, that a dropped run fact comes back from another machine, needs a tombstone row in the store as it stands, not a new database.

**Answer for work tracking: try it behind a flag, as a tracker mod.**
The owner chose Armada for the try; the mod `beads` reads Armada's issues from a beads database and passed its try.
Moving Armada's own issues off GitHub is a larger move than the tracker: its gates, docs and skills read GitHub issue numbers.

Measured 23 Oct 2026 with Dolt v2.6.0, released that day, on this Mac.
The question is the owner's, 23 Oct 2026; the second half is his note in the same Job.

## What the store does today, and where it hurts

The store is one Cloudflare Durable Object per account, `apps/store`, 550 lines of TypeScript over the object's own SQLite.
[Central store](../contracts/central-store.md) is the contract.

| Concern | Today | Hurts? |
|---|---|---|
| Two machines claim one red | The object runs one request at a time, so `INSERT OR IGNORE` is the claim: one wins, the other gets `409` and the winner | No. This is what the store is for |
| A store that cannot be reached | The take is refused; Fleet never fixes on a claim it could not make | No. Decided by the owner, 13 Oct 2026 |
| Pull request owners | Last write wins per key; only one machine opens a given pull request | No |
| Runs from two machines | Fleet reads the set, merges its own in and writes the whole set back, `crates/fleet/src/run_memory/cloud.rs` | **Yes, for drops.** Fleet's deny list, `run_facts_dropped`, is kept on the machine only. A fact the person dropped on one machine is written back by the next machine that still remembers it. An interleaved write loses facts only until the loser's next turn, which merges again |
| History and audit | A deleted claim is gone; a row keeps only its holder and `since` | Not yet. Nobody has asked who held a red last week |
| Conflicts | None by design: a claim is claim-if-free, a run set is replaced whole | No |
| Identity | GitHub's numeric id, read once at enrolment; the token is dropped; every call signed by a device key | No |

The drop fault was read from the code, not run.

## What Dolt would add

Dolt is a SQL database, MySQL on the wire, with git's branch, merge, diff and push over its own tables.

### The experiment

`037-run.sh` seeds a remote, clones it twice as machines `a` and `b`, then does two things.
`037-transcript.txt` is its output.

| Step | What happened |
|---|---|
| `a` and `b` each insert a claim on the same repository and red, and commit | Both commits succeed. Each machine believes it holds the red |
| `a` pushes | Accepted |
| `b` pushes | Refused: the remote moved, pull first |
| `b` pulls | `CONFLICT (content): Merge conflict in claims`; `dolt_conflicts_claims` holds `mac-b` as ours and `mac-a` as theirs |
| `a` deletes the run fact `cargo clippy` and pushes; `b` adds `pnpm dev` and pulls | The three-way merge keeps both: `b` holds `cargo test` and `pnpm dev`. The drop reached the other machine |
| `dolt_diff_claims` on `a` | One row: `mac-a` added, with the commit that did it |

The binary is 117 MB, from a 41 MB download. Each clone of this two-table database took 52 KB on disk.

### What it would cost

| Cost | What it means here |
|---|---|
| A server to host | Dolt is a Go binary; a Worker runs JavaScript or WebAssembly and has no place for it. The store would move to a machine or a container that someone keeps up |
| Hosted | Hosted Dolt starts at $50 a month. DoltHub keeps 100 MB of private data free and charges $5 a month to 5 GB, since April 2026. Either is an account beside GitHub |
| Its own auth | A `sql-server` has MySQL users and passwords; DoltHub has its own tokens. Neither knows a GitHub id, so the signing, the rate limits and the account-per-object shape would be rebuilt around it |
| A Git remote instead | Since February 2026 Dolt pushes to a Git remote under `refs/dolt/data`, so GitHub's own auth would do. But every machine then needs push rights to one repository, and every read fetches first |
| Offline | Dolt works offline and merges later. For a claim that is the double fix the store exists to stop: the contract refuses a take it cannot make, and a Dolt claim made offline is only learnt to be lost at the push |
| Names | Rows stay hashes either way. Dolt's history keeps every holder's machine name for good, which the store today forgets on delete |
| Fleet's side | No Rust client beyond MySQL; embedded Dolt is Go only. Fleet would hold a database connection, or shell out to a 117 MB binary |

## What needs nothing new

| Alternative | Fits |
|---|---|
| The current store, plus a dropped mark | A fact row the person dropped is kept as a tombstone with its time, and Fleet's merge honours it. Fixes the one fault; one table, one rule in the contract |
| The current store, plus a log table | If history is ever wanted, an append-only `claims_log` in the same object. Not built until asked |
| Git, in the shared home | A ref update with a lease is already compare-and-swap, which is what Dolt's Git remote rides on. It fits runs, which are a note; it is slow and push-gated for claims |
| SQLite | It is the store already: the Durable Object's storage is SQLite, and Fleet's own record is SQLite |

## Dolt for work tracking

The owner said he would be open to Dolt replacing GitHub issues for Armada's work tracking.
Dolt is a better fit here than for the store: issues are written by many hands, offline merge is welcome, and nothing is a lock.
Beads, the agent issue tracker the owner named, already runs on it: `bd`, with ids made from random UUIDs so machines never collide, `bd ready` for work with nothing blocking it, an atomic `bd update --claim` within one database, and `--json` on its commands.
DoltHub's April 2026 price cut followed its adoption.

What already makes a try cheap:

- **A tracker mod.** Fleet reads a tracker it has no adapter for through `tracker.toml`, over a `cli` transport, [Mods](../concepts/mods.md), *Tracker mods*.
  A mod that runs `bd … --json` lists and reads issues with no Fleet code, and beads syncs through the repository's own GitHub remote, so no new account.
- **Read only stays read only.** The tracker port never writes, so agents would file with `bd create` themselves, as they use `gh` today.
- **Beads' claim is not the store's.** It is atomic inside one database; two machines with their own copies are both told yes, and the sync settles it silently, [spike 38](038-how-far-behind-is-another-machine-on-beads.md).
- **Beads overlaps Armada's own plans.** `bd ready`, its dependencies and its claims cover what a Job's plan, its task list and claiming an issue cover, so tracking Armada's work in it would mean two answers to what is next and who has it.

### The try, on Armada

The owner chose to try it on Armada itself, 23 Oct 2026.
Beads 1.3.1 was installed with Homebrew, its usage metrics switched off, and a database made at `~/.armada/machine/beads/armada`, outside the repository and on this Mac only.

| Step | What happened |
|---|---|
| `bd github sync --pull-only`, with `gh`'s token in the environment | 955 of Armada's issues imported, 120 open and 835 closed, in 193 seconds. No token was written into the database. Nothing was written to GitHub |
| On disk | 62 MB |
| An imported issue | Keeps its GitHub address in `external_ref`, and gets a long id from its time and number, such as `armada-1792806783819-955-760bc69c`. Labels and assignees did not come across |
| `bd query … --json`, the 120 open | 0.75 seconds |
| `bd show <id> --json` | 0.45 seconds |
| The tracker mod `beads`, over `bd -C <database> --readonly` | `validate_mod` valid; `try_tracker_mod` against the Manifest `armada` passed, listing 100 and reading one |

A call runs from Fleet's own folder, not the repository's, so the mod names the database by its path with `-C`.
An issue made in beads has no page to open: its address is empty unless it came from GitHub.

What moving Armada's own issues would cost:

- Its docs autolink a bare `#123`, and `xtask/src/rules_design.rs` parses one.
- Its milestones, its pull requests that close issues and Fleet's GitHub tracker adapter all assume GitHub.
- Six skills under `.claude/skills/` call `gh issue`.
- A link the owner opens lands in a browser on GitHub today; a Dolt issue has no page unless DoltHub hosts it.

## Recommendation

| Question | Verdict |
|---|---|
| Dolt as the central store | **No.** File the dropped-fact tombstone against the store instead |
| Dolt as a tracker | **Try behind a flag**: the tracker mod `beads`, made and tried against Armada. A Manifest naming it is the flag |

## Not measured

- Latency of a push, pull or read against DoltHub or a GitHub remote: the experiment used a local file remote.
- A `sql-server`'s memory and start time, and Dolt under several concurrent writers.
- The dropped-fact fault in the running store: it was read from `run_memory/cloud.rs` and the `run_facts_dropped` migration.
- Hosted Dolt's price from DoltHub's own page: it is from its blog and from listings that cite it.
- The beads tracker as Armada's tracker: the Manifest still names GitHub, so no Job has read an issue from it.
- Beads across machines: the database was never pushed to a remote.
