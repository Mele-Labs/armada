# When a Job's panes are dropped

armada-vs8t. Bridge keeps each Job's pane arrangement under `job-panes:job:<id>` and never removes it. This change drops a Job's key once the Job is gone. Everything turns on one decision: **when does a Job count as gone?**

## What Bridge knows

Bridge reads `GET /sessions` with no filter: every Job on this Mac, in every repository, live and ended. Each record has `state`, which is `live` or `ended`. Bridge works out `dead` from that: `ended`, or `quiet` for a live Job it has not heard from lately. An ended Job stays in the list and can still be opened, and its panes are drawn as they were. A failed re-read after a good one leaves the last list in place (`sessions-wired.tsx`, `take`). Before the first read, there is no list.

## The options

| Rule | What it costs |
|---|---|
| **A. Gone when it ends** (`state` is `ended`) | Keys stop growing once Jobs end. But an ended Job is still listed and can still be opened, and its panes would come back as the default layout. That changes a layout while its Job exists, which the brief rules out. A Job that is continued later also loses its arrangement. |
| **B. Gone when a successful read of the list no longer holds it** | No Job that is listed loses its layout, ended or quiet. The keys are bounded by the Jobs Fleet keeps, not by every Job there has ever been. If Fleet keeps ended Jobs forever, their layouts stay as long as the Jobs do. |
| **C. Gone when ended for more than N days** | The tightest bound. It still drops a layout that can be opened, just later, and it adds a time limit that nobody has asked for. |

## The pick: B

A Job is gone when a successful read of the list does not hold its id. Bridge checks this on **every** read. The first read after Bridge starts is the sweep for Jobs that went while it was closed, so the sweep needs no separate code and follows the same rule. Nothing is dropped before the first read, and a failed read changes nothing, because the last good list stays. Every key under `job-panes:job:` whose id is not in the list is dropped, whether it is open in this window or only kept from an earlier one. The Job's ⌘Z history and measured area go with the key, so if the same id is listed again it opens as a new Job on the layout for all. The saved layouts and the layout for all are the family's, not any one Job's, and are never touched.

**What B leaves open.** If a Bridge reads a different Fleet's list from the same kept storage, such as a development Fleet in the same profile, it would drop the other Fleet's Jobs' layouts. Today one profile talks to one Fleet, so I accept this cost. If Fleet ever stops listing ended Jobs past a retention period, B follows it with no change here.

## Pull requests and the Operator

- **Pull requests (`pr-panes:pr:<key>`)**: these grow the same way, but nothing in Bridge holds a full list of every pull request it ever drew. The Reviews page shows a filtered, current set, so a pull request missing from it is not gone. **My call:** the family gets the same drop-one-owner ability, because it lives in `createPaneFamily`, but no rule applies it yet. I file a follow-up for a rule based on merged or closed pull requests, instead of guessing one now.
- **Operator (`operator-panes:operator`)**: one owner with a fixed id. It never grows, so it is never swept.
