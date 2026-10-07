# Session

**What it is:** An agent session a person runs, in a terminal today and in Bridge later, kept as one row on a ledger so Fleet can say which sessions are open and which one holds a branch, a pull request, a Job or a slot.

---

**Kind:** Record.

You have five terminal sessions open across three repositories. One is in slot 3, one opened pull request 1853, and one dispatched a Job an hour ago. You ask who has `fleet/session-ledger`, and the answer is a session by its title, still running, with the slot it stands in.

## What it is

> **Rule.** A session from a terminal is a first-class row, the same shape a session Bridge hosts will have.
> Why: the question is who holds a thing, and it should not depend on where the session was started.

> **Rule.** A session is reported as it happens and is never read back from.
> Why: the third attempt in [scope](../scope.md) was a conversation read as a claim. The ledger keeps what a session took and did, as structure, and nothing downstream reads a transcript.

> **Rule.** Fleet never waits on a session and a session never waits on Fleet.
> Why: a terminal session is the person's own, and Fleet being down, slow or gone must cost it nothing.

A session is not [Helm](helm.md), which is Armada's own conversation with Fleet, and not a [Drone](drone.md), which Fleet starts, places in a worktree and gates. A session started by a person was none of those, and until the ledger Fleet could not see it.

## How a session gets in

The harness sends Fleet facts in Armada's own shape and nothing past the adapter that sends them knows what the session runs in.

```
 harness (mod in a terminal session)          Fleet
 ───────────────────────────────────          ─────────────────────────────
 session starts ── started ───────────────▶  POST /sessions/report
 git, gh, dispatch, subagent, message ────▶   stamp, resolve cwd → repository, slot
 turn ends, usage measured ───────────────▶   keep in sessions, ledger_attachments
 session ends ─── ended ──────────────────▶   publish session.changed, whole
```

| `fact` | Carries | Fleet does |
|---|---|---|
| `started` | `cwd`, `title?`, `origin?` | Creates the row or starts a resumed one again; resolves the directory to a repository and a pool slot |
| `titled` | `title` | Sets it |
| `moved` | `cwd` | Re-resolves; a slot the session left is given back |
| `attached` | `attachment`: `kind`, `target`, `detail?` | Takes it; `branch` and `slot` replace the one held |
| `settled` | `attachment`: `kind`, `target`, `state` | Moves it to `spent` or `given_back` |
| `measured` | `usage`: `context_tokens?`, `context_window?`, `cost_micros?` | Keeps what was reported |
| `turn_completed` | nothing | Stamps the last turn |
| `ended` | `reason` | Ends it and gives back everything it still holds |

**No instant is reported**: Fleet stamps each fact on arrival. **A repeated fact publishes nothing**, so a harness may report on every turn.

The mod for Claude Code is `plugins/armada/`, and `crates/adapters` is the only Rust that names the harness. It sends no message text and no prompt apart from the first line of the first, as a title.

## What is kept

Two tables, in `crates/store`. The second is not the first's child.

| Table | One row is | Key |
|---|---|---|
| `sessions` | A session: harness, origin, repository, directory, title, `live` or `ended`, when it started, was last seen and last finished a turn, the figures it last reported | `id`, the harness's own |
| `ledger_attachments` | One thing a holder took or did | `holder_kind`, `holder_id`, `kind`, `manifest_id`, `target` |

A holder is `{ kind: session | job, id }` and points at neither table, so a Job or a need can hold rows without a table of its own. **A Job's own slot and branch are rows held by the Job**, written where Fleet leases the slot and records the branch, so `who_owns` names a Job that holds a slot. A `kind` is open text:

| `kind` | `target` | Held until |
|---|---|---|
| `slot` | the slot's number | the session leaves it or ends |
| `branch` | the branch's name | the session checks out another or ends |
| `pr` | the pull request's number | it merges (`spent`) or closes (`given_back`) |
| `job` | the Job's id | nothing settles it yet |
| `need` | the path, repository-relative | its holder's work merges (`spent`), or the holder is gone or releases it (`given_back`). Not exclusive: `docs/capabilities/needs.md` |
| `subagent` | the agent's id | |
| `message` | `to:<who>` or `from:<who>`, with a count in `detail` | |
| `studio` | the Studio's id | not reported by anything yet |

