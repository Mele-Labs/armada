# Logbook

**What it is:** What Armada has learned about a repository, about how its owner works and about this Mac, kept as short entries that every new Job and subagent is handed, written and curated with no one's approval.

---

**Kind:** Entity. Kept by [Fleet](fleet.md), read on one page of [Bridge](bridge.md).

## What it is

A Logbook is a list of entries. Each entry says in one or two lines what to do, and cites the rows of Fleet's record that show it. The owner's decision, 22 Oct 2026.

Agents in one repository kept learning the same things again.
Of the open retro items, "Chrome is not in /Applications" had been learned by 8 Jobs, "write refused before the slot was leased" by 13, and the shape of one Fleet tool's argument by 11. A retro waits on a person's review, and nothing reached the next agent while it waited.

**The Logbook has no approval step.** Nothing waits on the person. He reads the Logbook when he wants to, and Armada pushes nothing at him: no notice, no Cockpit mark, no digest.

## Three sections

| Section | Holds | Example |
|---|---|---|
| `repo` | How one repository runs and what goes wrong in it | The test command that works; a tool that refuses a write before the slot is leased |
| `habits` | How the owner works, shared by every repository | Walks before landing; no AI-sounding copy |
| `machine` | What this Mac has and lacks | Chrome is not in /Applications |

A `repo` section belongs to one repository, and a Job in another never sees it. `habits` and `machine` have no repository and go to every Job on this Mac.

## An entry

| Part | Is |
|---|---|
| `text` | One or two lines saying what to do |
| `section` | `repo`, `habits` or `machine` |
| `kind` | `command`, `gotcha`, `convention`, `environment` or `habit` |
| `paths` | Optional. The paths it applies to, so a subagent working there is handed it |
| `evidence` | The record rows that show it |
| `confirmed_by` | How many distinct Jobs confirmed it |
| `first_seen`, `last_confirmed`, `last_served` | When |
| `state` | `candidate`, `standing` or `faded` |
| `locked`, `edited` | Whether the person locked it, and whether he changed its text |

## The loop

Three steps run with no one's press, and a fourth, Tidy, runs beside them on a timer. Distil and Tidy call a model.

```
Job ends, or before a handoff
        |
        v
   Distil (Sonnet) --> Curate (code) --> Recall
        ^                                  |
        |                                  v
        +---- the Job's record      start note, subagents, recall tool
```

### Distil

**One Sonnet call when a Job ends and before a handoff, on the road a Job's retro takes** ([Retro](retro.md), *The retro call*).
It reads the Job's record since the last Distil and the repository's current entries.
It answers four things:

| Answer | Means |
|---|---|
| Confirms | An existing entry the record shows holding again |
| Contradicts | An existing entry the record shows wrong, with its new text |
| Tripped on | An entry whose warning the Job met anyway |
| Candidates | New entries the record shows |

**Every claim cites record rows, and a claim that cites none the record holds is dropped.** The call names no cause the record does not show, and "unclear" is an allowed answer.
It is written to the same rules for plain words as the retro call.

### Curate

**Curate is plain code with no model.** It reads Distil's answer and moves entries between states.

| Event | Result |
|---|---|
| Two distinct Jobs confirm a candidate | It **stands** |
| One Job shows hard evidence, a command failed and a different form then worked | It **stands** |
| An entry is contradicted | Its text is **rewritten**, and the old text is kept as history |
| An entry is tripped on | It is **strengthened**: rewritten more plainly, and moved to the top of what is served |
| A standing entry is neither confirmed nor served for 30 days and 20 Jobs of its repository, or of any repository for `habits` and `machine` | It **fades**: kept and searchable, no longer served |
| An entry is locked | It never fades and is never rewritten |
| An entry is forgotten | It goes on a deny list and never comes back |

**The deny list is the rule run memory's dropped lines follow** ([Kit](kit.md), *A private repository*). The merge reads it first, so a later Distil does not bring a forgotten entry back.

**A candidate is not served.** Only a standing entry reaches an agent.

**An entry folded in from the machine note cannot be edited, locked or forgotten.** Its id starts `machine-`, Fleet refuses every act on it, and Bridge draws none.

### Tidy

**Tidy folds near-duplicates, which Curate's word overlap misses.** Curate merges only texts at least 80% alike in words, so a command that differs by a log path or a flag, or several entries restating one refusal, pile up.

| Step | Does |
|---|---|
| Ask | Fleet makes one model call per repository, over its standing and candidate entries it may offer, with the `habits` and `machine` entries held under that repository's key |
| Answer | The model groups the entries that say the same thing and writes one text for each group |
| Keep | The survivor keeps every folded entry's evidence and takes the new text. Its earlier text stays in history with the reason Merged |
| Fade | The folded entries become faded: kept and searchable under Archived, not deleted |

**A locked entry, or one a person edited, is never shown to the model.** It is never merged and never rewritten.
**A bad answer changes nothing for that repository.**

**It runs on its own about once a week per repository, and only where an entry changed since the last tidy.** A call that failed is tried again after a day.
A person can run it now with Tidy now on the Logbook page.
It runs over every repository in the background: the button answers at once and the page says Tidying until Fleet is done, then how many entries it folded, or that it failed and how many it folded before it stopped.
A second press while one runs starts nothing, and a page opened mid-run shows Tidying.

### Sort and Organize

