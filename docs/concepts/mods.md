# Mod

**What it is:** A folder on this machine that changes how Bridge looks, how it is arranged, what it links or what Fleet tells a Session, or teaches Fleet a tracker it was not built for, written by a person or by a job, which Fleet reads and checks and which can never stop Fleet from starting or from listing.

---

**Kind:** Record.

You ask a job for a warmer dark theme.
It makes a mod called `warm`, edits its colours and checks them; you pick `warm` in Settings, Theme, and Bridge redraws.
A month later you ask for it in the repository, and it is on a branch under `packages/mods/<name>/`, ready to open as a pull request.

## What it is

> **Rule.** A mod is a folder under the machine's `mods/` directory, and nothing else is one.
> Why: a job that can write a file can make one, without a build and without touching Bridge's source.

> **Rule.** Fleet stores and checks a mod, and Bridge is what applies a theme, a layout or a tweak.
> Why: Fleet never draws, and a mod is a look, an arrangement, a pattern to link or a rule for a notice. A tracker mod is the one kind Fleet runs itself, because Fleet is what reads a tracker.

> **Rule.** A broken mod is a row with `valid` false, and never a failure of Fleet's start or of `list_mods`.
> Why: the folder is written by anything on the machine, so every read of it is bounded and every fault is that mod's own.

> **Rule.** Each mod is a git repository of its own.
> Why: its history is kept where it was written, and promoting it is a copy of two files.

## Where it lives

`~/Library/Application Support/Armada/mods/<name>/`, beside `armada.db`.

| File | Holds |
|---|---|
| `mod.toml` | `name`, `kind`, `version`, `description` |
| `theme.css` | Token values Bridge injects, in a theme mod |
| `layout.json` | Which tabs, panels and places Bridge draws and in what order, in a layout mod. [Layout mods](layout-mods.md) |
| `words.toml` | A pattern Bridge links and where it opens in each repository, in a Words mod |
| `fleet.toml` | What Fleet does with a notice, by Job, repository or everywhere, in a Fleet mod |
| `tweak.css` | Token values layered over whichever theme is chosen, in a tweak mod |
| `tracker.toml` | How to list, search and read a tracker's issues, in a tracker mod. *Tracker mods* below |
| `.tried.toml` | A tracker mod's last try, written by Fleet and never promoted |
| `.git/` | The mod's own history, one commit at scaffold |

`name` is a slug of 1 to 40 lowercase letters, digits and `-`, starting with a letter or digit. `dark`, `light`, `system` and `default` are refused, because Bridge's own themes answer to them.

## mod.toml

A flat file of `key = "value"` lines and `#` comments, read by hand: a table, an array or a number is a problem.

| field | type | required | default | meaning |
|---|---|---|---|---|
| `name` | string | yes | | The folder's name |
| `kind` | string | yes | | `theme`, `layout`, `words`, `fleet`, `tweak` or `tracker` |
| `version` | string | yes | | Up to 32 letters, digits, `.`, `+`, `-` |
| `description` | string | no | none | Up to 200 characters |
| `taught_on` | string | no | none | What Teach was pressed on, up to 120 characters. The mod's row lists it |
| `entry` | string | no | none | The Logbook entry a teaching from a Job made. Forgetting that entry removes the mod |

A teaching writes a Words, Fleet, tweak or layout mod, and records `taught_on` and `entry` in it. [Teaching](teaching.md).

Any other key is a problem.
A `kind` this build does not have makes the mod invalid, so a valid mod's kind is always one Bridge can act on.
A theme mod holds `theme.css`, a layout mod `layout.json`, a Words mod `words.toml`, a Fleet mod `fleet.toml`, a tweak mod `tweak.css` and a tracker mod `tracker.toml`, and the kind says which file is read.

## theme.css

> **Rule.** A stylesheet is `:root` and `[data-theme]` blocks of `--token: value;` declarations, at most 32 KiB, and anything else makes the mod invalid.
> Why: a theme is read by whatever it is substituted into, so the accepted language is a whitelist and not a filter.

The tokens are the names in `packages/tokens/tokens.json`, compiled into Fleet, so a token added there is accepted by the next build.

