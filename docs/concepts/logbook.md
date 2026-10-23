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

Three steps run with no one's press. Only the first calls a model.

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

**An entry folded in from run memory or the machine note cannot be edited, locked or forgotten.** Its id starts `runs-` or `machine-`, Fleet refuses every act on it, and Bridge draws none.

### Recall

Standing entries reach agents three ways.

| Way | What is handed over |
|---|---|
| The start note of every new Job | `habits` first, then the repository's standing entries, then `machine`. Capped near 2k tokens, and what the cap leaves out is found with `recall` |
| A subagent | The `habits` and the entries whose `paths` match the paths its prompt names, near 800 tokens, appended to its prompt by the Armada mod's `agent.spawn` hook ([Installing the Claude Code mod](session/installing-the-claude-code-mod.md)). A prompt that names no path gets the `habits` and the repository's best entries. A Fleet that does not answer hands it nothing |
| The `recall` tool | A search of the whole Logbook for the Job's repository, its `habits` and `machine`, faded entries included, mid-task: the words of `query` held in an entry's text, those holding more first, then those more Jobs confirmed. It marks nothing served |

**Serving an entry sets its `last_served`,** on the start note and on a subagent's, never on a search. A strengthened entry leads its section when the cap cuts.

## What the person does

**The person sees one Logbook page in Bridge.** He can read an entry and its evidence, edit it, lock it or forget it. Nothing else is asked of him. **The page has two parts**: what agents know, one panel per repository and then Habits and This Mac, and below it Armada fixes, the [retro](retro.md) items: what got in a Job's way and where its fix lands. Three tabs say which entries the panels hold: In use, the entries handed to every new Job, which is where the page opens; New, the candidates; and Archived, the faded ones. An entry draws the same in each tab, and its state shows in the open entry. Armada fixes stays below in every tab.

| Act | Does |
|---|---|
| Edit | Replaces the text and marks the entry `edited` |
| Lock | It never fades and is never rewritten |
| Forget | Puts it on the deny list |

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
| Run memory, in [Kit](kit.md) (*What a repository runs*) | Keeps its detector, which reads results off a Job's stream with no agent's word. Its facts become entries of the `repo` section, and its note is part of the Logbook note |
| The machine note, in [Machine](machine.md) (*What a Job is told about this Mac*) | Keeps its detector. Its facts become entries of the `machine` section, and its note is part of the Logbook note |
| The start note | The run note and the machine note become one Logbook note |
| Claude Code's own memory | Untouched. It is shared by every worktree of a repository on one Mac and written at an agent's choice. The Logbook is written for every Job and curated |

**No memory tool from outside is used.** claude-mem, Mem0 and Graphiti were weighed and turned down.
Fleet already records every Job, so the capture half exists, and the Logbook adds Distil and Curate on top of it.
