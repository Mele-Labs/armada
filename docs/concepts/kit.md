# Kit

**What it is:** The tool set you bring — Skills, MCP servers, sub agents, Agent files, Plugins, Commands, the allowlist, the models list. Holds defaults a Manifest may extend or restrict.

---

**Kind:** Entity.

Defines Kit — the tool set you bring. Companion to `../contracts/configuration.md`.

**Guild is retired from the lexicon.** It was two concepts wearing one name, split into Kit and [Machine](machine.md).

## What it is

The tool set you bring: Skills, MCP servers, sub agents, Agent files, Plugins, Commands, the allowlist, the models list. Settings **plus** onboarding, not an occasional-visit config screen. Kit is where your tooling gets set up in the first place (Init), and where you return to edit it afterward.

**Two tiers.** Kit holds your defaults; a [Manifest](manifest.md) holds its own version; they merge with a strategy and a resolution order. **Kit rows are the only ones with a Manifest tier.**

**Kit sets defaults a Manifest may extend or restrict — not a ceiling a Manifest may only narrow.** Nothing above the Manifest constrains anything; the config direction rule is withdrawn.

## Scope

The machine's AI tool set and defaults, and everything in `../contracts/configuration.md` tagged "Kit only" or "Kit → Manifest."

Resources, timing, interface and notification routing are **not** here — those are [Machine](machine.md). Setting up your *first project* is also out of scope — that's Manifest's Init, chained together in the broader First-Run Onboarding journey (not yet designed).

## Kit's two homes

**Kit has two homes and says which is which, decided with the owner on 18 Sep.**

| Home | Whose | What Armada does with it |
| --- | --- | --- |
| The harness's own — `~/.claude` for the first adapter | The person's. It is what every other tool they use reads | **Reads it**, to show it. Writes only what they edit in Kit, and not yet |
| `~/.armada` | Armada's own | Makes it, reads it, writes it. Workflows, and since 5 Oct 2026 the allowlist (`allowed-commands`); and anything with no counterpart in a harness |

**Neither is authoritative over the other's idea of anything**, which is what makes two homes survivable where the issue that raised this feared they would not be. Nothing in `~/.armada` overrides a skill; nothing in the harness's home names a Workflow. A row Kit draws is labelled with the home it came from, so a person can always tell what they are looking at and where to go to change it.

**One home was the alternative and was refused.** Putting everything in the harness's home would mean Armada writing into a directory it does not own for things that are Armada's alone rather than only for a person's own edits. Putting everything in `~/.armada` would mean copying a setup that already exists — the drift the owner refused when he decided write-back, because two copies drift the moment either is touched and nobody can then say which a Job ran against.

**Workflows are the first thing read from `~/.armada`.** Fleet makes the folder, with `workflows/` inside it, when it starts. A definition in `~/.armada/workflows/` replaces the one Armada carries with the same `workflow_id` in every repository on this machine, and a repository's own `.armada/workflows/` replaces both — [Workflow](workflow.md) holds the rule. #425. **A definition added to the folder while Fleet runs is read without a restart**, whether a person wrote it there or [Helm](helm.md) saved it for them with `save_workflow`; Kit's scope is what Helm writes when you ask for a workflow every repository on this machine should have. Bridge's Workflows row lists a Kit file beside the carried and repository ones, marks it with the Kit case, and writes one the same way, with scope Kit.

**Review tools are the second.** `~/.armada/review-tools/` holds one `.md` file for each review tool or explanation a person defines for every repository on this machine: front matter naming `kind` (`tool` or `explanation`) and `name`, then the prompt. A repository's own `.armada/review-tools/` replaces both Kit's and Armada's of the same id (the name lower cased, words joined by a dash). A file that does not fit is left out and named, and the rest stand. Bridge's Add sheet writes one with scope Kit or the repository; a definition added to the folder while Fleet runs is read by the next press. `docs/concepts/pull-review.md`, *Review tools*.

