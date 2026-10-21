# Kit

**What it is:** The tool set you bring — Skills, MCP servers, sub agents, Agent files, Plugins, Commands, the allowlist, the models list. Holds defaults a Manifest may extend or restrict.

---

**Kind:** Entity.

Defines Kit — the tool set you bring. Companion to `../contracts/configuration.md`.

**Guild is retired from the lexicon.** It was two concepts wearing one name, split into Kit and [Machine](machine.md).

## What it is

The tool set you bring: Skills, MCP servers, sub agents, Agent files, Plugins, Commands, the allowlist, the models list.
Settings **plus** onboarding, not an occasional-visit config screen.
Kit is where your tooling gets set up in the first place (Init), and where you return to edit it afterward.

**Two tiers.** Kit holds your defaults; a [Manifest](manifest.md) holds its own version; they merge with a strategy and a resolution order. **Kit rows are the only ones with a Manifest tier.**

**Kit sets defaults a Manifest may extend or restrict — not a ceiling a Manifest may only narrow.** Nothing above the Manifest constrains anything; the config direction rule is withdrawn.

## Scope

The machine's AI tool set and defaults, and everything in `../contracts/configuration.md` tagged "Kit only" or "Kit → Manifest."

Resources, timing, interface and notification routing are **not** here — those are [Machine](machine.md).
Setting up your *first project* is also out of scope: a Job starts on any folder, and [First-run onboarding](../journeys/first-run-onboarding.md) is the journey that opens one.

## Kit's two homes

**Kit has two homes and says which is which, decided with the owner on 18 Sep.**

| Home | Whose | What Armada does with it |
| --- | --- | --- |
| The harness's own — `~/.claude` for the first adapter | The person's. It is what every other tool they use reads | **Reads it**, to show it. Writes only what they edit in Kit, and not yet |
| `~/.armada` | Armada's own | Makes it, reads it, writes it. Workflows, and since 5 Oct 2026 the allowlist (`allowed-commands`); and anything with no counterpart in a harness |

**Neither is authoritative over the other's idea of anything**, which is what makes two homes survivable where the issue that raised this feared they would not be.
Nothing in `~/.armada` overrides a skill; nothing in the harness's home names a Workflow.
A row Kit draws is labelled with the home it came from, so a person can always tell what they are looking at and where to go to change it.

**One home was the alternative and was refused.** Putting everything in the harness's home would mean Armada writing into a directory it does not own for things that are Armada's alone rather than only for a person's own edits.
Putting everything in `~/.armada` would mean copying a setup that already exists — the drift the owner refused when he decided write-back, because two copies drift the moment either is touched and nobody can then say which a Job ran against.

**Workflows are the first thing read from `~/.armada`.** Fleet makes the folder, with `workflows/` inside it, when it starts.
A definition in `~/.armada/workflows/` replaces the one Armada carries and a repository's own `.armada/workflows/` one with the same `workflow_id`, in every repository on this machine, and says so where it hides a repository's that differs.
[Workflow](workflow.md) holds the rule. #425.
**A definition added to the folder while Fleet runs is read without a restart**, whether a person wrote it there or [Helm](helm.md) saved it for them with `save_workflow`; Kit's scope is what Helm writes when you ask for a workflow every repository on this machine should have.
Bridge's Workflows row lists a Kit file beside the carried and repository ones, marks it with the Kit case, and writes one the same way, with scope Kit.

**Review tools are the second.** `~/.armada/review-tools/` holds one `.md` file for each review tool or explanation a person defines for every repository on this machine: front matter naming `kind` (`tool` or `explanation`) and `name`, then the prompt.
Kit's replaces a repository's own `.armada/review-tools/` and Armada's of the same id (the name lower cased, words joined by a dash), and says so where it hides a repository's that differs.
A file that does not fit is left out and named, and the rest stand.
Bridge's Add sheet writes one with scope Kit or the repository; a definition added to the folder while Fleet runs is read by the next press.
`docs/concepts/pull-review.md`, *Review tools*.

### Flight controls

**Flight controls are the third.** `~/.armada/flight-controls.toml` holds the Cockpit's tiles as a person arranged them, since 18 Oct 2026: their order, which are hidden, what a built-in sends, and actions of their own.
It has a `version` and two lists, `[[job]]` for the tiles while a Job is open and `[[deck]]` for those with none.
A built-in tile a list does not name is appended by Bridge in Armada's order, so a file that says nothing is Armada's own.
A tile's `kind` marks an action of the person's own: `words` or `command` sent to the Job, `shell` run in the Job's bay, or `page` opened.

