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
is built today, and why, is `merge-line.md` (*Needs*) and `docs/concepts/fleet.md`
(*Declared needs*).

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

A need is one more kind of attachment on the session and Job ledger
(`fleet/session-ledger`), under the same holder as a slot, a branch or a pull
request.

| Field | Meaning |
| --- | --- |
| `holder` | `{kind: session or job, id}`, the same shape every ledger row has |
| `path` | The file or resource, repository-relative: `protocol-version.toml` |
| `what` | The declarer's own words: *a minor* |
| `took` | What it chose, recorded once chosen: `23.41`. Optional until then |
| `state` | `standing`, `spent`, or `given_back` |
| `declared_at` | When it was declared. **The order is this, then the holder id** |

`spent` is a need whose holder's work merged. `given_back` is one released by a
person, or whose holder was dropped, ended or deleted its branch. A released slot
and a spent need read the same way in the ledger on purpose: both are a lease with
an explicit state, never a row that disappears.

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
or Session that owns it where the ledger knows one, then stops reading the files.
Nothing is deleted until a person has seen the list in Bridge.

## Not decided

- Whether a need expires by time. It does not today.
- The row and intake event shapes, which wait on the ledger landing.
- What a repository without Fleet gets. It has no order and no status.
- Whether the required status is Fleet's alone or also a repository workflow.
