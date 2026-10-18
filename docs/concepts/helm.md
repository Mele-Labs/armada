# Helm

**What it is:** A dock over the content on every Bridge surface, holding the questions waiting on you and one [Job](session.md) per repository.

---

**Kind:** Job, Surface. Why it is a Job: `.claude/decisions/2026-10-13-helm-is-a-session.md`.

Companion to the main Armada brief.

## What it is

**Helm is a hosted [Job](session.md)**, one per repository, marked `helm` on its record.
It has everything a Job has — a live process under a keeper, Auto, a slot and a branch on its first write, `tune_session`, interrupt, subagents, questions, attachments and mentions — and what makes it Helm is a short note appended to its system prompt, a line before each message saying where you are in Bridge, and the door's `Helm only` rows.
This page owns what Helm adds; [Job](session.md) owns the rest.

Helm is additive to Bridge, not a replacement: Bridge is state-monitoring (what's happening), Helm is reasoning/synthesis across that state (why, and what to do about it).

## What it's for

| Capability | Example |
| --- | --- |
| Cross-Session pattern reasoning | "Why do all my stalled Jobs this week share a cause?" |
| Natural-language control surface | Talk to it instead of clicking through Bridge |
| Authoring a workflow | Work out with it what a Job should follow, and end with the definition saved in this repository or in Kit |
| Planning / dispatch help | Describe a goal, Helm breaks it into candidate Jobs to start |

Cross-Session reasoning covers every Job in the selected Manifest, not one Job at a time. The control surface is for exploratory questions.

**Helm drives a [Studio](studio.md).** It may add proposed nodes and propose edges unasked, and starts a [Scout](scout.md), a Run or a dispatch only on your ask. The Studio page owns which is which.

## Placement

**Helm is a dock, not a surface.** It draws over the content on every Bridge screen at 1100px and wider, folding to a sheet below that, opened from the title row's Helm button at every width; `⌘J` toggles it from anywhere, including from inside a field.
**Bridge opens with it shut, and opening it takes no width off the screen behind it** (#1583, owner 22 Sep 2026) — the content under it stays live, and Close or `⌘J` is one press.
It carries no rail digit and does not appear in the sidebar.
See `../contracts/design-system.md`, Two tiers, for the layout.

The dock's upper zone lists every question waiting on a person, from every repository Fleet serves, each card naming its own repository.
Answering one there settles it in that Job's own thread too.
The lower zone holds Helm's conversation.

**One Helm Job per repository, with Start fresh.** Which repository Helm answers for is set by the most recent explicit act — picking a repository, "Discuss with Helm" on a card, or the dock's own switch — never by the picker moving on its own.
**The first message the dock sends for a repository starts its Helm Job, and the next reuses it**; a second repository gets its own.
At most one is live per Manifest, and Fleet finds or starts it under one lock, so two first messages cannot make two.
It stays until Start fresh, which ends it (`close_session`, giving back any slot it held), and the next message starts a new one.
Nothing sweeps it on a clock.

**Every ask names where the person is.** Bridge sends the screen, the picked repository, the open Job chipped above the message box, and the Cockpit cursor row with each message; Fleet puts one line naming it ahead of what was typed, and the thread keeps only what the person wrote.
`#1075`.

**Fleet-wide reasoning is not a Helm capability, with one exception.** Why: a conversation belongs to the repository it is pointed at, so a Fleet-wide Helm would be the only thing in the product ignoring that.
The exception is the read that routes a thought (*Routing a thought*, below), which looks at every repository's live Jobs whatever repository Helm points at.
The owner chose it on 12 Oct 2026, because the work a thought belongs to is often in another repository.
It reads and answers where the words belong.
It does not reason about them, and nothing else in Helm becomes Fleet-wide.

Anything else that is genuinely cross-Manifest stays a Bridge job.

**It is in the Jobs list too.** A Helm Job is a row of `list_sessions` under its repository with `helm: true`, which Bridge draws as Helm's, and opening it from there opens the dock.
Helm's own `list_sessions` answers leave it out, and so does the thought reader: Helm is not a place to route a thought to.

**Old Helm threads stay on disk, unread.** The files under `<records_root>/helm/` belonged to the host Helm no longer uses. Armada is pre-alpha, so nothing migrates them and nothing deletes them.

## The message box

**Helm's box is a [Job](session.md)'s**, with the files and pictures, the sketch pad, `/`, `@` and `#`, the model and effort, and `Enter` to send with `⇧Enter` for a new line.
It has no permission mode: Helm runs in Auto like any Job.
The look is the walk `helm-rich-composer`.

| The box offers | Where it comes from |
|---|---|
| Files, pictures and sketches | Sent with the ask as `attachments` and kept as a Job keeps them (`uploads`), outside every worktree. A sketch goes as the picture it renders to |
| `@` | Jobs, as Bridge holds them, sent as `mentions`. Fleet writes one `The person named:` line ahead of the words, as a Job's send does. Once letters follow the `@` it also lists the repository's files that match them, which stay plain text in the words |
| `#` | The repository's open issues, its open pull requests and, for a number that is neither, one read by number. An issue tags as `issue`, with its number, title and address |
| `/` | The commands the agent named on its `init` line, as a Job's list. Until Helm's Job has started the list is the last one any Job read |
| Model and effort | `AskHelm.model` and `effort` choose them on the message that starts the Job (422 `fleet.unacceptable_effort`); `tune_session` changes a live one. Start fresh keeps nothing: the next Job starts on the box's pick |

**A message may be only a file or a tag; blank words with nothing carried are refused** with a Job's `fleet.session_message_empty`.
**The thread is the Job's.** The dock reads it with `get_helm_session` and follows `session.row` and `session.changed` on `/events`, the rows a Job page draws.
A person's message is a `message` row with the files and tags beside the words.
Start fresh ends the Job and the next one has a new thread.

**What the thread names is a link.**
`#N`, `owner/name#N`, the forge's address for an issue or a pull request, and a Job's id, address or exact title link in Helm's replies and in the person's own rows, and so do the `@` and `#` chips of a sent row.
**The Cockpit's Job thread links the same words**, read in that Job's repository.
A pull request opens on its row of the Merge line, and with nothing highlighted where the Merge line has no row for it; a Job opens as it does elsewhere; an issue of this repository opens in the composer's open issues.
A number of another repository (`owner/name#N`, or a github.com address that is not this repository's) is only its forge page and opens in the browser.
**Code is not read for numbers**: a `#N` in a span or a block stays plain, and a span that is exactly a Job's id links and stays code.
Bridge reads only what its window holds, so a number that resolves to nothing stays text, and a number that is not open is read once by number, as `#` reads it.

## Routing a thought

**Every message typed in Helm's box is read before it is asked.** Helm decides whether it is a question for Helm or a thought meant for work that is already running, and there is no key to say which.
A question goes to Helm as it always did.
A thought gets a card over the box: a match names the Job, with Send (`↵`), No (`N`) and Ask Helm (`H`); no match, or No, offers a new Job (`S`) in the Dispatch panel, holding the words, with Ask Helm (`H`) and Esc beside them.
The keys act only while the box is empty.
The look is the walk `route-a-thought`.

**Messages sent while a card is up queue behind it, and none is lost.** Each is read as it is sent, and the cards show one at a time in the order typed: answering one (`↵`, `N` then `S`, `H` or Esc) shows the next.
A message that reads as a question goes to Helm as soon as it is read and never waits behind a card.
Nothing marks how many are waiting.

**Fleet reads, Bridge delivers.** `read_thought` hands the words and every live Job in every repository to the model the proposer uses, with each one's title, repository and ledger (branch and pull request titles).
A Job that has ended, or has no title, is not offered.
A reading names a Job or none.
The model answers one line of JSON.
An id it names that was not offered is no match, and anything it answers that is not that JSON is a question.
**If anything fails, the words go to Helm.** A message that opens on a wh-word and ends in a question mark skips the call, since it is plainly Helm's, and a request in a question's clothes, such as "can you make the spike cover the jobs table?", still goes to the model.
A read measured 5.0 to 7.5 seconds against a live Fleet, most of it the model's CLI starting, so Fleet waits up to twenty before it gives up and the words go to Helm; Bridge shows the turning mark meanwhile.
At eight seconds the slower reads were counted as questions, and a request meant for a Job reached Helm, which had no way to pass it on (*Tools*, below).

**Send is the acts that already exist.** A Job gets a message as the composer sends one.
A refusal, such as a terminal Job that is not listening, is said in Fleet's words and nothing is recorded, and the card stays for another try.

**The thread keeps where the words went.** Once they arrived, Bridge says so (`record_thought_sent`) and Fleet appends a `sent` row (`SessionRow::Sent`) to the Helm Job's thread, which draws as the person's words with a "Sent to" chip that opens the Job.
It is a row of the Job's ledger, so it outlives the window and a restart, and it is never fed to Helm's process: the Job hears only what is asked of it.
The answer is the Helm Job's record, started if the repository had none live.
The audit trail has it as `helm.thought_routed`, naming where and never the words.

**Words that reach Helm as a request are passed on by Helm.** "Send this to the Walk Variations session: ..." is a question as far as the reading goes, so it arrives in Helm's job, and `tell_session` is how Helm delivers it.
It sends through the same path as the composer, so a terminal Job that is not listening is refused in the same words and Helm says so.
On success Fleet keeps the same `sent` row and publishes the same `helm.thought_routed`, so the thread shows where the words went whichever way they got there.
A refused send records neither.
It runs on the person's ask with no card: it destroys nothing, pushes nothing and writes nothing off this machine.

## Tools

**Helm holds what a Job holds, and the Fleet MCP besides.** The Helm Job opens in the repository and resolves what your Claude configuration resolves there — your MCP servers, plugins, skills, agent files, and the ordinary built-in tools.
Nothing is withheld by a list Armada wrote.
What Helm is told is a short note, `../contracts/agent-prompt.md` section 5a; it holds no workflow schema, sample or standing rule beyond the few the note names.

**Kit is what should resolve that set, and Kit is not built (#1275).** Until it is, the set is whatever Claude Code itself resolves for this repository on this machine: your user-scoped configuration and the repository's own.
Kit takes it over, and a Manifest tier arrives with it.

**An unattended launch's confinement is not Helm's, and widening one must never widen the other.** `--strict-mcp-config` exists because an agent came up holding every server an operator had connected — `../scope.md`.
An unattended agent has no person watching; Helm is a person talking, in their own checkout, to a job they are watching.
The two launches are rendered separately and a test holds them apart.

Helm's advantage is not more access than you have.
It is reasoning over results and chaining calls you would otherwise make one at a time — "why is that Job stuck", "what needs my attention, one by one", "what is open that we could dispatch".

**Helm gets every query, and every command it is offered.** `../contracts/system-architecture.md`, section 6 carries the inventory of queries, commands and events.
`fleet::helm::may` is the line: every read is Helm's, and a command is Helm's when the machine's authority setting allows acting at all and the act is not in `fleet::helm::reach::RESERVED`, which is empty.
With action authority set to read-only, Helm is refused every command.

**Helm authors a workflow through the door, and the door teaches it.** The schema and sample left the prompt.
`get_workflow` returns a definition to copy the shape from, and `save_workflow` checks one against this repository's Checks and this machine's models with the loader's own rules, refuses with the reason and writes nothing where it does not fit.
The person chooses the place: the repository's own `.armada/workflows/`, or Kit's `~/.armada/workflows/` for every repository on the machine. [Workflow](workflow.md), *Where a definition comes from*, owns which place wins.

**Helm passes words to a live Job with `tell_session`, on an ask.** It reads `Helm only` in the inventory, so no other agent's door job is offered it, and the note names it in one line (`../contracts/agent-prompt.md`, section 5a).
*Routing a thought* has the rest.
**The door is a Job's door plus Helm's extras**: `tell_session`, `ask_the_person` and the other `Helm only` rows.
Every operation the door offers is tagged,.

**Helm changes any setting on an ask.** Nick asked for it on 15 Oct 2026, so `save_settings`, `set_sleep`, `set_mod_enabled`, `set_repository_profile`, the Kit server acts, `get_limits` and `get_model_downshift` read `Helm only`, beside `save_limits` and `save_preferences`, which already did.
The note's Settings line sends Helm to `get_settings` for a key and the values it takes.
A read-only Helm calls none of them, so it cannot raise its own authority.

**On a Studio, a few commands are Helm's without an ask**: adding a node that starts proposed, proposing an edge, and naming an untitled Studio — `fleet::helm::reach::UNASKED`, named in the note.
The door cannot tell an ask from its absence, so that line is drawn in the note and not at the door.
It is the one standing rule the note keeps.
See [Studio](studio.md), Helm on a Studio.

**The WebSocket events are Bridge-only.** Why: an agent cannot be interrupted mid-turn.
Helm may call `get_events_since(cursor)` for what happened since it last looked, a count plus one line per event kind, and is no longer told to at the start of every turn.

**Fleet does not wake Helm.** Why: it would make Helm a second notification channel alongside the Cockpit's calls.

**Every call is scoped to the selected Manifest, except Doctor, which Helm reads machine-wide.** Doctor's modules are machine-level by nature — the Fleet daemon, disk, Armada API reachability — and cannot be Manifest-scoped, so under strict scoping Helm could not answer whether the daemon is healthy.

Machine health is often the cause: three Jobs stalled because the disk filled is exactly the cross-Session pattern Helm exists to find, and a strictly-scoped Helm would see three stalls and no reason.
It leaks nothing, because the Manifest boundary exists to stop one project's work being visible from another, and machine health is **shared context rather than another project's work**.

What Helm may *do* rather than read is a separate limit — see Action authority below.

## Action authority

Helm may call any command it is offered, and the machine's authority setting decides whether it is offered any.
**With action authority set to read-only, Helm is refused every command**, and its note says so (`prompts.helmReadOnly`).
No command is reserved to a person now (`fleet::helm::reach::RESERVED` is empty).

**Helm writes in a slot, not in your checkout.** Its Job is a Job: the first write leases a slot and cuts a branch, as any Job's does (`session_host::gate`), and the thread gets the `lease` row.
The change is reviewed the way a Job's is, on a branch.
This reverses the 17 Sep 2026 exception that let Helm edit the main checkout with no branch, and `helm.changed_checkout` went with it.
`.claude/decisions/2026-10-13-helm-is-a-session.md`.

**Permission is the Job's Auto gate.** A call runs unless it reaches past the slots the Job holds.
A read of a folder macOS guards or your credentials, a write into your own files, and a `Write` or `Edit` outside the slots and `/tmp` are put to you on a card with Allow once, Reject once, Always allow and Always block; the last two name a folder of the path, and go on the folder list (`features.reachFolders`, in Settings, Features) the gate reads before it draws a card: blocked is refused with no card, allowed runs, the deeper entry wins and a block wins a tie.
Unanswered inside the hold is a refusal.
A removal or overwrite outside the slots and `/tmp` is refused with the reason, which the agent reads, whatever folder is allowed.
A question the agent asks is put to you as a card, as a Job's is.
**A push runs**: Auto does not classify it, so `git push` and `gh pr merge` are not stopped here.
The three classes Helm used to ask about (destructive, pushes to a shared space, writes off this machine) and their card are gone for the dock, and so is the 5-minute silence that refused.

**"On your ask" is how Helm is meant to behave and not a gate.** The door never could tell an ask from its absence, and the note does not state the rule in prose: a Job acts on what it is told, and Helm is told what the person said.
The one place the line is still drawn is a Studio's unasked acts, in the note.

## Audit trail

**Every Helm-initiated action is logged as its own distinct event type**, never conflated with your manual actions.

**A Helm write is a Job's write**: the `lease` row and the branch say where it happened, and the acts a Helm connection took through the door are recorded as Helm's by the connection they arrived on (`HelmCalled`), never by a field in the request.
`helm.changed_checkout` is retired.

**Words Helm's reading sent to a Job are `helm.thought_routed`**, naming the repository whose thread holds them, the kind and the id it went to.
A person who finds a message in a Job that they typed into Helm reads this to see where it was sent from.
A person's press delivers it and Helm's reading chose the target.

**An answer to a Helm Job's question is `helm.call_answered`**, naming the repository, the tool and the end it came to.
The Auto gate does not ask, so there are no `helm.asking_to_run` cards for the dock and no `ran_unasked` record; a Job's refusals are on its thread as a Job's are.

**On a Studio, Helm's act is `studio.helm_acted`**, published beside the `studio.changed` every write publishes, naming the Studio, the act and what it added.
A person's act on a Studio publishes `studio.changed` alone, so the two are told apart by kind rather than by a field someone has to remember to read.
**The record keeps it too**: each node and edge carries `added_by` and a Studio its `named_by`, a person or Helm, so a client that was not connected when Helm acted reads who did what off `get_studio`.

## Reporting a bad answer

**A job is carried as one record, with the same act every error carries.** *Copy debug info*, bound to `c`, one producer formatting exactly what is copied, and the expanded view rendering that same string — `../contracts/design-system.md`, *The debug payload*.
On 17 Sep 2026 Helm refused a question and the only way to report it was retyping the conversation by hand, without the note, the tools, the authority or the events that decided the answer.

**One press copies it, and *Details* beside that opens the same artifact to read.** That is the banner form of the error treatment, and the dock is the standing surface it is for: Helm has just answered badly and the record is wanted now, so the reading is second rather than in the way.
The disclosure takes the banner's own word, and the sheet it opens is titled *Job record*.
`get_helm_debug_info` answers the repository, its authority and model; the note as it is sent; the tools the door offered by name; the thread's messages, calls, refusals and "Sent to" rows; what the last poll was answered; and the protocol Fleet speaks beside the Fleet process that answered.
It carries no per-turn cost or "fresh" and "unanswered" lines: a Job's thread has none.

**It says what it holds rather than promising what it does not.** A thread carries the person's own words and Helm's, and the checkout names a folder on their disk; nothing here is a structured field a type bounds, so the sentence under it states that and makes no claim about the rest.

**One artifact taken at a moment, and not a second observe.** The Job's thread streams to the dock as `session.row`; this is read once, when a person asks for it.
A long job is bounded and the record says how many lines it left out, the way a log tail does.

## Job model

**One Helm Job per repository, not per topic.** See Placement above for how it is reached and ended.
A job per topic would require deciding when a topic has ended — a judgement nobody wants to make mid-investigation — so the boundary is the repository, which already exists, rather than a question.

## Budget & cost

| Aspect | Resolution |
| --- | --- |
| Hard gating | None — Helm is outside any $ / quota-% gating |
| Visibility | Helm surfaces its own usage and cost; nothing enforces a cap |

The exception is deliberate, against the Known Risks "cost / rate-limit blowup" mitigation elsewhere in the brief: Helm is a human-driven tool you're actively steering in real time, not autonomous background spend.
Revisit if usage patterns prove this wrong.

## Voice & conduct

Helm is the only surface in Armada that speaks in first person. These are behavioural constraints, not styling. Source: `../contracts/design-system.md`.

**The note no longer states these.** The old brief spelled out the one observation, the first person and the hedge; the note keeps the Machine Voice block and nothing else of them, on the reading that a Job needs saying what it is and where, and not how to write.
They stay the standard Bridge and Fleet hold their own words to, and the standard a reply is read against.
If Helm's replies drift from them, the fix is a line in the note, in `agent-prompt.md` section 5a first.

**First person is Helm's alone, and only for what Helm itself did.** Bridge and Fleet never say "I". Helm says "I" for its own acts, its own reasoning and its own suggestions.

**Reporting a Fleet-originated event uses the same impersonal phrasing Bridge does** — "Job 4 stopped reporting", never "I paused Job 4".
Why: if Helm narrates Fleet's work as its own, the separation between Fleet's events, human action and Helm action stops being legible in the one place a human actually reads it.

**Answer, plus at most one observation.** Helm answers what was asked.
It may add a single observation, subject to two conditions: it actually went and looked, and the observation is flagged as its own inference.
No second and third observation, no throat-clearing openers.

> Check `test` failed in slot 3. `pnpm test` exited 1 on 4 assertions.

> I checked the last three Jobs in `api`. That suite has now failed on the same assertion twice. The test may be the problem, not the agent.

**Hedge by source.** Helm and Judge are the only things in Armada permitted to state a hypothesised cause. Fleet never guesses in its own voice. When Helm speculates, it says so.

**Not linted.** Helm replies are governed by prompt only, not by the copy lint. Why: a lint in a real-time conversational loop is visible lag. See `../contracts/agent-copy.md`.

## Configuration

The following settings (see `../contracts/configuration.md`) directly affect this concept:

| Setting | Scope |
| --- | --- |
| Helm action authority (acting enabled vs. read-only) | Machine |
| Helm budget soft-warning threshold | Machine |
| Voice/tone | Machine — may adjust length and formality, never override Voice & conduct |

## Open questions

- **[helm-target-per-window]** Should each Bridge window keep its own Helm target, or does the most recent explicit act in any window set it for all?
  Today there is one conversation target for the whole Bridge: a pick or a "Discuss with Helm" in one window moves Helm in every open window, because since #1022 picks are per window while `HelmConnection` in `apps/desktop/src/main/helm.ts` holds one target.
  Two windows open at once is rare, and the rule as built is consistent with "the most recent explicit act wins".
  A per-window target would need `HelmConnection` to hold one aim per window in main, and the dock's switch and chip to read their own window's target.
