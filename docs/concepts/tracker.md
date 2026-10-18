# Tracker

**What it is:** The adapter through which Fleet reads a repository's open issues.
Bridge lists the issues from Fleet, and a person starts a Job on one by its address.
Each issue is named by its tracker's own reference: `#123` on the forge, `ABC-123` on Jira.

---

**Kind:** Adapter.
`#91`.
The seam is `Tracker` in `crates/adapter-traits/src/tracker.rs`.
There are two implementations, and each repository's Manifest chooses between them: `crates/adapters/src/issue_tracker.rs`, against the same forge the rest of delivery assumes, and `crates/adapters/src/jira_tracker.rs`, [Jira](#jira).
*Choosing a tracker* says how.

**An issue's identity is its reference and its address, never a bare number.** A number names nothing once a tracker has more than one project.
`TrackedIssue` and `IssueRead` carry `reference` as text, `get_issue` takes it, and Autopilot's picker is offered and answers it.
Bridge draws the reference as it is.

## Why Fleet reads it, and not an agent

Bridge talks to Fleet and nothing else, and a person choosing what to start has no agent running.
So the backlog in Bridge needs Fleet itself to read the tracker.
The credential stays where the forge CLI already keeps it, in the operator's own environment, and no agent receives it.

## What it does

| Act | When | Where |
|---|---|---|
| List the open issues | On ask from Bridge, held five minutes, or read again on refresh | `crates/fleet/src/tracking.rs`, `list_issues` |
| Read one issue | By its reference | `read_issue` |

**Fleet never comments on, labels or edits an issue or a pull request on the owner's behalf**, unless he presses for it.
A review or comment he approves or types and sends from Bridge is his own act. #91 asked for write-back to the issue a Job came from, and it is not built.

## Rules

> **Rule.** The list is a view. Listing creates no Job and writes nothing on the tracker.
> Why: the owner's call on `#91`, 11 Oct 2026. Nothing enters Armada until a person starts a Job on an issue or Autopilot picks one.

> **Rule.** Starting a Job on an issue is an ordinary start whose request is the issue's address.
> Why: a second path would be a second way for a Job to name its issue.

## Choosing a tracker

A repository's issues are not always where its pull requests are.
The Manifest's top-level `tracker:` block says which tracker holds them, and Fleet asks the one each repository chose, never one for all.

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
> Why: a provider this build has no word for, a missing key or a stray one makes that repository's tracker answer every call with the reason, as any refused tracker does.
> The other repositories are not touched, and the owner is never sent to a Manifest because Fleet would not start.

The choice is read when Fleet starts, so a save that changes it is reported as waiting for a restart (`manifest.md`, *`tracker` and `ci_gate`*).
The match from provider to implementation is in `crates/adapters/src/tracker_providers.rs`, because the provider words are vendors', and the composition root names that one type and no other.

## Jira

The `jira` provider reads and writes through `acli`, Atlassian's command line, and reduces its JSON with `jq`.
The credential stays where `acli` keeps it, in the operator's own environment, as it does for the forge's CLI.

| Key | Required | Meaning |
|---|---|---|
| `site` | yes | `acme` for `acme.atlassian.net`, or a full host where it has a dot |
| `project` | one of `project` and `jql` | A project key, or several separated by commas: capitals, digits and underscores. `ABC, OPS` |
| `jql` | one of `project` and `jql` | A filter, ANDed into the search. It takes no `ORDER BY` |

**The backlog is whatever the block selects.** `project` alone lists those projects, `jql` alone lists what it matches on the site, and both together list the `jql` within those projects.
A block naming neither is refused, since it would list every open issue on the site.

**Needs on the machine Fleet runs on:** `jq` on the PATH, and `acli` on the PATH and signed in to the site. Either missing is said as the reason the tracker refused, naming the tool.

| Act | What Jira sees |
|---|---|
| List the open issues | `project = "ABC" AND statusCategory != Done`, or `project in ("ABC", "OPS") AND ...` for several, or no project clause where only `jql` is given, plus `jql` in parentheses, newest edit first. Whatever the search returns is listed, each under its key |
| Read one issue | By its key, `ABC-123`, in any project. A reference that is not a key, such as `123`, names no issue and is a 404 without reaching `acli` |
| Dispatch by link | Only `https://<site>.atlassian.net/browse/ABC-123` is recognised, any project's key. A bare number or another host is not |
| Close the issue | The tracker returns no closing line, because nothing in a pull request body closes a Jira issue. The pull request **title** is prefixed with the key (`ABC-123 The title`), which Jira's integration with the forge reads, and left alone where it already holds the key |

**Not verified against a real `acli`.** The output shapes below were written from the documentation and are unconfirmed.
Every filter reads both `.fields.x` and a flat `.x`, and flattens a rich-text description to plain text, so a plausible variation reads.
Whoever first points a repository at Jira runs these and compares, with a real key and project:

```bash
acli jira workitem search --jql 'project = "ABC" AND statusCategory != Done' --fields key,summary,labels,assignee,updated,description --json --limit 3
acli jira workitem view ABC-1 --fields key,summary,status,resolution,labels --json
```

What to look for: each issue has `key` and `fields.summary` (or `summary`), `fields.labels` is a list of strings, `fields.updated` is a timestamp, `fields.status` is an object with `statusCategory.key`, and `fields.description` is text or a rich-text document.
Also unconfirmed: the wording `acli` prints for an issue that does not exist, which `read_issue` matches on (`does not exist`, `not found`, `404`).

## What a person sees

| Surface | What it draws |
|---|---|
| The composer | The repository's open issues, newest edit first. `#` at the start of a word offers them by number, or by key where the tracker names issues by one |
| Each row | The issue's reference as its tracker writes it: `#123`, `ABC-123`. Autopilot's Next up and its picks draw the same |

## What it does not do yet

| Missing | Waiting on it |
|---|---|
| The Cockpit lane Autopilot chooses from | Autopilot's Bridge half, `#2092`. Its picker already reads this list, held the same five minutes: [autopilot.md](autopilot.md) |