**A missing file is the defaults; a file that will not read is not the defaults.**
`get_flight_controls` answers `not_read` with why for a TOML error, a key Fleet does not know, a version other than 1, or a broken rule.
A person shown their tiles reset would save over the file that held them.
`save_flight_controls` takes the whole file, refuses a repeated `id` or `key` in one list or a key that is not a digit or a letter, and names the field.
It writes `flight-controls.toml.writing` and renames it, one writer at a time, and comments typed into the file by hand are not kept.

**Fleet does not know the built-in ids.**
They are Bridge's, and a copy in Fleet would drift from them.
Checking that a `kind` sits only on an id that is not built in, and that `on_text` and `off_text` sit only on the land tile, is Bridge's.

**A person can keep their own actions in one repository only.**
They are the person's alone, and reach collaborators only as a copy the person commits ([Commit to repository](#commit-to-repository)) or a package they send ([Share](#share)).
`get_repository_flight_controls` and `save_repository_flight_controls` read and write them where that repository's Armada configuration is kept ([What a repository runs](#what-a-repository-runs)):

| Place | Kept in |
|---|---|
| On this Mac | `~/.armada/flight-controls/<owner--name>.toml` |
| Synced | the same layout under the synced folder |
| Cloud, a bucket | the object `<prefix>/flight-controls/<owner--name>.toml` |
| In the repository, or Armada's store | on this Mac, as above |
| Any place, while the repository is [kept on this Mac](#kept-on-this-mac) | on this Mac, as above |

**The last row holds because a save never writes the repository, and Armada's store keeps only runs.**
Every tile in such a file carries a `kind`; built-in tiles stay machine-wide.
Bridge draws them after the person's own tiles and before Pause, and Edit's action sheet offers *Only in* the repository's name.
`get_repository_flight_controls` also answers `in_repository`: the repository's own `.armada/flight-controls.toml` as committed, read from the tree of the Job the read names, or from the checkout with none.

**A shell tile the repository committed asks before it runs.**
A press of a tile from `in_repository` is refused until this Mac has approved its command, and the refusal carries the command so Bridge can show it.
Fleet reads that command from the repository itself, from the Job's tree where the press names a Job, and runs it: the command text a press sends is not used, and a tile id the repository does not hold is refused.
`approve_repository_tile` keeps a hash of the exact command in `~/.armada/approved-tiles/<owner--name>.toml`, by tile id, on this Mac only and never in the store or a synced place.
A tile whose command changes asks again, and an approval of an old command cannot approve the new one, since it names the hash it was shown.
`get_repository_flight_controls` marks each such tile `approved` or not, so the ask shows before the press.
A person's own tiles, and ones imported from a package, never ask.
`docs/design/who-sees-each-piece-of-configuration.md`, *A tile a teammate committed asks before it runs*.

**A shell action runs where the Job's work is.**
`run_session_shell` runs the line with the person's shell in the worktree slot the Job holds on the ledger, and refuses a Job that holds none, so it never reaches the main checkout.
It answers when the line has ended, with the exit status and the last 64 KiB of output, standard output and error in the order they arrived.
A line still running after 120 seconds is stopped.
The press is the person's own, so no allowlist prompt asks about it, and the Job is told what ran as it is for a block run from its thread.
`docs/practices/protocol.md`, *Flight controls*.

**A folder a person can see and sync**, apart from the store, the runtime file and [Machine](machine.md)'s settings: Kit travels and the Machine does not.
Everything else this page lists arrives in one of the two homes under #41.

### Commit to repository

**Commit to repository copies one of a person's own pieces into the repository by a Job of its own**: its own slot and branch, one commit adding the piece, and a pull request for that alone.
It works with no Job open, and the piece never rides in another Job's pull request.
The owner decided on 19 Oct 2026 that committing is an option on a piece, and never where a piece is kept: `docs/design/who-sees-each-piece-of-configuration.md`, *Commit to the repository is an option on a piece*.
He decided on 21 Oct 2026 that it starts a Job of its own: `.claude/decisions/2026-10-21-commit-starts-a-job-of-its-own.md`.

| Piece | Copied from | Written to |
|---|---|---|
| Tiles, chosen by id | the person's own for the repository, above | `.armada/flight-controls.toml`, beside the tiles main already holds |
| A workflow | Kit's `~/.armada/workflows/` | `.armada/workflows/`, under the Kit file's own name |
| A review tool | Kit's `~/.armada/review-tools/` | `.armada/review-tools/<id>.md` |
| The person's `armada.yml` for the repository | Kit's `manifests/<owner--name>.yml`, at the place the tiles are kept | `armada.yml` at the workspace root |

**Committing copies.** The person's own piece stays where it is kept, and nothing of theirs is moved or deleted.
**Never the checkout.** The checkout is read and never written.
**Refused before any Job is made.** Every refusal is decided against main's tree in the checkout, so a refused commit leaves no Job behind.
An id main already holds is refused unless the person chose to replace it.
An `armada.yml` main already holds is refused the same way, and a Kit copy that is not a whole Manifest is refused.
A piece main already holds exactly as it would be written is refused, since its pull request would carry nothing.
**The Job.** Fleet starts a Job titled for the piece, such as *Commit the tile Run tests*, and leases it a slot and a branch as any Job's first write does.
No agent runs in it.
The files are written in its tree and committed alone on its branch, with a message saying what was added.
The branch is pushed and a pull request opened into the base through the forge Fleet delivers every Job through.
From there the pull request is the Job's, and is watched, merged and reported as every Job's is.
**A pull request that does not open loses nothing.** The Job keeps its branch with the commit on it, and the answer says why no pull request opened.
`commit_to_repository`; `docs/practices/protocol.md`, *Commit to repository*.

### A repository's armada.yml

**A person can keep a repository's `armada.yml` in Kit**, as `~/.armada/manifests/<owner--name>.yml`.
It is the Manifest where the repository has none, and replaces the repository's Checks and Commands by name where it has one.
**It is kept in the place the repository's tiles are kept in**, and stays on this Mac while the repository is marked Keep on this Mac.
A place that cannot be read reads as no copy, and the Manifest's summary says so.
[Manifest](manifest.md), *Kit's copy*, holds the rule.

### Share

**Share makes one file, `<name>.armada.toml`, from pieces a person picks, and Copy puts the same text on the clipboard.**
It is built at the Fleet end: `share_package` makes the text, and Bridge writes the file and the clipboard.
The owner decided on 19 Oct 2026 that sharing is a package a person sends, and never a place Armada keeps: `docs/design/who-sees-each-piece-of-configuration.md`, *Share is a package*.

The package is a `version` key and one table per piece.
Each holds the piece's kind, the piece exactly as stored, and the remote of the repository it is kept for.
It names no path, user name, machine, Kit place or time.

**A colleague drops the file, or pastes the text, and reads before anything is written.**
`read_package` lists every piece and every shell command it would run, the repository each would land on, and whether their Kit already holds the id.
**Add is the one ask**, and `add_package` writes the pieces where the colleague's own pieces live: a repository's tiles beside their own, `~/.armada/workflows/` and `~/.armada/review-tools/`.
What is added becomes theirs and runs like their own.
An id already present is refused unless the colleague chose to replace it, and a tile the package does not hold is never touched.
A workflow or review tool is kept once for every repository, so a package holding one id of either for two repositories is refused when read.
Everything that would be refused is found before the first piece is written; only a file that cannot be written leaves the pieces written before it written.

**A person's own `armada.yml` for a repository is a piece too**, and Share carries it as it is written, with the repository's remote.
Reading it lists every Check and Command line it would add, and adding it writes it to the place the colleague's Kit keeps their own `armada.yml` for that repository ([Manifest](manifest.md), *Kit's copy*).
One the colleague already has is refused unless they chose to replace it, and a place that will not answer refuses the add before anything is written.

