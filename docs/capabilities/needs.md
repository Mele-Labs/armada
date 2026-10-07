---
capability: needs
issue: 1059
milestone: Throughput
---

# A Job or a Session says what it needs, and Armada keeps one log

Two agents both mean to bump the protocol minor. Today the second one finds out
when its merge fails and it renumbers. A need is how the second one hears, while
it is still working, that the first is already ahead and what it took.

This page is where the need moves once Jobs and Sessions share one ledger. What
is built, and why, is `merge-line.md` (*Needs*) and `docs/concepts/fleet.md`
(*Declared needs*).

## Where it stands

| Part | State |
| --- | --- |
| The row, and the one order a Job and a Session share | Built, protocol 23.46 |
| `armada need` asking Fleet (`act_on_need`, `list_needs`) | Built; where Fleet is not running it says so and records nothing |
| A Job's own slot and branch as rows | Built |
| Converting a clone's files | Built, read once at first start, files left on disk |
| The required status on a pull request | Not built |
| The `armada` mod reporting a need from a terminal Session | Not built |
| A Bridge list, and giving a need back from Bridge | Not built |
| `armada land` | Still reads the files, and is not moved: it is being retired for pull requests |

## What a person sees

- **One list of what is wanted and by whom**, in Bridge beside the ledger of
  slots, branches and pull requests: the file or resource, what was said, who holds
  it, who is waiting behind, and what was taken.
- **A Job or a Session that declares a need on something another holds is told
  at once**, as a message that wakes a Session and as the peer turn for a Job. It
  hears who is ahead and what that one took.
- **A pull request waits its turn.** Its status reads pending while a need ahead
  has not merged, and green when the turn is its own.
- **A stalled need is given back from Bridge**, not a terminal.

## The row

A need is a row in `ledger_attachments`, the table the session ledger keeps
(`crates/store/src/session_ledger.rs`), under the same holder as a slot, a branch
or a pull request. The kind column is open text, so a need adds no migration.

| Column | For a need |
| --- | --- |
| `holder_kind`, `holder_id` | `session` or `job`, and its id. A Job holds a need as a Job does a slot |
| `kind` | `need` |
| `manifest_id` | The repository the path belongs to |
| `target` | The file or resource, repository-relative: `protocol-version.toml` |
| `detail` | `{what, took}`: the declarer's words (*a minor*), and what it chose (`23.41`), absent until chosen. Fleet adds `branch` where it declares for a branch, so the line that names who is ahead can say it |
| `state` | `standing`, `spent` (the holder's work merged), or `given_back` (released, or the holder is gone) |
| `since`, `changed_at` | When declared, and when the state last changed |

**The order is `since`, then the holder id.** A need is **not exclusive**: the
ledger's exclusive attach gives back a holder's other standing rows of one kind
and repository, which is right for a slot and a branch and wrong here, because
one Job may wait on a protocol minor and on a second path at once.

**Reading it.** `who_owns` (`GET /sessions/owner?kind=need&target=&manifest_id=`)
returns standing holders first, so *who is ahead on this path* is that answer
minus the asker. `list_needs` (`GET /needs`) is the same rows for one repository
in the order they are served, each holder named as a person would. A Session declares through the intake
(`POST /sessions/report`, fact `attached` with `{kind: need, target, detail}`),
and gives one back or spends it with fact `settled`. A Job's needs are written by
Fleet, from `declare_scope`, a plan's task and `add_task`, on the same table.
**`armada need` names a branch and Fleet finds the holder**: the Job whose branch
it is, else a Session standing on it, else the branch alone, held as a `session`
row with the id `branch:<name>`. A need held by a branch alone is given back when
git no longer has the branch.

## The rules, one for both holders

- **First to declare goes first.** The answer to a declaration is the standing
  needs ahead on that path, each with its `what` and `took`.
- **A merge waits for every standing need ahead of it on the same path.** A
  pull request's status reads Fleet's order; `fleet.merge_waiting_behind` refuses
  Fleet's own press the same way.
- **Nothing expires by time.** A person gives a stalled one back.
- **A holder that is gone gives its needs back**: a Job dropped, a Session
  ended, a branch deleted.
- **A late declarer is told to search for the number it already used**, when its
  branch already changes the path.
- **A minor taken with no need is refused**, as `adapters::undeclared` does now;
  it becomes a required check on the pull request.

## What moves

| Today | After |
| --- | --- |
| One JSON file per need under `.git/armada-needs/`, one clone | A table in Fleet's store, one row per need |
| `armada need` writes the files | `armada need` asks Fleet, from any session or machine |
| A Job declares through `declare_scope`, `record_plan`, `add_task` | Unchanged for the Job; Fleet writes the ledger row |
| `armada land` holds a branch behind its needs | The pull request's required status, published by Fleet |
| A terminal Session cannot declare | The `armada` mod reports the need through the ledger channel |

**Converting what exists.** On the first start with the table, Fleet reads each
clone's `armada-needs/` files into rows, with the branch name resolved to the Job
that owns it where there is a live one, else held by the branch alone, in the
order the clone had, then stops reading the files. A row of kind `needs_files`
records that it was done. Nothing is deleted until a person has seen the list in
Bridge. A repository added after Fleet started is converted on the next start.

## Not decided

- Whether a need expires by time. It does not today.
- What a repository without Fleet gets. It has no order and no status.
- Whether the required status is Fleet's alone or also a repository workflow.