| Accepted | Refused |
|---|---|
| `:root`, `[data-theme]`, `[data-theme="name"]`, `:root[data-theme="name"]` | Any other selector, a block in a block |
| A token name as the property | Any other property, any custom property that is not a token |
| Values of letters, digits, spaces and `# . , % + - * / ( ) _ ' " :` | `url(`, `image-set(`, `expression(`, `javascript:`, `data:`, `script` |
| `/* comments */` | `@import` and every other at-rule |
| | A backslash, `<`, `>`, `{`, `}`, `!`, a value over 512 characters |
| A regular file | A link, a pipe, a file that is not UTF-8 |

Every problem is returned with its line, up to 20.

## tweak.css

> **Rule.** A tweak is checked as `theme.css` is, token declarations and nothing else, and names `tweak.css` in its problems.
> Why: it is substituted into the same page a theme is, so it gets the same whitelist.

Bridge layers every tweak that is on and valid over whichever theme is chosen, in mod name order, and `validate_mod` returns its text as `css`.
A tweak sits in a sheet of its own with every selector raised above a theme's, so choosing another theme replaces the theme's sheet and leaves the tweak, and `--no-mods` lays none.
A tweak is never chosen as the theme.

## words.toml

> **Rule.** A Words mod holds a pattern learned once and an address for each repository that has one.
> Why: `ARM-123` links to one place in one repository and to another place in the next, and the pattern is the same.

```toml
name = "Jira key"
pattern = "[A-Z][A-Z0-9]+-\\d+"
example = "ARM-123"

[[address]]
repository = "armada"
template = "https://acme.example.net/browse/{match}"
found = "settings"
```

`repository` is a Manifest id, or `*` for a repository with no address of its own.
`found` is `settings` (the Manifest's tracker named the site), `pull_request #<number>` (a merged pull request's text held it) or `person`.
The pattern is held to what both Bridge's JavaScript engine and Fleet's read: no lookaround, backreference or inline flag, no repeated group that holds a repeat or a choice, at most 200 characters, and it matches `example` whole.
An address is an `http` or `https` address with `{match}` outside its host. At most 100 addresses and 16 KiB.

**A repository with no address is looked for, once, when a teaching or a read of the rules names it.**
Fleet asks the Manifest's tracker first (a Jira site gives its `browse` page, `found = "settings"`), then reads the text of up to five recently merged pull requests for a web address that holds a match of the pattern on its own in a path or query value (`found = "pull_request #<number>"`).
Where neither says, the Session whose person taught it is asked in Needs you, once for that mod and repository: they pick an address another repository has, or write one.
An address a person wrote is never replaced by one that was found.
`read_teaching` runs the same search before a save, so the proposal carries the address, and the save writes it without searching again.

## fleet.toml

> **Rule.** A Fleet mod names a notice Fleet sends, what to do with it, and where.
> Why: the same line is wanted once a day in one repository and never in another.

```toml
[[rule]]
notice = "behind_base"
does = "batch"
where = "repo armada"
```

`does` is `tell`, `quiet`, `batch` or `call`. `where` is `job <id>`, `repo <manifest id>` or `everywhere`.
`notice` is one of the words a notice goes by, so a word Fleet does not send is a problem.
**The nearest place wins** (a Job's own over its repository's over everywhere), and among rules of one place the later mod name does. At most 64 rules and 8 KiB.

A rule may also say which notices of its word it covers, and what to do instead when something is true of the Job:

```toml
[[rule]]
notice = "pr_checks_failed"
does = "batch"
where = "everywhere"
matches = "lint|fmt"
when = [{ if = "blocks_merge", does = "tell" }, { if = "owner_away", does = "quiet" }]
```

> **Rule.** A rule covers a notice of its word whose text `matches`, and the first condition of `when` that holds replaces its `does`.
> Why: "batch these, and tell me only when it blocks a merge" is one wish, and a rule that could only name a word would make it two mods and a guess.

`matches` is a pattern over the notice's text, held to what a Words pattern is held to (no lookaround, backreference or inline flag, no repeated group that holds a repeat or a choice, at most 200 characters, never the empty string) except that it need not match whole: it finds its words anywhere in the text.
A rule taught on a notice must find words in that notice's text. A bad pattern makes the mod invalid, with the reason.
Two rules for one word and place are different when their `matches` differ, and the nearest place of those whose pattern finds words in the notice wins.

`when` is a list of `{ if, does }` tried in order. **The conditions are a closed set**, and a word outside it makes the mod invalid:

