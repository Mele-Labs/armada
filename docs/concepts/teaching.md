# Teaching

**What it is:** The owner selects something in Bridge or a Job, presses Teach and says what he wants, and Fleet reads that into a proposal of up to five parts that he checks and saves: a note for agents, a Words mod, a Fleet mod, a theme tweak and a layout change.

---

**Kind:** Concept.

A Job's reply names `ARM-123`, and it is plain text.
You select it, press Teach and write "this is a Jira key, link it".
Fleet answers with a Logbook note, a pattern for Jira keys and, for this repository, the address its Jira site opens them at, found in the Manifest's tracker settings.
You save, every `ARM-123` in every message links from then on, and the live Job is told.
The mods are files on this machine, so a later Job in another repository finds the same pattern and is given the address that repository has.

## What he presses it on

Teach is on the selection bar, which lists it first, and on a right-click or the ContextMenu key over a row.

| Source (`kind`) | Pressed on | May produce |
|---|---|---|
| `words` | Words selected in a Job's prose | Note, Words |
| `command` | A command row | Note, Words |
| `notice` | A Fleet notice, whose word is `ids[0]` | Note, Fleet |
| `task` | A task in a Job's plan | Note, Words |
| `check` | A Check's result | Note, Words |
| `pull_request` | A pull request | Note, Words |
| `panel`, `row`, `title_bar` | One of Bridge's own panels, rows or the title bar | Tweak, Layout |

A part a source cannot make is dropped, and never refused.
A teaching on Bridge's own interface has no agent in the room, so it has no note, and the panel offers no reach for it.

## The proposal

> **Rule.** `read_teaching` is one model call that keeps nothing, and `save_teaching` keeps what the owner left.
> Why: the owner checks what Fleet understood before anything is written, and a reading he never saves leaves no trace.

The call runs on the retro model, client and budget, and takes what he selected, the message it came from, what he wrote and how far it reaches.
Reach is this repository or every repository, and a reach of one repository needs a repository Fleet serves.

| Part | Is | Kept as |
|---|---|---|
| `note` | What an agent should be told from now on: a section, a kind and one or two lines | A Logbook entry. [Logbook](logbook.md) |
| `words` | A pattern learned once, such as a Jira key, with its name and the selected words as its example | A Words mod: `words.toml` |
| `fleet` | A notice rule: `{notice word, tell, quiet, batch or call, job, repo or everywhere}` | A Fleet mod: `fleet.toml`, one rule |
| `tweak` | Token declarations over whichever theme is chosen, at most 12 | A tweak mod: `tweak.css` |
| `layout` | The same text a layout mod's `layout.json` holds | A layout mod. [Layout mods](layout-mods.md) |

At least one part is present, and an answer with none left is refused.
Fleet fills a tweak's `was` from the token's current value, and its `used_in` and `note` from the tokens file.
The model is given the token names from `packages/tokens/tokens.json` and the regions and ids of `packages/shell/layout-registry.json`, and can propose no others.

### An address for each repository

> **Rule.** A Words mod holds the pattern once and an address per repository.
> Why: `ARM-123` opens at one place in one repository and at another place in the next, and the pattern is the same.

Where the owner gave no address, Fleet looks for one, off the request, and the save does not wait for it.

| Order | Source | Written as |
|---|---|---|
| 1 | The Manifest's tracker names a site, and its adapter builds the address from the example | `found = "settings"` |
| 2 | The text of up to five recently merged pull requests holds an address with a match of the pattern in it | `found = "pull_request #<number>"` |
| 3 | Neither: he is asked once, in Needs you, for that mod and repository | `found = "person"` |

An address a person wrote is never replaced by one that was found.
Teaching a pattern a Words mod already holds adds or replaces that address in it, and makes no second mod.
The mod's layout is in [Mod](mods.md), *words.toml*.

### A rule for a notice

Every notice Fleet sends to a Job carries a kind word, and that word is what a Fleet rule names.
The words are the `Notice` enum in `crates/ipc/src/notices.rs`.

| Group | Words |
|---|---|
| Pull requests | `pr_checks_failed`, `pr_conflicts`, `pr_dropped`, `pr_unmergeable`, `pr_merged`, `pr_auto_merge`, `pr_queued`, `auto_land_refused` |
| Slots and the build | `behind_base`, `slot_free`, `slot_kept`, `slot_stranded`, `build_moved`, `write_clash`, `main_red` |
| Workflow and delivery | `rail_nudge`, `rail_step`, `plan_approved`, `workflow_changed`, `delivery_asked`, `delivery_chosen`, `watch_saw` |
| Usage and handoff | `usage_trip`, `limit_wake`, `context_handoff` |
| Others | `retro_note`, `reply_claim`, `walk_failed` |
| Teaching | `taught` |