**Sort gives each entry one topic**: Git and PRs, Slots and worktrees, Testing and checks, Fleet and Bridge tools, or Mac and shell.
An entry with none shows as Other, and no `other` is stored.
Fleet makes one model call per repository over the entries that have no topic, locked and person-edited ones too, since sorting rewrites no text.
**A bad answer changes nothing**, as Tidy's does.
It runs at the start of every Organize pass, and after a Distil on that Distil's own task, so noting never waits on it.

**Organize looks, and the person applies.** Three looks propose and change nothing.

| Look | Finds |
|---|---|
| Merge duplicates | Tidy's groups, one model call per repository, as a kept entry, the entries it absorbs and the text it would read |
| Find stale entries | Standing, unlocked entries not confirmed in 45 days and not tripped in that time. No model is asked |
| Find Armada bugs | One model call per repository: the entries that read as workarounds for a fault in Armada, each with the fault in a line |

Applying a merge folds as Tidy does. Applying stale entries archives them. Applying bugs writes nothing and answers the list back, since Fleet never writes to a forge.
Apply reads the entries again, so a proposal over one locked, edited, archived or forgotten since is skipped.
The weekly Tidy still runs on its timer.

### Recall

Standing entries reach agents three ways.

| Way | What is handed over |
|---|---|
| The start note of every new Job | `habits` first, then the repository's standing entries, then `machine`. Capped near 2k tokens, and what the cap leaves out is found with `recall` |
| A subagent | The `habits` and the entries whose `paths` match the paths its prompt names, near 800 tokens, appended to its prompt by the Armada mod's `agent.spawn` hook ([Installing the Claude Code mod](session/installing-the-claude-code-mod.md)). A prompt that names no path gets the `habits` and the repository's best entries. A Fleet that does not answer hands it nothing |
| The `recall` tool | A search of the whole Logbook for the Job's repository, its `habits` and `machine`, faded entries included, mid-task: the words of `query` held in an entry's text, those holding more first, then those more Jobs confirmed. It marks nothing served |

**Serving an entry sets its `last_served`,** on the start note and on a subagent's, never on a search. A strengthened entry leads its section when the cap cuts.

## What the person does

**The person sees one Logbook page in Bridge.** He can read an entry and its evidence, edit it, lock it or forget it.
Nothing else is asked of him.
**The page reads as a log.** On the left, one table of entries, a line each: its kind as a code, its title, its flags, how many Jobs confirmed it and when one last did.
A row heads each repository's entries, then Habits' and This Mac's, and the column head and that row stay as the table scrolls.
On the right, the open entry under its own head, which holds its kind, id, flags and the acts. Its title stays above, and its rule, its record and its evidence, a row a Job, scroll under it.
Three tabs say which entries the log holds: In use, the entries handed to every new Job, which is where the page opens; New, the candidates; and Archived, the faded ones.
A fourth, Retros, holds the [retro](retro.md) items in place of the entries: what got in a Job's way and where its fix lands, drawn as a log and the item open beside it.
**An entry's title is a few words naming it**, written by Distil with the entry.
An entry written before titles were is given one on a later Distil run, and until then its line reads the first clause of its rule.

| Act | Does |
|---|---|
| Edit | Replaces the text and marks the entry `edited` |
| Lock | It never fades and is never rewritten |
| Forget | Puts it on the deny list |
| Tidy now | Starts Tidy over every repository in the background and says how many entries it folded when done |

He can also teach an entry by pressing Teach on words, a command, a notice, a task, a Check or a pull request.
A taught entry stands at once, cites what it was taught from as its one evidence row, and forgetting it removes the mods the same teaching made. [Teaching](teaching.md).

## Where it is kept

**The Logbook is kept by Armada and never committed to a repository.** Many repositories do not commit this kind of knowledge, a work repository among them.

A `repo` section follows the place that repository's Armada configuration is kept ([Kit](kit.md), *What a repository runs*), as flight controls do:

| Place | Logbook kept |
|---|---|
| On this Mac | In `~/.armada`, keyed by the repository |
| Synced | The same layout under the synced folder |
| Cloud, a bucket | One object in the bucket |
| In the repository, or Armada's store | On this Mac, as above |

**The last row holds because a Logbook is never written into the repository,** and Armada's store keeps only runs. `habits` and `machine` are kept on this Mac.

## How it relates to what exists

| Existing | Relation |
|---|---|
| [Retro](retro.md) | Keeps only what Armada itself got wrong: bugs for Fleet or Bridge. A repository fact stops going to the Retros page, and Distil takes it. The open items that are really repository facts move into the Logbook once, and close as `discarded` (*What the Logbook takes* in Retro) |
| Run memory, in [Kit](kit.md) (*What a repository runs*) | Kept apart. It keeps every command that worked word for word, so its facts are not entries: folded in, one-off runs buried what Jobs learnt. The owner's decision, 23 Oct 2026 |
| The machine note, in [Machine](machine.md) (*What a Job is told about this Mac*) | Keeps its detector. Its facts become entries of the `machine` section, and its note is part of the Logbook note |
| The start note | The machine note becomes part of the Logbook note |
| Claude Code's own memory | Untouched. It is shared by every worktree of a repository on one Mac and written at an agent's choice. The Logbook is written for every Job and curated |

**No memory tool from outside is used.** claude-mem, Mem0 and Graphiti were weighed and turned down.
Fleet already records every Job, so the capture half exists, and the Logbook adds Distil and Curate on top of it.