| `if` | Holds when | Read from |
|---|---|---|
| `blocks_merge` | The Job has an open pull request that cannot merge now: its required checks failed, it conflicts, or the merge queue marked it unmergeable | The ledger's `pr` rows for the Job (their `checks`), and the forge reading the pull request watch last made (`mergeable`, the queue) |
| `open_pull_request` | The Job has an open pull request, a draft included | The ledger's `pr` rows for the Job, standing and `open` or `draft` |
| `working` | The Job's agent is in the middle of a turn | The Job's turn in Fleet's memory |
| `owner_away` | Sleep mode is on | The sleep switch |

**Unknown does not hold.** Nothing here calls the forge when a notice is sent, so a pull request the forge has not yet been read for, a merge the forge is still computing and a pull request behind its base (nothing Fleet keeps says so) do not block a merge.
With no condition holding, the rule's own `does` stands. A condition tested twice in one rule is a problem, since the second could never be reached.

Fleet reads the rules where it sends every notice (`told_by_fleet`), after the person's mute: a mute beats any rule, with or without conditions, and a notice with no rule is told.

| `does` | The Session | Needs you | A caller that marks the notice done |
|---|---|---|---|
| `tell` | Told, as with no rule | | After the send |
| `quiet` | Nothing is sent or written to the thread | | At once: it is handled |
| `batch` | Notices of the word held for the Job and sent as one message, joined by a blank line, when `timeouts.noticeBatchSeconds` after the first ends | | At once: it is held. A Job that cannot be told when the window ends loses them, and a restart inside a window does the same |
| `call` | Told, as with no rule | An item on the Job with the words it was told, until the person dismisses it | After the send |

A mute is not handled: the notice is asked again at the caller's next reading, so lifting the mute resumes it.
A notice that is a line to the Session and not a word of the table above (the answer to a request, a plan's change, a linked Job's news) carries no word and no rule reads it.

## Tracker mods

A tracker mod teaches Fleet a tracker this build has no adapter for, over the tracker's command line, its HTTP API or an MCP server.
A repository's Manifest then names it as `provider`, as it names `jira`. [Tracker](tracker.md), *Choosing a tracker*.

> **Rule.** A tracker mod is used only while it has passed a try of its `tracker.toml` as the file now stands.
> Why: the file passing its checks says nothing about whether the command or the address it names answers, and an edit can break either.

### tracker.toml

At most 32 KiB of TOML, read by `crates/adapters/src/taught_tracker/file.rs`.

| Section | Required | Holds |
|---|---|---|
| `[guide]` | yes | `language`, `guide`, `open`, `assigned_to_me` and an optional `fields` list: what `query_guide` answers |
| `[credential]` | no | `argv`, a command whose output is the secret, run for each call and never kept |
| `[search]` | yes | A call: `{query}` is the query, in the tracker's own language |
| `[list]` | no | A call listing the open issues. Absent is a search for the guide's `open` |
| `[read]` | yes | A call: `{reference}` is the issue's reference. Its `map` yields one issue or `null` |
| `[links]` | no | `reference`, a regular expression for how a ticket's reference is written in the words of a dispatch or a message. Text that matches links to that ticket |

A call names its `transport` and takes that transport's keys and no others.

