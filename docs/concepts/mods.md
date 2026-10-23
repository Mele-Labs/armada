# Mod

**What it is:** A folder on this machine that changes how Bridge looks or how it is arranged, or teaches Fleet a tracker it was not built for, written by a person or by a job, which Fleet reads and checks and which can never stop Fleet from starting or from listing.

---

**Kind:** Record.

You ask a job for a warmer dark theme.
It makes a mod called `warm`, edits its colours and checks them; you pick `warm` in Settings, Theme, and Bridge redraws.
A month later you ask for it in the repository, and it is on a branch under `packages/mods/<name>/`, ready to open as a pull request.

## What it is

> **Rule.** A mod is a folder under the machine's `mods/` directory, and nothing else is one.
> Why: a job that can write a file can make one, without a build and without touching Bridge's source.

> **Rule.** Fleet stores and checks a mod, and Bridge is what applies a theme or a layout.
> Why: Fleet never draws. A tracker mod is the one kind Fleet runs itself, because Fleet is what reads a tracker.

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
| `tracker.toml` | How to list, search and read a tracker's issues, in a tracker mod. *Tracker mods* below |
| `.tried.toml` | A tracker mod's last try, written by Fleet and never promoted |
| `.git/` | The mod's own history, one commit at scaffold |

`name` is a slug of 1 to 40 lowercase letters, digits and `-`, starting with a letter or digit. `dark`, `light`, `system` and `default` are refused, because Bridge's own themes answer to them.

## mod.toml

A flat file of `key = "value"` lines and `#` comments, read by hand: a table, an array or a number is a problem.

| field | type | required | default | meaning |
|---|---|---|---|---|
| `name` | string | yes | | The folder's name |
| `kind` | string | yes | | `theme`, `layout` or `tracker` |
| `version` | string | yes | | Up to 32 letters, digits, `.`, `+`, `-` |
| `description` | string | no | none | Up to 200 characters |

Any other key is a problem.
A `kind` this build does not have makes the mod invalid, so a valid mod's kind is always one Bridge can act on.
A theme mod holds `theme.css`, a layout mod `layout.json` and a tracker mod `tracker.toml`, and the kind says which file is read.

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
| An `mcp` call with no `server` or no `tool`, or `arguments` holding a date | |

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
A scan and a Manifest's lookup read the record and run nothing.
A tracker mod switched off in Mods is refused too, with that as the reason; the lookup reads the switches as Fleet last scanned them, every two seconds and at once on `set_mod_enabled`.

## The operations

| Operation | Kind | A job may | Does |
|---|---|---|---|
| `list_mods` | query | yes | Every mod: kind, version, description, `enabled`, `valid`, `reason`, `changed_at`, and a tracker's `tried` |
| `scaffold_mod` | command | yes | Makes the folder, a starter `theme.css` or, for `kind` `layout`, a `layout.json` that changes nothing, or for `kind` `tracker` a `tracker.toml` that validates and fails its try, `git init` and one commit |
| `validate_mod` | query | yes | Every problem found, and the checked text when there are none: `css` for a theme, `layout` for a layout |
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

## In the shared home

A person's own several machines carry their mods through the shared home, [Kit](kit.md), *The shared home*.
A mod is the home's `mods/<name>/` folder, and Kit lists mods as a kind of its own, with the Theme rows carrying the same marks.

> **Rule.** A mod that is not valid on this machine is not added to the home.
> Why: a copy that no machine can apply is a broken file on every other one.

> **Rule.** Add to this machine writes a mod into `mods/` and never over one that differs; only choosing a side replaces it.
> Why: the person's own edit to a mod is theirs, and a machine's copy changing under them would be a theme redrawn by someone else.

Dotted files stay behind: a mod's `.git/` and a tracker mod's `.tried.toml`, so a brought tracker mod is untried until it is tried here.
Mods are off in What syncs until the person turns the kind on, [Account](account.md).
A brought mod is checked as any other, so it is on and valid, or on and invalid with its reason.

## Safe mode

> **Rule.** A mod that cannot be read, or that fails its checks, is never given to Bridge to apply.
> Why: Bridge applies only what `validate_mod` returned, and that is nothing for an invalid mod.

Fleet's half is above: a bad mod is a row with its reason, and `set_mod_enabled` and `save_preferences` are Fleet calls, so switching a mod off or saving `dark` works while Bridge cannot draw the mod.

`bridge --no-mods` is Bridge's half.
Main loads the window with `?nomods`, and the window then lists no mod in Settings or on the Mods surface and never calls `validate_mod`, so no mod's CSS reaches the page and the layout is the shipped one.
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
