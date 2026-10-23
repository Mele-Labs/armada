# Tracker

**What it is:** The adapter through which Fleet reads a repository's open issues.
Bridge lists the issues from Fleet, and a person starts a Job on one by its address.
Each issue is named by its tracker's own reference: `#123` on the forge, `ABC-123` on Jira.

---

**Kind:** Adapter.
`#91`.
The seam is `Tracker` in `crates/adapter-traits/src/tracker.rs`.
There are two implementations, and each repository's Manifest chooses between them: `crates/adapters/src/issue_tracker.rs`, against the same forge the rest of delivery assumes, and `crates/adapters/src/jira_tracker.rs`, [Jira](#jira).
Any other tracker is taught by a tracker mod, `crates/adapters/src/taught_tracker.rs`.
*Choosing a tracker* says how.

**An issue's identity is its reference and its address, never a bare number.** A number names nothing once a tracker has more than one project.
`TrackedIssue` and `IssueRead` carry `reference` as text, `get_issue` takes it, and Autopilot's picker is offered and answers it.
Bridge draws the reference as it is.
A Session's `issue` rows on the ledger are held the same way: the forge's number as digits, or a key such as `ABC-12`, and the pull reading reads each through the Manifest's tracker.
An issue tag, a plan group's `issue` and `claim_issue` all take either. [what-is-kept.md](session/what-is-kept.md).

## Why Fleet reads it, and not an agent

Bridge talks to Fleet and nothing else, and a person choosing what to start has no agent running.
So the backlog in Bridge needs Fleet itself to read the tracker.
The credential stays where the forge CLI already keeps it, in the operator's own environment, and no agent receives it.

## What it does

| Act | When | Where |
|---|---|---|
| List the open issues | On ask from Bridge, held five minutes, or read again on refresh | `crates/fleet/src/tracking.rs`, `list_issues` |
| Read one issue | By its reference | `read_issue` |
| Search the issues | A query in the tracker's own language, open or closed as it says, newest edit first, at most 100 | `search_issues` |
| Teach its query language | What a model needs to write a query, and the tracker's own Open and Assigned to me | `query_guide` |
| Say which issues a text links | Whenever a Job is dispatched or a message names a link, in the tracker's own words: `7` on the forge, `ABC-12` on Jira. None for a tracker that reads no links | `issues_linked_in` |
| Write a query from a person's words | On ask from Bridge, one model call | `crates/fleet/src/issue_queries.rs` |
| Run, save, edit and forget a query | On ask from Bridge, a saved one run as one tracker call | `crates/fleet/src/issue_queries.rs`, `crates/store/src/issue_queries.rs` |

**Fleet never comments on, labels or edits an issue or a pull request on the owner's behalf**, unless he presses for it.
A review or comment he approves or types and sends from Bridge is his own act. #91 asked for write-back to the issue a Job came from, and it is not built.

## Rules

> **Rule.** The list is a view. Listing creates no Job and writes nothing on the tracker.
> Why: the owner's call on `#91`, 11 Oct 2026. Nothing enters Armada until a person starts a Job on an issue or Autopilot picks one.

> **Rule.** Starting a Job on an issue is an ordinary start whose request is the issue's address.
> Why: a second path would be a second way for a Job to name its issue.

## Searching

**A query is written in the tracker's own language, and running it is one call.** The forge's search syntax, JQL, or whatever a tracker speaks.
No translation layer stands between them, so a tracker taught later brings its own language with it.
`QueryGuide` is what a model reads to write one: the language's name, prose on its syntax with examples and what it cannot ask, the two built-in queries Open and Assigned to me, and the names of the fields the tracker fills.
Assigned to me is the tracker's own word for whoever is signed in (`assignee:@me`, `currentUser()`), so no identity is looked up.

Every issue says its reference, title, address and state.
Its author, assignees, last edit and excerpt are said where the tracker has them, and empty where it does not.
Everything else is a named field, under the tracker's own name, each value a display string; a field with no values is left out.

