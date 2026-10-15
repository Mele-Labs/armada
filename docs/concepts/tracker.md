# Tracker

**What it is:** The adapter through which Fleet reads a repository's open issues and writes back on the one a Job came from. Bridge lists the issues from Fleet, and a person dispatches one by its address. Each issue is named by its tracker's own reference: `#123` on the forge, `ABC-123` on Jira.

---

**Kind:** Adapter. `#91`. The seam is `Tracker` in `crates/adapter-traits/src/tracker.rs`. There are two implementations, and each repository's Manifest chooses between them: `crates/adapters/src/issue_tracker.rs`, against the same forge the rest of delivery assumes, and `crates/adapters/src/jira_tracker.rs`, [Jira](#jira). *Choosing a tracker* says how.

**An issue's identity is its reference and its address, never a bare number.** A number names nothing once a tracker has more than one project. `TrackedIssue` and `IssueRead` carry `reference` as text, `get_issue` takes it, Autopilot's picker is offered and answers it, and a write-back goes by the address (`url`). Bridge draws the reference as it is.

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
| `project` | one of `project` and `jql` | A project key, or several separated by commas: capitals, digits and underscores. `ABC, OPS` |
| `jql` | one of `project` and `jql` | A filter, ANDed into the search. It takes no `ORDER BY` |

**The backlog is whatever the block selects.** `project` alone lists those projects, `jql` alone lists what it matches on the site, and both together list the `jql` within those projects. A block naming neither is refused, since it would list every open issue on the site.

**Needs on the machine Fleet runs on:** `jq` on the PATH, and `acli` on the PATH and signed in to the site. Either missing is said as the reason the tracker refused, naming the tool.

| Act | What Jira sees |
|---|---|
| List the open issues | `project = "ABC" AND statusCategory != Done`, or `project in ("ABC", "OPS") AND ...` for several, or no project clause where only `jql` is given, plus `jql` in parentheses, newest edit first. Whatever the search returns is listed, each under its key |
| Read one issue | By its key, `ABC-123`, in any project. A reference that is not a key, such as `123`, names no issue and is a 404 without reaching `acli` |
| Dispatch by link | Only `https://<site>.atlassian.net/browse/ABC-123` is recognised, any project's key. A bare number or another host is not |
| Comment | `acli jira workitem comment create` on the key in the issue's address |
| Armada's labels | `armada: in progress` is the label `armada-in-progress`, and `armada: set aside` is `armada-set-aside`: Jira labels hold no whitespace. A label taken off is written with `--remove-labels` alone. A label put on is written as the issue's whole label set, since the reference does not say whether `--labels` adds or replaces |
| Close the issue | The tracker returns no closing line, because nothing in a pull request body closes a Jira issue. The pull request **title** is prefixed with the key (`ABC-123 The title`), which Jira's integration with the forge reads, and left alone where it already holds the key |

**What the `acli` reference settles about labels.** `https://developer.atlassian.com/cloud/acli/reference/commands/jira-workitem-edit/` lists `--labels` as "Edit the labels" and `--remove-labels` as "Remove the labels", and gives no example of either. A removal has its own flag, so it is written as a removal and reads nothing first: a label somebody else sets at the same moment is not touched. Whether `--labels` appends to the labels or replaces them is not said, so an addition still reads the issue's labels and writes them all. **That write can lose a label set between the read and the write.** It stays until someone runs the edit against a real issue and sees which it does.

**Not verified against a real `acli`.** The output shapes below were written from the documentation and are unconfirmed. Every filter reads both `.fields.x` and a flat `.x`, and flattens a rich-text description to plain text, so a plausible variation reads. Whoever first points a repository at Jira runs these and compares, with a real key and project:

```bash
acli jira workitem search --jql 'project = "ABC" AND statusCategory != Done' --fields key,summary,labels,assignee,updated,description --json --limit 3
acli jira workitem view ABC-1 --fields key,summary,status,resolution,labels --json
```

What to look for: each issue has `key` and `fields.summary` (or `summary`), `fields.labels` is a list of strings, `fields.updated` is a timestamp, `fields.status` is an object with `statusCategory.key`, and `fields.description` is text or a rich-text document. Also unconfirmed: whether `acli jira workitem edit --labels` adds to the labels or replaces them, and the wording `acli` prints for an issue that does not exist, which `read_issue` matches on (`does not exist`, `not found`, `404`).

## What a person sees

| Surface | What it draws |
|---|---|
| The composer | The repository's open issues under the request, newest edit first. Each has Dispatch, or *A job is on it* where a Job from it has not ended |
| The issue | The two comments, Armada's label, and the closing on the pull request: its line in the body, or the key in its title |
| Each row | The issue's reference as its tracker writes it: `#123`, `ABC-123`. Autopilot's Next up and its picks draw the same |
| The Job's log | A warning when a write was refused, with the tracker's own words |

## What it does not do yet

| Missing | Waiting on it |
|---|---|
| An issue named by a key in the `#` picker | `#` types digits, so a Jira issue is left out of it. It is in the issue list and Autopilot's |
| A label put on without a read | Until `--labels` is known to add rather than replace |
| A Job's address outside Bridge | The started comment names the Job by number and title, since there is no link to give |
| A milestone's issues on their own | `../journeys/dispatch-a-milestone.md`, step 1 |
| A Drone reading the same issues through a tool | `../capabilities/epic-as-one-job.md` |
| The Cockpit lane Autopilot chooses from | Autopilot's Bridge half, `#2092`. Its picker already reads this list, held the same five minutes: [autopilot.md](autopilot.md) |
