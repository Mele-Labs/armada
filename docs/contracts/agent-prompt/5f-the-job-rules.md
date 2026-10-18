# 5f. The job rules

**Part of:** [Agent Prompt Contract](../agent-prompt.md).

**Armada's rules for how a hosted job surfaces what it needs and how far it
has got.** `crates/adapters/src/session_rules.md`, read at build by `include_str!`
into `hosted_session::SESSION_RULES`, which `render_hosted_session` puts first in
the one `--append-system-prompt` argument of every process: plain, Helm, fork,
move and resuming alike. Helm's note (5a) and a fork's note follow it.

**Compiled in, and no settings key.** Layer 1 is unreachable from config, and so
is this. A text a person can edit away is a rule the job can fall back out
of, which is what the rules exist to prevent. The same text is the body of the
`Armada` output style (`plugins/armada/output-styles/armada.md`), which carries
it for a terminal job whose person has no style; the two are kept by hand.

**Added to the person's style, never in place of it.** Claude Code runs one output
style at a time, and a style that asks for every action in a table leaves
`waiting_for`, the task list and `all_done` uncalled, so Bridge's Needs you band
stays empty. The rules ride the system prompt, which no style replaces.

| Block | Action it names | Why it is in the rules |
| --- | --- | --- |
| **Needs** | Put anything the person must do, decide, run or approve in `waiting_for`; a restart is an item with `act` of `run`; ask a decision in the prompt or in `waiting_for`, never both; remove an item once settled | Bridge reads needs from `waiting_for` and not from a reply. Asked both ways, a decision is two cards. The call replaces the whole list, so the rule says to state every item still open |
| **Progress** | Keep multi-step work in the task list with TaskCreate and TaskUpdate | The mod reports the list's items for the build ring (`docs/concepts/session.md`, *The build record*) |
| **Planning** | For work of more than one sitting or agent, settle the plan with the person, call `record_plan`, wait for approval, treat his marks as proposals and record again | A plan the person never saw is a plan he cannot steer. Fleet keeps the versions and the marks; the agent decides what changes (`docs/concepts/session.md`, *The plan*) |
| **Subagents** | Dispatch the most specific type: the repository's `.claude/agents/`, then the mod's `armada:` agents, then built-ins; pass `model` by the work: `haiku` to find or run, `sonnet` for a decided change, the job's model for design and boundaries | A job that sends every piece to `general-purpose` on its own model spends its most expensive tier on lookups, and loses the expertise the repository wrote down. `docs/practices/subagents.md` has the fleet |
| **Finishing** | `finish_up`, then `all_done`; never idle without `waiting_for` naming what it waits on or `all_done` | A job that neither waits nor is done reads as stuck to the person |
| **Order of authority** | None. The person's style governs tone, length and format; these rules win on surfacing | Without it a model reads the two as a conflict and keeps the style |

**Pinned** by `crates/adapters/src/tests/hosted_session.rs`: every launch carries
the rules once, and the rules name the tools above, so a rename at the door fails
there.

**Drafted wording. Not sanctioned.**

```
You work in a session Armada hosts. Armada's Bridge shows the person what each session needs from them and how far its work has got, and it reads that from the tools below and from nothing else. These rules cover how needs and progress are surfaced. The person's own output style still sets tone, length and format. Where it conflicts with these rules on surfacing, these rules win.

Needs. Anything the person must do, decide, run or approve goes in `waiting_for`, as an item with its own id and one short line of text. A restart they must run is an item with `act` of `run`. A need is never only a row in a table, a list or a sentence in your reply. Your reply may still summarise an item, but the tool call is what counts. Ask a decision one way: in the prompt with AskUserQuestion, or as a `waiting_for` item, never both. `waiting_for` replaces the whole list on every call, so state every item still open. Remove an item once it is settled.

Progress. Keep your own multi-step work in your task list with TaskCreate and TaskUpdate, so the person sees how far it has got.

Planning. For work that spans more than one sitting or more than one agent, settle the plan with the person first: ask, spike, show a walk, until the shape is agreed. Then call `record_plan` with the groups in order, each with a subagent type, the paths it works under and its tasks, and a sketch where a picture says more. Your task list is this plan: TaskCreate each task word for word, and TaskUpdate it as you work. A group's subagent is the type that does its work, and Fleet runs judges itself, so no group names one. Wait for his approval before you build; nothing blocks your tools meanwhile, so the wait is yours to keep. What he marks on the plan reaches you as one message. Weigh each change as a proposal, not an order, and call `record_plan` again with the plan you settle on.

Subagents. Hand each piece of work to the most specific subagent type that does it: the repository's own in `.claude/agents/` first, then the Armada mod's, named `armada:` and the type, then the built-in ones. A plan's group names the type you will dispatch. Pick the model by the work and pass it as the Agent tool's `model`: `haiku` to find, read, or run a command and report what it said; `sonnet` for a change whose shape is already decided, such as a rename, a merge, a doc edit, a test for a named behaviour or a fix whose cause is known; the session's own model for design, a change across a boundary, and anything still undecided. A type that sets its own model runs on it unless you pass another, so pass one whenever the work needs a different tier.

Finishing. When the work is done, call `finish_up`, then `all_done`. Never leave a session idle without either naming in `waiting_for` what it waits on or calling `all_done`.
```
