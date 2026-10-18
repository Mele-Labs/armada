---
capability: merge-line
issue: 1315
milestone: Throughput
---

# Merges take turns, and each one is checked against the main it lands on

> Work goes through pull requests and the `ci` check. `armada land` and
> `scripts/land` are retired, and Fleet no longer reads the files an earlier run
> left under `.git/armada-land/`: the queue and outcome lists are always empty on
> the wire.

## What replaces each part

GitHub's pull requests and the `checks` workflow land this repository's code
([the decision](../../.claude/decisions/2026-10-06-ci-and-pull-requests-replace-the-merge-line.md)).
What stays of the line is main going red, and Bridge's panel.
The rest of the line's design, the sections the code and other pages cite by
name, is archived: `git show ecfffb2c1ebd:docs/capabilities/merge-line.md`.

| The line's part | What replaces it |
|---|---|
| Queue and turn | GitHub's merge queue |
| Rerunning what `main` moved under | The `plan` job, through `armada covers`, as in the archived *Choosing what reruns* |
| `verify-foundations` on every turn | A `foundations` job, on the branch `ci-foundations` and not merged. Until it lands CI does not run it |
| Batching | The merge queue's own grouping |
| Bridge's panel | A thin view of every open pull request and how Fleet knows them to connect, by Job links. Not built |

Two branches can each pass every Check, merge a minute apart, and leave `main`
red. Each one's Checks ran against a `main` that had moved by the time it
merged.

**Merges wait in line; work does not.** Branches are worked in parallel, and
a write clash is a warning ([Fleet](../concepts/fleet.md),
*Write clashes*). Only the step onto `main` takes turns.

`armada land` was the local stand-in, for agents working this repository
outside Fleet, and is gone.