**State is one vocabulary for every kind.** `standing` is in force, `spent` is used and done with, and `given_back` was let go before it was used. A released slot and a spent need read the same way.

## What Fleet answers

| Operation | Answers |
|---|---|
| `list_sessions` | Every session, most recently seen first, narrowed by repository, `live` or `ended`, and `q`: a title, a branch, a pull request number, a Job id or a slot |
| `who_owns` | Every holder of a branch, a pull request, a Job or a slot, the ones still holding it first. Empty is an answer |
| `session.changed` | One session whole, whenever a fact changed something |

Both reads are on the agent's door, so a session can ask who else holds the branch it is about to take.

## Acts on a pull request

A pull request a session holds is a `pr` row, and a person can act on it without leaving the session. The pull request is named by its repository and number, and is nobody's Job.

| Operation | Does | Refuses |
|---|---|---|
| `get_pull_request` | Reads it: draft, open, merged or closed, auto-merge, the forge's checks (`pending`, `passed`, or `failed` with the failing names), title, branch, address | Where the forge would not answer |
| `ready_pull_request` | Takes a draft out of draft | Anything that is not a draft |
| `merge_pull_request_by_number` | Merges it | While the checks are running or failed, a draft, a closed one, and the forge's own refusals |
| `enable_auto_merge` | Asks the forge to merge when the checks pass | Once they have passed or failed, a draft, a closed one |
| `review_pull_request` | Drafts a Code Review Job against it, by number or address | A name that is neither, a Session the ledger never heard of |

```
 a person presses       Fleet                                  the forge
 ───────────────        ──────────────────────────────────     ─────────
 merge ───────────────▶ read state + checks ────────────────▶ gh pr view, gh pr checks
                        checks not passed ─▶ refuse, no write
                        passed ────────────────────────────▶ gh pr merge --merge
                        read again, refresh every `pr` row   ◀─
 ◀─ the new state       merged ─▶ row `spent`, closed ─▶ `given_back`
```

> **Rule.** Every write is read first, and a refusal Fleet can name is named before the forge is touched.
> Why: a merge pressed while the checks run must reach nobody, and a count of what the forge was asked to write is the only proof it did not.

> **Rule.** The ledger follows the forge through its own attach.
> Why: a Session's `pr` row is what the harness reported, and `detail` (`state`, `auto_merge`, `checks`, `failing`, `title`, `branch`, `address`) is Fleet's reading of the same pull request, so one writer is not two.

**The three writes are a person's ask, `pushes to shared`.** Helm asks first, as for `merge_pull_request`, and the operations are `Helm only`. Bridge's own buttons are the person's press and reach the routes directly. `get_pull_request` is open to any agent.

**A review is at the approval gate**, like every dispatch, with `origin` `session_dispatched` (*From a Session, by you*) and the pull request as the Job's subject. The workflow is `code_review`, named by Fleet and never read off a link by the proposer. Where a Session pressed it, the Session holds a `job` row whose `detail.origin` says *dispatched from Session <id>*, which is how the Board row and `who_owns` name it.

## Not built

**A session that dies without ending stays `live`.** Its `last_seen_at` is what says it has gone quiet; nothing yet checks the process.

**No Bridge surface.** The ledger is read through the two queries and the event.

**A message names where it went and not which session**, because the harness does not say: a delivery says whether it came from a peer or a teammate, and never a session id.

## Installing the Claude Code mod

```
claude plugin marketplace add <this repository>/plugins
claude plugin install armada@armada-local
```

It loads in the person's own sessions only. A Drone, a Judge call and a scout are started with `--setting-sources project,local`, so a mod installed in a person's user settings never loads in one ([spike 23](../spikes/023-does-a-user-installed-mod-load-in-a-drone.md)).