| `transport` | Keys |
|---|---|
| `cli` | `argv`: the program and its arguments, run with no shell |
| `http` | `url`, and optional `method` (`GET` or `POST`), `headers` and `body` (a `POST`'s only) |
| `mcp` | `server`: the program and its arguments, started with no shell and speaking MCP on standard input and output. `tool`: the tool to call. Optional `env`, a table of environment variables for the server, and `arguments`, a table sent as the tool's JSON arguments |

An `mcp` call starts its server for the one call: it sends `initialize` and `notifications/initialized`, calls the tool with `tools/call`, closes the server's input and stops it.
The `map` reads the result's `structuredContent` where it has one, and otherwise the first text part of its `content` parsed as JSON.
A result that has neither fails the call and says so, and so does a text part that is not JSON.
A result with `isError` fails the call with its text, and a JSON-RPC error fails it with the server's message.
A server's own requests, such as `ping`, are not answered.

Every call has a `map`, a `jq` filter turning the answer into a list of issues under Armada's own names.
`reference`, `title`, `url` and `closed` (true or false) are required, and a row missing one fails that call naming the row and the key.
`author`, `assignees`, `labels`, `updated_at`, `excerpt` or `body`, `opened_at`, `fields` (`[{name, values}]`), `comments` (`[{author, at, body}]`) and `linked_pull_requests` (`[{number, title, url, state}]`) are read where they are there.

`{query}`, `{reference}` and `{credential}` are filled into one value each and never through a shell: as written in an argument or a header, percent-encoded in a URL, and escaped inside a JSON string in a body or in an `mcp` call's `arguments`.
In an `mcp` call they are filled into `env` and `arguments` only, since a token in `server` would show in the process list, so `server` and `tool` are written as they are.
Any other `{word}` is text, so a body's own braces stand.
A URL is `https://`, or `http://` to this machine, and names its host as written, so a credential goes only to the host the file names.

| Refused, with its reason | Example |
|---|---|
| A section or a key a tracker does not read | `[comments]`, `shell = true` |
| A transport this build does not have | `transport = "grpc"` |
| A placeholder the section cannot fill | `{query}` in `[read]`, `{credential}` with no `[credential]` |
| A search that never uses `{query}`, a read that never uses `{reference}` | Every call would ask the same |
| A call with no `map` | |
| A `[links] reference` that is not a pattern, or matches the empty text | `LIN-(\d+`, `x*` |
| An `mcp` call with no `server` or no `tool`, or `arguments` holding a date | |

`[links] reference` is a regular expression in Rust's syntax, which has no backtracking.
Each match links the reference it holds, once and in the order written: the first group where the pattern has one, so `linear\.app/acme/issue/(LIN-\d+)` links `LIN-42` from an address, and the whole match where it has none, so `\bLIN-\d+\b` links `LIN-42` from the words.
What it yields is the reference as the tracker writes it and `read` takes it, `7` or `LIN-42`; a match that is not one of those is dropped.
A file with no `[links]` links nothing, and a request that names a ticket is dispatched as written.

Every program and request is bounded: 30 seconds, and 4 MiB of answer.
An `mcp` exchange is bounded as a whole by the same, and a server that says more or is slower is stopped and the call refused.
The credential is held as a `Secret` for the one call, and every refusal is scrubbed of it before it is said.

### A tracker that only offers an MCP server

The server is started for each call with the token in its environment, which is how most MCP servers take one.
The tool here answers with `structuredContent`, or with JSON in a text part, and the `map` is the same either way.
To see what a server answers, start it by hand and call the tool once, then keep that result to test the `map` against.

```toml
[guide]
language = "Tracker filter"
guide = "A filter is `field:value` pairs joined by spaces. Fields: state, assignee, label."
open = "state:open"
assigned_to_me = "state:open assignee:me"

[credential]
argv = ["security", "find-generic-password", "-s", "tracker-token", "-w"]

[search]
transport = "mcp"
server = ["npx", "-y", "@example/tracker-mcp"]
env = { TRACKER_TOKEN = "{credential}" }
tool = "search_issues"
arguments = { filter = "{query}", limit = 100 }
map = """
.issues | map({reference: .key, title, url: .link, closed: (.state == "closed"),
  updated_at: .updated, body: .description})
"""

[read]
transport = "mcp"
server = ["npx", "-y", "@example/tracker-mcp"]
env = { TRACKER_TOKEN = "{credential}" }
tool = "get_issue"
arguments = { key = "{reference}" }
map = """
{reference: .key, title, url: .link, closed: (.state == "closed"), body: .description}
"""
```

### The try

`try_tracker_mod` runs the search for the guide's `open` against a repository Fleet serves, then `[list]` where there is one, then reads the first issue listed by its reference.
It keeps the outcome in `.tried.toml` beside the mod, with when, the repository, how many issues were listed, the reference read or the reason it failed, and the digest of the `tracker.toml` it tried.
`list_mods` carries it as the row's `tried` while that digest is the file's, so an edit shows the mod untried again.
Where a record is kept and its digest is not the file's, the row says `stale` instead, so Bridge can tell an edited mod from one never tried.
Bridge's Try on a tracker row is this operation, sent without a repository, so Fleet tries it against the first served repository whose Manifest names the mod as its tracker and the first served only where none does, and the row is redrawn from the answer.
A scan and a Manifest's lookup read the record and run nothing.
A tracker mod switched off in Mods is refused too, with that as the reason; the lookup reads the switches as Fleet last scanned them, every two seconds and at once on `set_mod_enabled`.

## The operations

| Operation | Kind | A job may | Does |
|---|---|---|---|
| `list_mods` | query | yes | Every mod: kind, version, description, `taught_on`, `entry` (the Logbook entry that made it), `enabled`, `valid`, `reason`, `changed_at`, and a tracker's `tried` or `stale` |
| `scaffold_mod` | command | yes | Makes the folder, a starter file for its `kind` that changes nothing (a tracker's validates and fails its try), `git init` and one commit |
| `validate_mod` | query | yes | Every problem found, and the checked text when there are none: `css` for a theme or a tweak, `layout` for a layout, `words` for a Words mod and `fleet` for a Fleet mod, each the file as typed data |
| `try_tracker_mod` | command | yes | Runs a tracker mod against a repository and keeps how it went |
| `set_mod_enabled` | command | Helm only | This machine's switch for one mod |
| `promote_mod` | command | no | Copies the mod onto a new branch of a repository |

`mods.changed` carries the whole list.
Fleet rescans every two seconds and publishes it when the list differs from the last, and `scaffold_mod` and `set_mod_enabled` publish at once, so a file a job wrote reaches an open Bridge without a restart.

> **Rule.** Bridge injects the `css` that `validate_mod` returned and no other text.
> Why: what loads is then what passed, with no window between the check and a second read.

## What is kept

| What | Where | Default |
|---|---|---|
| A mod's switch | `mod_switches`, a row per mod, with the other preferences | On |
| The active theme | `theme` in `get_preferences`, saved by `save_preferences` with `text`: a built-in's id, `catalogue:<slug>` or a mod's name | `dark` |
| The owner's own layout | `layout_choices` in `get_preferences`, saved with `text` holding a `layout.json`, which outranks every layout mod | None |

A mod nobody switched is on. A switch only says whether the owner allows the mod, and `valid` says whether Bridge may act on it: a mod can be on and invalid.

> **Rule.** A mod becomes the theme only when the owner picks it, and a layout mod applies only while its switch is on.
> Why: choosing is the act that changes his screen, and a job can write a mod but not save a preference, which is Helm's on his ask and Bridge's.

## Promotion

> **Rule.** Promotion puts the mod on a local branch of the repository and pushes nothing.
> Why: landing it is the pull request's, as for any other change.

1. `promote_mod` refuses a mod that is invalid, and a repository with no `packages/` directory.
2. Fleet leases a slot from the repository's pool, with a branch `armada/mod-<name>-<milliseconds>` cut fresh from the base.
3. It writes the two checked files, `mod.toml` and `theme.css`, `layout.json` or `tracker.toml`, to `packages/mods/<name>/` in the slot, never the mod's `.git` or its `.tried.toml`.
4. It commits exactly those paths and gives the slot back.
5. It answers with the branch and the commit.

The owner's checkout is never written, and a branch that exists is never moved. Every slot being in use is a refusal to try again.

## Safe mode

> **Rule.** A mod that cannot be read, or that fails its checks, is never given to Bridge to apply.
> Why: Bridge applies only what `validate_mod` returned, and that is nothing for an invalid mod.

Fleet's half is above: a bad mod is a row with its reason, and `set_mod_enabled` and `save_preferences` are Fleet calls, so switching a mod off or saving `dark` works while Bridge cannot draw the mod.

`bridge --no-mods` is Bridge's half.
Main loads the window with `?nomods`, and the window then lists no mod in Settings or on the Mods surface and never calls `validate_mod` or `list_word_rules`, so no mod's CSS reaches the page, no taught words are linked and the layout is the shipped one.
Dark, Light and the catalogue still choose.
Safe mode does not touch the saved theme: a mod chosen before stays chosen for the next ordinary start, and a built-in or catalogue theme picked in safe mode is saved like any other.

## What Bridge does with a list

| The list says | The window shows | The preference |
|---|---|---|
| The chosen mod is on and valid | Its theme, from `validate_mod`'s `css` | Kept |
| The chosen mod is on and invalid, or its check fails | Dark | Kept, so the mod returns when it passes |
| The chosen mod is switched off, or gone | Dark | Saved as `dark` |
| A new mod, on by default | Nothing changes | Kept |
| No list has been read yet | Dark for a chosen mod | Kept, since "not read" is not "gone" |

A layout mod in the list applies while it is on and valid and draws the shipped layout otherwise, with the owner's own choices over it; the table above is a theme's. [Layout mods](layout-mods.md), *Precedence*.

Bridge saves the theme with `save_preferences` as `theme` with `text`.
A catalogue theme is saved as `catalogue:<slug>`, the id Bridge gives it, and a mod's name has no `:`, so the two cannot meet.
Promote shows the branch `promote_mod` named and nothing more, since nothing is pushed and no pull request is opened.
