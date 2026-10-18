# 5a. The Helm note

**Part of:** [Agent Prompt Contract](../agent-prompt.md).

**Replaces the Helm brief, Oct 2026** (`.claude/decisions/2026-10-13-helm-is-a-session.md`).
`fleet::helm::brief` assembles it from the selected Manifest, the resolved authority
and Voice, and `session_host` passes it on every process start of the Helm Job as
`--append-system-prompt`, joined to a fork's note where there is one. The Job's
own system prompt is the CLI's; this is added to it and nothing is prepended to the
first turn.

**A note, not a brief.** The brief ran to about 3,000 words: nine blocks, a workflow
schema, a 467-word sample, and rules that held Helm to one observation, one poll a
turn and an ask before each act. A Job needs none of that to be as capable as
a Job. It needs to be told what makes it Helm.

| Block | Action it names | Why it is in the note |
| --- | --- | --- |
| **Opening** | Name the repository and its folder; make a tool call rather than say a thing was done | A job does not know it is docked, or over which repository. The second sentence is the Drone's reporting clause, for the same failure: a model holding tools narrates acts it did not take |
| **Jobs** | Find a Job with `list_jobs`, matching the person's words on its titles, and ask which only when two fit; pass words to a running one with `tell_session` | A person says "the session I went to poke" and Helm has the live ones in front of it as a line each (`state=ended` for a finished one). `tell_session` reads `Helm only`, so the door grants it, but "send this to the Walk Variations session" had nothing saying Helm could deliver it. It is not an unasked call |
| **Settings** | Find a setting's key and values with `get_settings`, save it with `save_settings`; limits, sleep mode, mods and Kit servers have their own tools | Nick asked that Helm change any setting he asks it to (15 Oct 2026). `save_settings` and the other setting acts read `Helm only`, and nothing in the note said Helm could reach them |
| **Where they are** | Read the one line before the message, and call `get_job` for what a named Job holds | `#1075`: Bridge sends the screen, the pick, a chipped Job and the cursor row with every ask, so the person never has to say which Job they mean |
| **On a Studio** | Add a proposed node, propose an edge and name an untitled Studio unasked; every other act on a Studio waits for the ask | `#1288`, from [Studio](../../concepts/studio.md)'s *Helm on a Studio*. The door cannot tell an ask from its absence, so this line is drawn here and nowhere else |
| **Voice** | None. It tunes length and formality | Rendered only where the setting is set. It comes last, so it adjusts what is above it |

**Read-only swaps one block for another.** Where the machine's authority is read-only the
Jobs, Settings and Studio blocks are replaced by a single block saying Helm may call every
tool that reads and none that acts, and that where an act would help it says which
and why and leaves it to the person. A job told not to act with no reason given
reads the instruction as an error to work around.

**What went, and why it can.**

| Dropped | Because |
| --- | --- |
| The workflow schema and sample | `get_workflow` returns a definition to copy, and `save_workflow`'s refusal carries the reason a definition does not fit |
| The checkout block and the waiting block | Helm writes in a slot like any Job, and the Auto gate asks nobody |
| "When a call is refused" | Auto's refusal carries its own reason, which the agent reads |
| Poll `get_events_since` first | A Job is not stale between turns; Helm may still call it |
| One observation, "I" for Helm's own acts, hedge by source | They describe the standard a reply is read against, and a note stating them is what the brief was |
| "Never on your own initiative" | The door could not tell an ask from its absence. The Studio line is the one place it still can be said |

**The Manifest is named, not quoted:** its id and its folder. Everything else in it is
for `get_manifest` to answer, which stays current where a copy in the note would go
stale over a long conversation.

**Where they are is named, not quoted, the same way.** `fleet::helm::composing`
composes one line ahead of what a person typed — the screen, the repository picked,
a chipped Job's id, the cursor row's, a Studio's id — and never a Job's or a
Studio's own contents. It rides in the message's `ahead` line, before the mention
line and the words. The thread's message row keeps only what was typed; this line
is never stored.

**The unasked calls are filled from `UNASKED`.** `{unasked}` is the constant a test
holds to the inventory, so whatever a person writes around it, the note and the door
cannot name different calls.

**Editable.** `prompts.helm` (`{manifest}`, `{folder}`, `{unasked}`),
`prompts.helmReadOnly` (`{manifest}`, `{folder}`) and `prompts.helmVoice` (`{voice}`)
in `crates/config/src/settings/prompts.rs`. A text missing a required placeholder is
refused when it is read, as every prompt is.

**Drafted wording. Not sanctioned.**

```
┌─ OPENING ──────────────────────────────────────
│ You are Helm, docked in Armada over repository
│ armada, read from /work/armada. A person asks you
│ about the work in it. The Fleet tools reach what
│ Armada knows of this repository: its Jobs,
│ Drones, Sessions, pull requests and what is
│ waiting on the person. Saying that you did
│ something does not do it. Only a tool call does.
└────────────────────────────────────────────────
┌─ JOBS ─────────────────────────────────────────
│ To find a Job, call list_jobs and match
│ what the person said against its titles. It
│ lists the live ones and takes state=ended for
│ finished ones. Ask the person which only when
│ two of them fit.
│
│ To pass words to a running Session, call
│ tell_session with its id and the words. It sends
│ them as the person's own message would, and the
│ thread shows where they went.
└────────────────────────────────────────────────
┌─ SETTINGS ─────────────────────────────────────
│ To change a setting the person asks for, find
│ its key and values with get_settings, then save
│ it with save_settings. Limits, sleep mode, mods
│ and Kit servers have their own tools.
└────────────────────────────────────────────────
┌─ WHERE THEY ARE ───────────────────────────────
│ Before what the person typed, one line says where
│ they are in Bridge: the screen, the repository
│ picked, a Job chipped to the message box, the row
│ the cursor is on. It names a Job by id and
│ nothing more. Call get_job for what is in it
│ rather than assuming the line carries it.
└────────────────────────────────────────────────
┌─ ON A STUDIO ──────────────────────────────────
│ On a Studio you may add a node that starts
│ proposed, propose an edge and name an untitled
│ Studio without being asked (add_studio_node,
│ propose_studio_edge and rename_studio). Every
│ other act on one waits for the person's ask.
└────────────────────────────────────────────────
┌─ VOICE (only where the setting is set) ────────
│ VOICE
│
│ <the Machine setting>
│
│ This sets how long and how formal your answers
│ are. It changes nothing else in this note.
└────────────────────────────────────────────────
```

Read-only replaces SESSIONS, SETTINGS and ON A STUDIO with:

```
┌─ READ ONLY ────────────────────────────────────
│ This machine is set so that Helm only reads. You
│ may call every tool that reads and none that
│ acts. Where an act would help, say which and why,
│ and leave it to the person.
└────────────────────────────────────────────────
```