> **Rule.** A mute beats a rule, and the nearest place beats a farther one: a Job's rule, then its repository's, then everywhere's.
> Why: a person turning a notice off is the stronger, newer word, and a rule for one Job is more particular than one for all.

Among rules of one place, the later mod name wins.
A notice with no rule is told.
What `tell`, `quiet`, `batch` and `call` do is in [Mod](mods.md), *fleet.toml*.

## Saving

> **Rule.** Everything is checked before anything is written, so a refused part leaves no entry and no mod behind.
> Why: a teaching that is half kept would be a Logbook line with no link, or a link with no line.

1. The note becomes a Logbook entry first, through `teach_logbook_entry`, with the Job and the message, notice or Check as its one evidence row. It stands at once.
2. Each other part is written as a mod, switched on. A name that is taken gets `-2`, `-3`.
3. Each mod's `mod.toml` records `taught_on`, what Teach was pressed on, and `entry`, the Logbook entry's id when the teaching had a note.
4. Bridge is told the mods moved.
5. The Job it was taught in is told "From now on: <note>" with the `taught` notice, in Fleet's voice.
   A Job mid-turn has the line queued behind the turn, an idle one is woken, and one that has ended is not told.
   The save does not fail for want of someone to tell.

> **Rule.** Forgetting the Logbook entry removes every mod that records it as its `entry`.
> Why: the entry is the one thing that says the owner taught it and withdrew it.

A mod with no `entry` stays, such as one taught on Bridge's own interface.
Switching a mod off in Mods is the way to stop one without forgetting the note.

## Safety

> **Rule.** A pattern is held to what Bridge's JavaScript engine and Fleet's both read, and an address to an address that opens.
> Why: Bridge runs every taught pattern on every message it draws, so one bad pattern would slow or hang every screen.

| Refused | Because |
|---|---|
| A pattern that does not match all the selected words | The example must be found whole, from its first character to its last |
| A pattern that matches the empty string | It would match everywhere |
| Lookaround, a backreference, an inline flag, a named group, a POSIX class, a class inside a class, an escape JavaScript lacks | Only the subset both engines read the same |
| A repeated group that holds a repeat or a choice: `(a+)+`, `(a\|aa)+` | JavaScript backtracks through it without end on a near miss |
| A pattern over 200 characters, or empty | Bounded |
| An address that is not `http` or `https`, has a space, a brace other than `{match}`, or no host | It could not open |
| `{match}` in the address's host | The selected words must not choose where it goes |
| A token that is not in the tokens file, a value `theme.css` refuses, or over 12 tokens | A tweak is checked as a theme is |
| A layout `validate_mod` refuses, or a region the registry lacks | It is checked as a layout mod is |
| A notice word no `Notice` has, or a place that does not parse | A rule must name something Fleet sends |

The codes, `fleet.teaching_words_refused` and its siblings, are in `crates/ipc/operations/read_teaching.toml`.

## What it is not

> **Rule.** A teaching never changes Armada's code.
> Why: a mod is a file checked against a whitelist, and a sentence read by a model is not safe to turn into a change to Fleet or Bridge.

Teaching changes what agents are told, what Bridge links, how it looks and is arranged, and what Fleet does with a notice it already sends.
A new kind of rule, or a different way Fleet works, is a code mod: a Job implements it, and it lands as a pull request like any other change.
**Code mods are the next plan,** and nothing here depends on them.

## Where it sits

| Piece | Home |
|---|---|
| The operations | `read_teaching`, `save_teaching`, `list_word_rules` in `crates/ipc/operations/` |
| The wire | `crates/ipc/src/teaching.rs`, mirrored by `packages/protocol/src/teaching.ts` |
| Notice words | `crates/ipc/src/notices.rs` |
| Reading, checking, saving, finding addresses, asking | `crates/fleet/src/teaching/` |
| The panel | `TeachPanel` in `packages/components/src/compositions/TeachPanel/`, and the walks `teach-anything` and `teach-mods` in the mock |
| What a Words mod and a Fleet mod hold | [Mod](mods.md) |

Fleet's three operations are built, and Bridge's calls to them are in `apps/desktop/src/main/teaching.ts`.
No real window provides a Teach source yet, so only the mock draws Teach.

## What is kept

| What | Where | Default |
|---|---|---|
| The note | The Logbook, as a standing entry with its evidence | |
| A Words, Fleet or tweak mod | `~/Library/Application Support/Armada/mods/<name>/`, `mod.toml` with `taught_on` and `entry` | On |
| An address nobody found | A call in Needs you, asked once per mod and repository | |
| A reading that was not saved | Nowhere | |