**A piece lands on the one repository whose remote is its own.**
A remote that names none of the repositories Fleet serves waits unbound on this Mac until the colleague chooses a repository once, as [Account](account.md) keeps a repository no profile owns.
`share_package`, `read_package`, `add_package`, `list_unbound_pieces` and `bind_unbound_piece`; `docs/practices/protocol.md`, *Share*.
Bridge does not draw Share yet.

## What a person already has, read to be shown

**Armada reads the harness's own home to show what is in it** — `#1491`.
Somebody arrives with a way of working already built: skills they wrote, plugins they installed, a global file saying how they want an agent to behave, servers they connected.
Kit opened on *Nothing in your Kit yet* with all of it one directory away, and **see** was the verb the build was missing.

**An adapter per harness**, decided with the owner on 18 Sep.
Which folder, which file and how one harness spells a skill are `crates/adapters`' to know, and a second harness is a second adapter with nothing above it moving.
`get_kit_inventory` is the read and `adapter_traits::HarnessSetup` is the seam.

**Every kind but one is read**: Skills, Plugins, Agent file, Output styles, Sub agents, Commands, the servers a person connected, and, since 5 Oct 2026, the allowlist, read from `~/.armada` rather than from the harness's home ([Kit's allowlist](#kits-allowlist)).
The models list is not in this read: it is asked of the agent itself and served by `list_models` ([Kit's models](#kits-models)), so this kind still answers *not read yet, and why*.
**A kind is never drawn as empty when nothing looked**, which is the defect this read exists to end rather than repeat one row along.
An allowlist file that is there and will not read is `not_read` with the reason, never an empty list.
A plugin's skills, sub agents and commands are read from where it is installed, beside the person's own, and each row carries `plugin`, the plugin's name without its marketplace.

**Reading is not granting, and no part of it is a control.** What is read carries a name, the thing's own words for itself and where it came from.
A server a person connected carries **the program's own file name, or the host it is at** — the owner's decision, 18 Sep: enough to tell two servers apart and to spot one pointing somewhere wrong.
What follows either is where a key sits and is never carried: no argument list, no query string, no userinfo, no environment.
So what is on the screen could not start the server it names, and a server connected outside Armada is visible here while reaching no unattended agent.
That is `../scope.md`'s one confinement, kept where an import would have broken it.

**The adapter cannot hold a credential rather than being careful not to draw one.** A program is kept as its file name and a URL as its host, by a reader that allocates only that much; the argument list and the environment are read into a type that keeps nothing at all.
A `Vec<String>` of arguments a screen happened not to draw is the thing this shape exists to avoid.

**A file that will not read is named, with its reason, under the kind it belongs to.** It costs that kind and nothing else, and nothing offers to repair it: the reader has no write to reach for.

**Output styles are read from four places**, #2413. The person's own are the `.md` files in the harness home's `output-styles` folder, a repository's are the same folder under its own `.claude`, and an installed plugin's are the `output-styles` folder under where it is installed.
The fourth is the harness's built-in styles, which have no file: one is a row only when the settings name it.

**Which one is active comes from `outputStyle` in the settings files, in the harness's own order: local over project over user.** A repository's project and local files decide for that repository, and the user file decides everywhere else, so a row is `active` when any of those resolutions names it.
A row carries its name and description from the front matter (the file name where it has none), where it came from (`person`, `repository`, `plugin`, `built_in` or `armada`, and a plugin's own name beside `plugin`), and whether it is `active`, `available` or `always`.
**The body is never carried**, and nothing else in a settings file is read.
A style or settings file that will not read is named with its reason, as the Agent file is.

**Armada's job rules are a row of their own**, named *Armada job rules*, from `armada` and applied `always`.
They sit on top of whichever style is active in the jobs Bridge hosts, so Kit labels them as Armada's and never as a style the person chose.
The row carries a name and one line, and none of the rules.

**Bridge draws each style with its name, its description and where it came from in words**: *Yours*, *This repository*, the plugin's name, *Built in*, *Armada*.
The active one carries a check with the tooltip *Active* and a heavier name.
The job rules stand under the styles, set apart under *Always applied*, with no mark, and no count is drawn beside the list.

**Write-back is decided and not built.** Kit writes to the harness's home rather than keeping a copy — one setup, Armada a window onto it — because two copies drift the moment either is touched and nobody can then say which a Job ran against.
The cost is that Armada would be writing into a directory it does not own, holding work a person depends on outside Armada entirely, so the read landed first and carries no write at all. #41.

**A Kit definition that does not fit a repository is left out there, and named.** One that will not parse, names a Check that repository does not declare, or shares its id with another Kit file, is set aside — two sharing an id are both left out, and named together.
Fleet starts anyway and says at start which definition was left out, why, and whose runs instead: *Kit's `bug` was left out, because …; Armada's `bug` is used instead.* One bad Kit file never stops Fleet for every repository.
The owner's decision.
A repository definition that Kit's replaces is never resolved against it.
A repository's own definitions are held to the same rule, at start and on every re-read: one that does not fit is left out with its reason and the others stand, and it does not refuse Fleet.

## What a repository runs

**Fleet remembers how each repository runs, from what its Jobs did, with no press.** The `Bash` calls of a hosted Job and what came back are read off its stream; each command becomes a fact as its result comes back, `{workspace, kind, command, port?, succeeded, last_ran, source}`, under the rules `get_import_runs` uses (`source` is `jobs` here).
Fleet's working record holds it at once, and the place has it written as each turn ends, so a line can show as not written yet until then.
A command whose every run failed is never kept, and the facts merge with those imported from past sessions for the same repository.
`get_run_memory` reads them with where each is kept.

**The place is chosen per repository when it is added**, `this_mac` until a person says otherwise (`get_run_memory_place`, `set_run_memory_place`):

| Place | Written to |
|---|---|
| `this_mac` | `runs/<owner--name>.toml` in Kit's `~/.armada`, keyed by the repository's remote, or its path where it has none |
| `synced` | the same layout under a folder the person named |
| `repository` | `armada.yml` on the Job's own branch, committed, so it lands with the Job's pull request. **Every line is a Command, never a Check**: this records how the repository runs, and nothing remembered gates a pull request. A name the file already declares is left alone |
| `cloud` | Armada's central store, or a bucket the person owns, chosen with the place: `{"store":{}}` or `{"bucket":"s3://bucket/prefix"}` (`r2://` and `gs://` too) |

**The store form** keeps the facts as rows under a hash of the repository's remote, as the profile the repository is bound to, [Central store](../contracts/central-store.md).
**The bucket form** keeps the Kit layout as one object, `<prefix>/runs/<owner--name>.toml`, signed with the provider's own credentials: `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` or a profile in `~/.aws/credentials` for `s3://`, `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID` and `R2_SECRET_ACCESS_KEY` for `r2://`, and `GOOGLE_APPLICATION_CREDENTIALS` or `gcloud auth application-default login` for `gs://`.
Kit holds none of them.
A key that may not list the bucket reads a missing object as refused.
**A cloud place that cannot be reached or signed in is refused when it is chosen, `422 fleet.run_cloud_unsigned`, `fleet.run_cloud_unreachable` or `fleet.run_cloud_refused`, and the old place stays.** A write that fails later keeps the facts waiting with the reason and tries again when the next turn ends; a fact is never dropped for it.
What another machine wrote is merged in before each write, and `get_run_memory` does not call the cloud.

### Kept on this Mac

**Any repository can be marked Keep on this Mac**, for a private work repository whose name and commands should not land in a synced folder or a bucket.
`get_kept_on_this_mac` reads the mark and `set_kept_on_this_mac` sets or clears it, per repository.

| While it is marked | What holds |
|---|---|
| Its run memory | The place is `this_mac`; marking moves it there and writes what it remembers |
| Another place | `set_run_memory_place` refuses it, `422 fleet.run_place_kept_here`, and says why |
| Its tiles | Read from and written to `~/.armada/flight-controls/<owner--name>.toml` |
| Kit's copy of its `armada.yml` | Read from and written to `~/.armada/manifests/<owner--name>.yml` ([Manifest](manifest.md), *Kit's copy*) |

**Marking leaves any copy already in a synced folder or a bucket where it was.**

**Clearing the mark moves nothing.** The place stays `this_mac` until a person chooses another.
A repository kept on this Mac does not follow the person to another Mac.
Bridge does not draw the mark yet.

**A dropped line is never kept again.** `drop_run_fact` puts it on a deny list that the merge reads first; Fleet's later runs and imports do not bring it back.

### What a person is shown

**Bridge names a repository's remembered runs, with its Manifest, Armada configuration.** It is how Armada remembers the repository runs, so an agent does not guess a test command or a port.

**A Job is told what was kept, in its system prompt.** Every process of a hosted Job or Helm Session in the repository carries the remembered runs as a note after the job rules and after Helm's or a successor's note, read from Fleet's store at each start.
So a run one Job kept reaches the next Job started there, even one started before the first Job's turn ended.
The note names only the runs whose last run worked, newest first, at most 20, each as kind, workspace, command and port where there is one.
**Every line is a command to run, never a Check, and nothing in the note gates a pull request.** A dropped line is not in it.
A repository marked Keep on this Mac is told from this Mac's store, and the note is written nowhere.
**A Job is also told what this Mac has**, in a third part after the run note: `machine.md`, *What a Job is told about this Mac*.
A remembered run whose first word is a missing command is left out of the run note.
`get_manifest` does not serve remembered runs.
The places are named as a person reads them:

| `place` | Named |
|---|---|
| `this_mac` | On this Mac, the default |
| `repository` | In the repository |
| `synced` | A synced folder, such as iCloud Drive or Google Drive |
| `cloud` | Cloud storage: Armada's store, or a bucket address |

**Cloud storage asks a second line.** Under it Bridge lists the store, as one row naming the profile the repository is bound to, and a field for a bucket address; the pick is sent as `{"store":{}}` or `{"bucket":"…"}`.
A repository bound to no profile has no store row.

| Where | Shows |
|---|---|
| The first Job on a repository, where no other Job there has run | An Armada configuration card at the top of its thread, with one line on what it is for and the four places, On this Mac picked |
| A later Job | The place's glyph in the Job's header, beside the repository chip |
| The repository picker | A glyph on each repository's row; a press opens Armada configuration for that repository, where `x` drops a line |

All three read Fleet's run memory routes (#2605); the mock draws the same surface from its own answers.

## Navigation — two functional groups

The original four groups split across the two concepts: AI Behavior and the tooling half of Safety are Kit; Resources & Budget, Interface & Notifications and Helm action authority are [Machine](machine.md).

| Group | Contents |
| --- | --- |
| AI Behavior | Skills, MCP, Agent files, Output styles, Plugins, Way I work, Expectations, Workflows, Sub agents, Models, AI-assisted prompt |
| Safety — the tooling half | Allowlist defaults, destructive-op list defaults |

- Agent files are the global "how I work" file — Kit-only, no Manifest-level counterpart, see [Manifest](manifest.md).
- Output styles are how the harness is told to answer, and sit beside Agent files in AI Behavior. They are read only: Kit shows them and writes none.
- Sub agents are global; a Manifest can layer project-specific ones on top.
- Helm action authority is not here. It is [Machine](machine.md), and was mis-tagged with a Manifest tier before the split.

## Destructive-op list defaults

**Governs agent-initiated operations only** — what a Job must stop and ask you about mid-run. Your own Bridge actions (stop a Job, force-merge) always confirm and are not configurable.

**Finding, not yet designed, and filed rather than replaced.** Union-only was what guaranteed a Manifest could never remove a confirmation Kit required; with the direction rule withdrawn, nothing does.

## Two-tier inheritance

**Kit holds the default allowlist, and a [Manifest](manifest.md) can extend or restrict it per project.** The same inheritance pattern governs Skills, MCP and Plugins.
**Extend or restrict, in either direction, is the model wording for every two-tier setting.**

**A scout gets no MCP server.** [Scout](scout.md) starts with none, and the sources it reads are fetched by Fleet rather than opened by the agent, so nothing here widens a scout.

**Helm's set is still the person's own, resolved by the CLI** — `#1373`, and Kit replaces none of it yet. Helm launches without `--strict-mcp-config`.

### Kit's allowlist

**Built 5 Oct 2026, the owner's decision: Accept on a Kit item in Retros really changes the Kit.** It began when an agent asked for `grep -a -c "arc-dispatch"` on a check log and was stopped, and the owner chose a one-off allowance.
That allowance died with the work, the retro listed *allow grep on check logs* as a Kit item, and Accept only set it `accepted`, so the next agent was stopped on the same command.

| | |
| --- | --- |
| Where | `~/.armada/allowed-commands`, a plain file. One command per line, `#` starts a note. A line Armada wrote ends in a tab and where it came from (`always allow`, or `retro <item id>`); a line a person wrote needs neither |
| Read | On every permission question, so a line added by hand while Fleet runs is in force and one taken out is not |
| Tier | **The machine's, which is the Kit tier.** A Manifest extending or restricting it is not built: a repository's own Always allow (`#836`) is its tier, and the two are both consulted |
| Matched by | `permitting::covers`, the rule a repository's Always allow already uses: the command itself, or it with more plain arguments after it, never one that chains another command. No glob and no regular expression |
| Never lifts | A command a Manifest declares destructive, one it runs a Check for, one the harness cannot grant. Kit is read after all three, and widens only the two answers asking would have decided: refuse, and ask |
| Not kept | A command that chains another, or spans lines. A standing grant on this machine for `a && b` is wider than the person who read `a` meant |

**A command Kit does not list is refused as before and the refusal is recorded as before.** Kit is read in the permission question itself, not written into the agent's toolbelt, so every call still reaches Fleet and the *denial record* the finding below flags is not bypassed.

**How a line gets in, and out.** A person writes a line.
An **Always allow** answered with `scope: kit` appends the command (or the rule they picked off its candidates) with source `always allow`.
**Accept** on a Kit retro item that carries a `change` appends it with source `retro <item id>` and answers with what it applied.
`remove_kit_allowed_command` takes a line out, and is what makes the other two reversible; it does not reopen the item that put it there.
Adding a command already listed adds nothing, so agreeing twice applies once.

**A retro item's `change` is copied off the record, never written by the model.** The writer names a refusal by its `cite`; Fleet takes the command from that refusal's own row, and drops the change (the item stays) where the row is not a shell call, holds no argument, holds one cut for length, or chains.
The writer may name a shorter start of the command, and only one of the command's own leading cuts.
An Accept for a command that is destructive in any served repository, or that the harness cannot grant, is refused (`fleet.kit_change_refused`) and the item stays open, since no listing could make it run.

**Only `allow_command` exists as a change.** The models list is asked of the agent (`list_models`) and its one stored part is `models.roster` in settings.json, so a `set_model` change would be a save of that key; it is not built.

**Reading is not granting.** `get_kit_inventory` carries the command text and where it came from, and nothing that could widen it.

**Where Bridge draws it.** System › Kit › Allowlist lists each command read, in monospace, with where it came from in words (`Retro item`, `Always allow`, `Written by hand`).
The picked command's detail carries a `Remove` that calls `remove_kit_allowed_command` and takes the row off.
It draws no count and no list where the file holds nothing; one that will not read keeps *Not read* and the reason.
Retros draws a Kit item with a `change` as `Update Kit` then `Updated Kit`: `retro.md`, *Where Bridge draws it*.

### Kit's models

**Kit reads the models from the agent itself**, the owner's decision of 20 Oct 2026.
Fleet asks the agent CLI once it is serving, off the start path, and again when `list_models` is asked fresh.
The agent answers with no prompt and no tokens, and the process is stopped as soon as it has.

One row per model, named without a version: `opus`, not a pinned release.
The version the name points at is shown beside it as a fact, with the agent's own words and the efforts it takes.
A pinned older version is not listed.

| | |
| --- | --- |
| Held | `models.roster` in settings.json: the names switched on. Absent, every model the agent offers is on |
| A switch | A save of `models.roster` through `save_settings`. It is live: the next definition checked and the next Job see it |
| Refused | A save that switches off a model a setting still names, in the same save: it names the setting. A definition naming a model switched off or not offered, as before |
| Not read | Until the answer is in, or where the agent cannot give it, the built-in names are served and `list_models` says it did not read them, with the reason |

**The Models page is Bridge's, and not built here.** It draws `list_models`' `offered` rows and saves a switch as the new `models.roster`.
The inventory's `models` kind above is a different read and stays *not read yet*.

**Known cost: allowlist rot.** Two-tier inheritance keeps changes scoped, but upkeep is ongoing as new tools are needed. No automated solution exists; worth monitoring rather than solving now.

**Findings the withdrawn direction rule leaves open**, filed rather than replaced:

- Nothing prevents a Manifest selecting a model outside the Kit set.
- Nothing prevents a Manifest removing a required destructive-op confirmation.
- A Command that bypasses the allowlist bypasses the denial record, which was the part that was working.

**Quota is not here.** The quota floor is [Machine](machine.md).

## Init

Guided, sequential walkthrough of the groups above, with sensible defaults pre-filled — you adjust as you go rather than starting from a blank form.
The tool half only; resources and interface are [Machine](machine.md) Init, and first-project setup is Manifest's (see Scope).

What this surface is called, and whether it is its own journey, is tracked in Open questions below.

## Actions

Kit is the first group of the System page's sidebar, whose places [Bridge](bridge.md) owns, and where this surface finally lives is the `kit-setup-surface-naming` question below.

| Action | What it does |
| --- | --- |
| Edit / View | Standard settings editing across the functional groups above |
| Push to Claude | Keeps your live local Claude environment in step with your Kit |
| Import / Export | Carries one Kit across your own several machines |
| Upgrade | Schema version auto-increment plus migration scripts |

"Sync" is not a standalone action. The name conflated two distinct ideas, split into Push to Claude and Import / Export.

### Push to Claude

Keeps your live local Claude environment in step with your Kit's Skills, MCP and Agent files.

**Automatic.** When you edit your Kit, Fleet pushes the change out; you never have to trigger it. Non-conflicting items merge silently.

**Not built.** Nothing writes a person's own Claude configuration.

**A same-named conflict with config Claude already has outside Armada surfaces for you to resolve**, rather than being silently overwritten. That is the one time you are involved.

A manual re-push exists as recovery — after resolving a conflict, or repairing a Claude install — not as the normal path.

### Import / Export

Two mechanisms, both carried over from the v1 prototype:

- Git-repo push/pull, for continuous cross-machine portability.
- One-off file export/import, for a simple copy.

**Your Kit travels, the Machine does not.** Import / Export is #1449.

### The shared home

**The git-repo mechanism is a private repository on the person's own GitHub account, `<gh login>/armada-home`, and every machine signed in to that account reads and writes it.** Signing in, the identity and the profiles a machine joins are [Account](account.md); Armada holds no account of its own.
It is a third place beside [Kit's two homes](#kits-two-homes), not a replacement for either: each machine keeps its own `~/.claude` and `~/.armada`, and the repository is what they are compared against.

| Carried | Not carried |
| --- | --- |
| Kit definitions, Workflows, the allowlist | A credential of any kind: a key, a token, an environment value |
| Memory for a repository, matched by the repository's name | A server's arguments, its query string, its userinfo |
| Global instructions | |
| An MCP server as its name and host | |

**A row that differs is never merged silently.** Where a kind's row is on both sides and the two disagree, Kit shows both versions side by side and the person picks one.
A row on one side only is added to the other by the person's act, in either direction.
A server whose key is not on this machine is marked as needing one; the home does not carry the key, so adding it here is the person's.

**Where the two new kinds sit.** Global instructions is drawn beside Agent file, as the same file held in the home.
Memory is drawn after the allowlist.
Both, and Workflows, are rows of the Kit lists only against a shared home; with none, the lists are as they were.

**Built: the Kit page, on mock data.** Each row carries a mark for how it stands against the home (the same, differs, only on this machine, only in the home, needs a key).
A bar names the repository and offers Sync.
A differing row opens a compare sheet with both versions and a button for each.
A row on one side only offers an icon act, *Add to the home* or *Add to this machine*.
Every Kit kind is a list with the picked item's detail beside it, and the list scrolls inside its column when it is long.
The state comes from `draft.sharedHome` in the mock Fleet.

**Not built: everything behind it.** Nothing creates, reads or writes the repository, no Fleet route carries a row's state, and Sync, Add and the compare choice change the mock's state alone.
The marks use Proposed glyphs, `packages/icons/icons/house.toml` for the home and `packages/icons/icons/laptop.toml` for this machine among them.

### Upgrade

**Same pattern as Manifest schema versioning:** auto-increment schema version plus migration scripts, applied automatically on Fleet startup.

## Configuration

The settings in `../contracts/configuration.md` that directly affect this concept are its "Kit only" and "Kit → Manifest" rows. This document's config analysis added these settings to that registry:

- Kit-level allowed/default models list
- AI-assisted prompt toggle
- Import/Export Git repo target
- Push to Claude conflict-resolution policy
- Push to Claude target (which local Claude install)

**The last three classify as [Machine](machine.md) and are flagged as low-confidence.** They configure the mechanism by which a Kit travels, which sits awkwardly across the split.

## Still open

First-run onboarding starts a Job on a folder and walks neither Kit Init nor Machine Init first.
Where first run offers them is tracked below.

## Open questions

- **[kit-setup-surface-naming]** What is the Kit setup surface called, and is it its own journey?
  Whether Kit Init and Machine Init are one step or two, and where first run offers them, is open.