**A folder a person can see and sync**, apart from the store, the runtime file and [Machine](machine.md)'s settings: Kit travels and the Machine does not. Everything else this page lists arrives in one of the two homes under #41.

## What a person already has, read to be shown

**Armada reads the harness's own home to show what is in it** — `#1491`. Somebody arrives with a way of working already built: skills they wrote, plugins they installed, a global file saying how they want an agent to behave, servers they connected. Kit opened on *Nothing in your Kit yet* with all of it one directory away, and **see** was the verb the build was missing.

**An adapter per harness**, decided with the owner on 18 Sep. Which folder, which file and how one harness spells a skill are `crates/adapters`' to know, and a second harness is a second adapter with nothing above it moving. `get_kit_inventory` is the read and `adapter_traits::HarnessSetup` is the seam.

**Every kind but one is read**: Skills, Plugins, Agent file, Output styles, Sub agents, Commands, the servers a person connected, and, since 5 Oct 2026, the allowlist, read from `~/.armada` rather than from the harness's home ([Kit's allowlist](#kits-allowlist)). The models list answers *not read yet, and why*: it has no stored tier to read. **A kind is never drawn as empty when nothing looked**, which is the defect this read exists to end rather than repeat one row along. An allowlist file that is there and will not read is `not_read` with the reason, never an empty list.

**Reading is not granting, and no part of it is a control.** What is read carries a name, the thing's own words for itself and where it came from. A server a person connected carries **the program's own file name, or the host it is at** — the owner's decision, 18 Sep: enough to tell two servers apart and to spot one pointing somewhere wrong. What follows either is where a key sits and is never carried: no argument list, no query string, no userinfo, no environment. So what is on the screen could not start the server it names, and a server connected outside Armada is visible here while reaching no unattended agent. That is `../scope.md`'s one confinement, kept where an import would have broken it.

**The adapter cannot hold a credential rather than being careful not to draw one.** A program is kept as its file name and a URL as its host, by a reader that allocates only that much; the argument list and the environment are read into a type that keeps nothing at all. A `Vec<String>` of arguments a screen happened not to draw is the thing this shape exists to avoid.

**A file that will not read is named, with its reason, under the kind it belongs to.** It costs that kind and nothing else, and nothing offers to repair it: the reader has no write to reach for.

**Output styles are read from four places**, #2413. The person's own are the `.md` files in the harness home's `output-styles` folder, a repository's are the same folder under its own `.claude`, and an installed plugin's are the `output-styles` folder under where it is installed. The fourth is the harness's built-in styles, which have no file: one is a row only when the settings name it.

**Which one is active comes from `outputStyle` in the settings files, in the harness's own order: local over project over user.** A repository's project and local files decide for that repository, and the user file decides everywhere else, so a row is `active` when any of those resolutions names it. A row carries its name and description from the front matter (the file name where it has none), where it came from (`person`, `repository`, `plugin`, `built_in` or `armada`, and a plugin's own name beside `plugin`), and whether it is `active`, `available` or `always`. **The body is never carried**, and nothing else in a settings file is read. A style or settings file that will not read is named with its reason, as the Agent file is.

**Armada's job rules are a row of their own**, named *Armada job rules*, from `armada` and applied `always`. They sit on top of whichever style is active in the jobs Bridge hosts, so Kit labels them as Armada's and never as a style the person chose. The row carries a name and one line, and none of the rules.

**Bridge draws each style with its name, its description and where it came from in words**: *Yours*, *This repository*, the plugin's name, *Built in*, *Armada*. The active one carries a check with the tooltip *Active* and a heavier name. The job rules stand under the styles, set apart under *Always applied*, with no mark, and no count is drawn beside the tile.

**Write-back is decided and not built.** Kit writes to the harness's home rather than keeping a copy — one setup, Armada a window onto it — because two copies drift the moment either is touched and nobody can then say which a Job ran against. The cost is that Armada would be writing into a directory it does not own, holding work a person depends on outside Armada entirely, so the read landed first and carries no write at all. #41.

**A Kit definition that does not fit a repository is left out there, and named.** One that will not parse, names a Check that repository does not declare, or shares its id with another Kit file, is set aside — two sharing an id are both left out, and named together. Fleet starts anyway and says at start which definition was left out, why, and whose runs instead: *Kit's `bug` was left out, because …; Armada's `bug` is used instead.* One bad Kit file never stops Fleet for every repository. The owner's decision. One the repository replaces is never resolved against it. A repository's own definitions are held to the same rule, at start and on every re-read: one that does not fit is left out with its reason and the others stand, and it does not refuse Fleet.

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

**Kit holds the default allowlist, and a [Manifest](manifest.md) can extend or restrict it per project.** The same inheritance pattern governs Skills, MCP and Plugins. **Extend or restrict, in either direction, is the model wording for every two-tier setting.**

**A scout gets no MCP server.** [Scout](scout.md) starts with none, and the sources it reads are fetched by Fleet rather than opened by the agent, so nothing here widens a scout.

**Helm's set is still the person's own, resolved by the CLI** — `#1373`, and Kit replaces none of it yet. Helm launches without `--strict-mcp-config`.

### Kit's allowlist

**Built 5 Oct 2026, the owner's decision: Accept on a Kit item in Retros really changes the Kit.** It began when an agent asked for `grep -a -c "arc-dispatch"` on a check log and was stopped, and the owner chose a one-off allowance. That allowance died with the work, the retro listed *allow grep on check logs* as a Kit item, and Accept only set it `accepted`, so the next agent was stopped on the same command.

| | |
| --- | --- |
| Where | `~/.armada/allowed-commands`, a plain file. One command per line, `#` starts a note. A line Armada wrote ends in a tab and where it came from (`always allow`, or `retro <item id>`); a line a person wrote needs neither |
| Read | On every permission question, so a line added by hand while Fleet runs is in force and one taken out is not |
| Tier | **The machine's, which is the Kit tier.** A Manifest extending or restricting it is not built: a repository's own Always allow (`#836`) is its tier, and the two are both consulted |
| Matched by | `permitting::covers`, the rule a repository's Always allow already uses: the command itself, or it with more plain arguments after it, never one that chains another command. No glob and no regular expression |
| Never lifts | A command a Manifest declares destructive, one it runs a Check for, one the harness cannot grant. Kit is read after all three, and widens only the two answers asking would have decided: refuse, and ask |
| Not kept | A command that chains another, or spans lines. A standing grant on this machine for `a && b` is wider than the person who read `a` meant |

**A command Kit does not list is refused as before and the refusal is recorded as before.** Kit is read in the permission question itself, not written into the agent's toolbelt, so every call still reaches Fleet and the *denial record* the finding below flags is not bypassed.

**How a line gets in, and out.** A person writes a line. An **Always allow** answered with `scope: kit` appends the command (or the rule they picked off its candidates) with source `always allow`. **Accept** on a Kit retro item that carries a `change` appends it with source `retro <item id>` and answers with what it applied. `remove_kit_allowed_command` takes a line out, and is what makes the other two reversible; it does not reopen the item that put it there. Adding a command already listed adds nothing, so agreeing twice applies once.

**A retro item's `change` is copied off the record, never written by the model.** The writer names a refusal by its `cite`; Fleet takes the command from that refusal's own row, and drops the change (the item stays) where the row is not a shell call, holds no argument, holds one cut for length, or chains. The writer may name a shorter start of the command, and only one of the command's own leading cuts. An Accept for a command that is destructive in any served repository, or that the harness cannot grant, is refused (`fleet.kit_change_refused`) and the item stays open, since no listing could make it run.

**Only `allow_command` exists as a change.** The models list is resolved from the harness at start (`model_choices`) and has no stored tier, so a `set_model` change would have nothing to write to; it waits for the models list to have a home.

**Reading is not granting.** `get_kit_inventory` carries the command text and where it came from, and nothing that could widen it.

**Where Bridge draws it.** The Kit page's allowlist row lists each command read, in monospace, with where it came from in words (`Retro item`, `Always allow`, `Written by hand`) and a `Remove` that calls `remove_kit_allowed_command` and takes the row off. It draws no count and nothing under its name where the file holds nothing; one that will not read keeps *Not read yet* and the reason. Retros draws a Kit item with a `change` as `Update Kit` then `Updated Kit`: `retro.md`, *Where Bridge draws it*.

**Known cost: allowlist rot.** Two-tier inheritance keeps changes scoped, but upkeep is ongoing as new tools are needed. No automated solution exists; worth monitoring rather than solving now.

**Findings the withdrawn direction rule leaves open**, filed rather than replaced:

- Nothing prevents a Manifest selecting a model outside the Kit set.
- Nothing prevents a Manifest removing a required destructive-op confirmation.
- A Command that bypasses the allowlist bypasses the denial record, which was the part that was working.

**Quota is not here.** The quota floor is [Machine](machine.md).

## Init

Guided, sequential walkthrough of the groups above, with sensible defaults pre-filled — you adjust as you go rather than starting from a blank form. The tool half only; resources and interface are [Machine](machine.md) Init, and first-project setup is Manifest's (see Scope).

What this surface is called, and whether it is its own journey, is tracked in Open questions below.

## Actions

Kit is the System strip's first page, whose places [Bridge](bridge.md) owns, and where this surface finally lives is the `kit-setup-surface-naming` question below.

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

**The git-repo mechanism is a private repository on the person's own GitHub account, `<gh login>/armada-home`, and every machine signed in to that account reads and writes it.** Signing in, the identity and the profiles a machine joins are [Account](account.md); Armada holds no account of its own. It is a third place beside [Kit's two homes](#kits-two-homes), not a replacement for either: each machine keeps its own `~/.claude` and `~/.armada`, and the repository is what they are compared against.

| Carried | Not carried |
| --- | --- |
| Kit definitions, Workflows, the allowlist | A credential of any kind: a key, a token, an environment value |
| Memory for a repository, matched by the repository's name | A server's arguments, its query string, its userinfo |
| Global instructions | |
| An MCP server as its name and host | |

**A row that differs is never merged silently.** Where a kind's row is on both sides and the two disagree, Kit shows both versions side by side and the person picks one. A row on one side only is added to the other by the person's act, in either direction. A server whose key is not on this machine is marked as needing one; the home does not carry the key, so adding it here is the person's.

**Where the two new kinds sit.** Global instructions is drawn beside Agent file, as the same file held in the home. Memory is drawn after the allowlist. Both, and Workflows, are tiles of the Kit page only against a shared home; with none, the page is as it was.

**Built: the Kit page, on mock data.** Each row carries a mark for how it stands against the home (the same, differs, only on this machine, only in the home, needs a key). A bar names the repository and offers Sync. A differing row opens a compare sheet with both versions and a button for each. A row on one side only offers an icon act, *Add to the home* or *Add to this machine*. Every Kit tile is one column that scrolls inside its tile when the list is long. The state comes from `draft.sharedHome` in the mock Fleet.

**Not built: everything behind it.** Nothing creates, reads or writes the repository, no Fleet route carries a row's state, and Sync, Add and the compare choice change the mock's state alone. The marks use Proposed glyphs, `packages/icons/icons/house.toml` for the home and `packages/icons/icons/laptop.toml` for this machine among them.

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

First-Run Onboarding (Kit Init → Machine Init → first Manifest → Doctor check → first dispatch) is analyzed but not yet designed — tracked as a user journey, not as an open item here.

## Open questions

- **[kit-setup-surface-naming]** What is the Kit setup surface called, and is it its own journey? Whether First-Run Onboarding uses the step names above, and whether Kit Init and Machine Init are one step or two, is open.
