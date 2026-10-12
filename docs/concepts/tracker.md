# Tracker

**What it is:** The adapter through which Fleet reads a repository's open issues and writes back on the one a Job came from. Bridge lists the issues from Fleet, and a person dispatches one by its address.

---

**Kind:** Adapter. `#91`. The seam is `Tracker` in `crates/adapter-traits/src/tracker.rs`; the one implementation is `crates/adapters/src/issue_tracker.rs`, against the same forge the rest of delivery assumes.

## Why Fleet reads it, and not a Drone

Bridge talks to Fleet and nothing else, and a person choosing what to dispatch has no Drone running. So the backlog in Bridge needs Fleet itself to read the tracker. `../capabilities/epic-as-one-job.md`, *The tracker is an adapter for Bridge*, carries the argument. The credential stays where the forge CLI already keeps it, in the operator's own environment, and no Drone receives it.

## What it does

| Act | When | Where |
|---|---|---|
| List the open issues | On ask from Bridge, held five minutes, or read again on refresh | `crates/fleet/src/tracking.rs`, `list_issues` |
| Comment that a Job started | The first pass after the Job leaves the queue | `crates/fleet/src/writing_back.rs` |
| Comment the pull request's address | The first pass after the Job opens one, and again if it opens another | the same |
| Put a label on | `armada: in progress` while it works; `armada: set aside` while it is escalated or its retries are spent | the same |
| Take Armada's label off | When the Job ends, landed or not | the same |
| Close the issue | Never directly. The pull request body carries the tracker's closing line, so merging it closes the issue | `closing_its_issue`, read by `crates/fleet/src/delivery.rs` |

## Rules

> **Rule.** The list is a view. Listing creates no Job and writes nothing on the tracker.
> Why: the owner's call on `#91`, 11 Oct 2026. Nothing enters Armada until a person dispatches an issue or Autopilot picks one.

> **Rule.** Dispatching an issue is an ordinary dispatch whose request is the issue's address.
> Why: the proposer already resolves an issue link and records the Job's `IssueSource` (`../concepts/job-proposer.md`). A second dispatch path would be a second way for a Job to name its issue.

> **Rule.** A write-back never fails, holds or slows a Job.
> Why: it runs in its own pass on the noticing interval, and nothing a Job does waits on it. A refused write is kept on the record, said on the Job's log, and tried again after ten minutes.

> **Rule.** Nothing is said twice.
> Why: a comment is not idempotent on the tracker. What has been said on each Job's issue is a row in `job_issue_writeback`, read before every write.

**Only Armada's own two labels move**, and only the one Armada put on. A label of the same name that somebody set by hand is moved with it, which is the cost the owner accepted on `#91`.

**A Job dispatched before this existed gets nothing written.** The migration marks every Job that already came from an issue as settled.

## What a person sees

| Surface | What it draws |
|---|---|
| The composer | The repository's open issues under the request, newest edit first. Each has Dispatch, or *A job is on it* where a Job from it has not ended |
| The issue | The two comments, Armada's label, and the closing line on the pull request |
| The Job's log | A warning when a write was refused, with the tracker's own words |

## What it does not do yet

| Missing | Waiting on it |
|---|---|
| A Job's address outside Bridge | The started comment names the Job by number and title, since there is no link to give |
| A milestone's issues on their own | `../journeys/dispatch-a-milestone.md`, step 1 |
| A Drone reading the same issues through a tool | `../capabilities/epic-as-one-job.md` |
| The Cockpit lane Autopilot chooses from | Autopilot's Bridge half, `#2092`. Its picker already reads this list, held the same five minutes: [autopilot.md](autopilot.md) |