| Tracker | Fields |
|---|---|
| The forge | `Labels`, `Milestone` (its title), `Projects` (each project's title, with its Status where it has one: `Roadmap: In progress`) |
| Jira | `Status`, `Priority`, `Issue type`, `Components`, `Fix versions`, `Labels`, `Sprint`, and each field the block's `fields` key names |

A token that may not read projects (`read:project`) still searches: it is asked again without them, and the rest is read.

## Saved queries and the query writer

**A person describes a filter in his own words, and one model call writes it as a query in the tracker's own language.**
The call has no tools and is given the tracker's `QueryGuide` and his words.
It is never given a ticket's title or body, so nothing a ticket says can reach it.
It answers a query and the names of the fields a row should show.
It runs on `models.review`, as Natural Search does.

**Fleet runs what was written before it answers.**
A query the tracker refuses fails the call with the tracker's own words, and so does a field the guide does not list.
The call is spent either way, and its cost is on the answer when it succeeds.
Nothing is kept until he saves.

**Saving makes no model call.**
It runs the query again and refuses one the tracker refuses.
With an id it edits that query in place, which is how he edits the text by hand, and the row keeps its place in the list.
Running a saved query later is one tracker call and no model spend.

**They are his, on this Mac.**
They live in Fleet's store, per repository (`crates/store/src/issue_queries.rs`), and survive a restart.
No Manifest holds them, so they are never committed to a repository.

**Every repository starts with two queries it does not save: Open and Assigned to me.**
They come from the tracker's guide, in its own language, so they need no identity and no model.

**A tracker that cannot be read says why, and Bridge draws that as the offer to teach Armada this tracker.**
A tracker that refuses every call puts its reason where the guide would be, and `list_issue_queries` answers it as `refused` with no guide.
Saved queries are still listed, so he can forget one.

| Act | Operation | Notes |
|---|---|---|
| Run one query | `search_issues` | A POST only because a query is typed text. Read from the tracker each time, never held |
| Guide summary and saved queries | `list_issue_queries` | Language, Open, Assigned to me, field names; or the tracker's refusal |
| Write a query from his words | `write_issue_query` | One call, the query already run, nothing kept |
| Keep or edit a query | `save_issue_query` | Runs the query first. No model |
| Drop a query | `forget_issue_query` | By id |

**A ticket shows the Jobs working it.**
`TrackedIssue.holders` and `IssueRead.holders` are Job ids read from the ledger's `issue` rows, so they read the same on every tracker.
A Job that gave the issue back is not one.
Bridge joins them with the Jobs it already has.

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
| `provider` | Which implementation. Absent, or `github`, is the forge's own tracker, as before the key existed. Any word this build has no tracker for names a tracker mod |
| every other key | Carried as written, for that provider to read. The central schema holds `provider` and nothing else, so a new tracker is not a change to `config` (`../contracts/adapters.md`). A tracker mod takes none |

> **Rule.** A block that cannot be used never stops Fleet.
> Why: a provider this build has no word for, a missing key or a stray one makes that repository's tracker answer every call with the reason, as any refused tracker does.
> The other repositories are not touched, and the owner is never sent to a Manifest because Fleet would not start.

The choice is read when Fleet starts, so a save that changes it is reported as waiting for a restart (`manifest.md`, *`tracker` and `ci_gate`*).
The match from provider to implementation is in `crates/adapters/src/tracker_providers.rs`, because the provider words are vendors', and the composition root names that one type and no other.

### A tracker Armada was not built for

A job teaches it: it finds the tracker's command line, API or MCP server, writes a tracker mod's `tracker.toml`, tries it against the repository with `try_tracker_mod`, and the Manifest names the mod.

```yaml
tracker:
  provider: linear
```

> **Rule.** A provider naming a tracker mod is used only while the mod is valid, switched on in Mods, and has passed a try of its file as it stands.
> Why: until it has answered once, nothing says the command or the address it names works, and every call is refused with the reason instead.

The mod is looked up at each call, so a try that passes is used without a restart, and an edit to `tracker.toml` refuses the mod until it is tried again.
A word that names neither a built-in tracker nor a mod is refused as before.
A taught tracker reads no link, so a request naming one of its issues by address is dispatched as written.
What the file says is [Mod](mods.md), *Tracker mods*.

## Jira

The `jira` provider reads and writes through `acli`, Atlassian's command line, and reduces its JSON with `jq`.
The credential stays where `acli` keeps it, in the operator's own environment, as it does for the forge's CLI.

| Key | Required | Meaning |
|---|---|---|
| `site` | yes | `acme` for `acme.atlassian.net`, or a full host where it has a dot |
| `project` | one of `project` and `jql` | A project key, or several separated by commas: capitals, digits and underscores. `ABC, OPS` |
| `jql` | one of `project` and `jql` | A filter, ANDed into the search. It takes no `ORDER BY` |
| `fields` | no | More fields a search reads, by Jira's id or name, separated by commas: `customfield_10016, Team`. Each is shown under the name written. None may hold a quote or a backslash |

**The backlog is whatever the block selects.** `project` alone lists those projects, `jql` alone lists what it matches on the site, and both together list the `jql` within those projects.
A block naming neither is refused, since it would list every open issue on the site.

**Needs on the machine Fleet runs on:** `jq` on the PATH, and `acli` on the PATH and signed in to the site. Either missing is said as the reason the tracker refused, naming the tool.

| Act | What Jira sees |
|---|---|
| List the open issues | `project = "ABC" AND statusCategory != Done`, or `project in ("ABC", "OPS") AND ...` for several, or no project clause where only `jql` is given, plus `jql` in parentheses, newest edit first. Whatever the search returns is listed, each under its key |
| Search the issues | What the block selects, `project` and `jql`, ANDed with the query in parentheses, then `ORDER BY updated DESC`. A query holding `ORDER BY` is refused, as `jql` is. The reporter is the author |
| Read one issue | By its key, `ABC-123`, in any project. A reference that is not a key, such as `123`, names no issue and is a 404 without reaching `acli` |
| Dispatch by link | Only `https://<site>.atlassian.net/browse/ABC-123` is recognised, any project's key, and a Job dispatched with one holds `ABC-123` on its ledger. A bare number or another host is not |
| Close the issue | The tracker returns no closing line, because nothing in a pull request body closes a Jira issue. The pull request **title** is prefixed with the key (`ABC-123 The title`), which Jira's integration with the forge reads, and left alone where it already holds the key |

**Not verified against a real `acli`.** The output shapes below were written from the documentation and are unconfirmed.
Every filter reads both `.fields.x` and a flat `.x`, and flattens a rich-text description to plain text, so a plausible variation reads.
Whoever first points a repository at Jira runs these and compares, with a real key and project:

```bash
acli jira workitem search --jql 'project = "ABC" AND statusCategory != Done' --fields key,summary,labels,assignee,updated,description --json --limit 3
acli jira workitem view ABC-1 --fields key,summary,status,resolution,labels --json
acli jira workitem search --jql 'project = "ABC"' --fields key,summary,reporter,status,resolution,priority,issuetype,components,fixVersions,sprint --json --limit 3
```

What to look for in the search: `fields.reporter` is a person, each of `priority`, `issuetype` and `status` an object with a `name`, `components`, `fixVersions` and `sprint` lists of objects with a `name`, and that `--fields sprint` is taken at all, since Sprint is a custom field on most sites.
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
