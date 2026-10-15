# Tracker

**What it is:** The adapter through which Fleet reads a repository's open issues and writes back on the one a Job came from. Bridge lists the issues from Fleet, and a person dispatches one by its address.

---

**Kind:** Adapter. `#91`. The seam is `Tracker` in `crates/adapter-traits/src/tracker.rs`. There are two implementations, and each repository's Manifest chooses between them: `crates/adapters/src/issue_tracker.rs`, against the same forge the rest of delivery assumes, and `crates/adapters/src/jira_tracker.rs`, [Jira](#jira). *Choosing a tracker* says how.

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
| Close the issue | Never directly. The pull request carries the tracker's closing, so merging it closes the issue: a closing line in the body, or a mark in the title where the tracker links by title | `closing_its_issue`, read by `crates/fleet/src/delivery.rs` |

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

## Choosing a tracker

A repository's issues are not always where its pull requests are. The Manifest's top-level `tracker:` block says which tracker holds them, and Fleet asks the one each repository chose, never one for all.

```yaml
tracker:
  provider: jira
  site: acme
  project: ABC
```

| Key | Meaning |
|---|---|
| `provider` | Which implementation. Absent, or `github`, is the forge's own tracker, as before the key existed |
| every other key | Carried as written, for that provider to read. The central schema holds `provider` and nothing else, so a new tracker is not a change to `config` (`../contracts/adapters.md`) |

> **Rule.** A block that cannot be used never stops Fleet.
> Why: a provider this build has no word for, a missing key or a stray one makes that repository's tracker answer every call with the reason, as any refused tracker does. The other repositories are not touched, and the owner is never sent to a Manifest because Fleet would not start.

The choice is read when Fleet starts, so a save that changes it is reported as waiting for a restart (`manifest.md`, *`tracker` and `ci_gate`*). The match from provider to implementation is in `crates/adapters/src/tracker_providers.rs`, because the provider words are vendors', and the composition root names that one type and no other.

## Jira

The `jira` provider reads and writes through `acli`, Atlassian's command line, and reduces its JSON with `jq`. The credential stays where `acli` keeps it, in the operator's own environment, as it does for the forge's CLI.

| Key | Required | Meaning |
|---|---|---|
| `site` | yes | `acme` for `acme.atlassian.net`, or a full host where it has a dot |
| `project` | yes | The project key: capitals, digits and underscores |
| `jql` | no | An extra filter, ANDed into the search. It takes no `ORDER BY` |

**Needs on the machine Fleet runs on:** `jq` on the PATH, and `acli` on the PATH and signed in to the site. Either missing is said as the reason the tracker refused, naming the tool.

| Act | What Jira sees |
|---|---|
| List the open issues | `project = "ABC" AND statusCategory != Done`, plus `jql` in parentheses, newest edit first |
| Read one issue by number | `ABC-N`. **The project key is always in the search, so the number alone names the issue**, and `OpenIssue.number` stays a number |
| Dispatch by link | Only `https://<site>.atlassian.net/browse/ABC-123` is recognised, any project's key. A bare number or another host is not |
| Comment | `acli jira workitem comment create` on the key in the issue's address |
| Armada's labels | `armada: in progress` is the label `armada-in-progress`, and `armada: set aside` is `armada-set-aside`: Jira labels hold no whitespace. The whole label set is written, the issue's own labels plus the added ones less the removed, so the write is right whether `--labels` adds or replaces |
| Close the issue | The tracker returns no closing line, because nothing in a pull request body closes a Jira issue. The pull request **title** is prefixed with the key (`ABC-123 The title`), which Jira's integration with the forge reads, and left alone where it already holds the key |

Only a key in the configured project is listed, whatever a custom `jql` let through.

**Not verified against a real `acli`.** The output shapes below were written from the documentation and are unconfirmed. Every filter reads both `.fields.x` and a flat `.x`, and flattens a rich-text description to plain text, so a plausible variation reads. Whoever first points a repository at Jira runs these and compares, with a real key and project:

```bash
acli jira workitem search --jql 'project = "ABC" AND statusCategory != Done' --fields key,summary,labels,assignee,updated,description --json --limit 3
acli jira workitem view ABC-1 --fields key,summary,status,resolution,labels --json
```

What to look for: each issue has `key` and `fields.summary` (or `summary`), `fields.labels` is a list of strings, `fields.updated` is a timestamp, `fields.status` is an object with `statusCategory.key`, and `fields.description` is text or a rich-text document. Also unconfirmed: whether `acli jira workitem edit --labels` adds to the labels or replaces them (the write above is right either way), and the wording `acli` prints for an issue that does not exist, which `read_issue` matches on (`does not exist`, `not found`, `404`).

## What a person sees

| Surface | What it draws |
|---|---|
| The composer | The repository's open issues under the request, newest edit first. Each has Dispatch, or *A job is on it* where a Job from it has not ended |
| The issue | The two comments, Armada's label, and the closing on the pull request: its line in the body, or the key in its title |
| The Job's log | A warning when a write was refused, with the tracker's own words |

## What it does not do yet

| Missing | Waiting on it |
|---|---|
| An issue's key in Bridge | Bridge draws `#123` for every issue, so Jira's `ABC-123` shows as `#123`. The wire carries a number only |
| A Jira issue's address when the project key is not the Manifest's | Only the configured project is listed and read by number. A link to another project's issue dispatches, and is not in the backlog |
| A Job's address outside Bridge | The started comment names the Job by number and title, since there is no link to give |
| A milestone's issues on their own | `../journeys/dispatch-a-milestone.md`, step 1 |
| A Drone reading the same issues through a tool | `../capabilities/epic-as-one-job.md` |
| The Cockpit lane Autopilot chooses from | Autopilot's Bridge half, `#2092`. Its picker already reads this list, held the same five minutes: [autopilot.md](autopilot.md) |
