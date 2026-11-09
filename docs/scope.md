# Scope

**What it is:** What Armada is for, in the owner's words, and what it is not. Read before proposing work.

---

**Kind:** Reference.

Armada exists because one person was running five coding agent jobs at once
and could not stop watching them. Everything here is downstream of that. A
proposal that does not serve it is a rabbit hole, however good it is on its own
terms — **and that has already cost a night's work**, so the rule is written
down rather than assumed.

## What it is: a workflow system

The owner's own description, and the shape everything is measured against:

1. He provides a job or task that needs doing.
2. The system determines the type of workflow needed.
3. It creates a workspace to do the work in.
4. Agents do the steps, and the hard parts — coding, building artifacts.
5. **The system does the deterministic parts**: running tests and checks, and
   establishing that the result is what was asked for.
6. **A Judge verifies the work before he needs to review it**, and kicks back
   anything that seems off.
7. When the work is complete he has a set of work he can review, in a pull
   request or a document.

> Anything outside of that is bonus.

## The core is Fleet

**Said 2026-11-05.** In his words:

> I think the core of what we have built here is fleet, and if fleet does what it should we have a powerhouse that sits on your machine that you can potentially build any bridge over.
> You could even integrate it anywhere you want.

> So all of the cool things that I like about it can just be mods that you can install.
> Like the battlefield and everything could just be mods that are not maintained in the core repo.

**Trust comes before speed, and reliability problems live in Fleet.**

### What the core Bridge delivers

| Piece | In his words |
|---|---|
| Projects | "A workspace that lets you select not just repos but specific workspaces in a repo as part of that project. You can customize and tweak any of the settings for that specific project like how fleet, operator, jobs and even the UI behave. Each project can have its own issue tracker, PR reviews and task system." |
| Operator | "A second brain to help you triage everything from jobs and what is going on in your head" |
| Jobs | "The work you're actively doing." |

None of the first row is in the repository yet.
Today's [Project](concepts/project.md) is a task list across repositories.

The core Bridge is being rebuilt natively beside the Electron Bridge, decided 8 Nov 2026.
See [Native Bridge](concepts/native-bridge.md).

**A project's settings override Fleet's defaults key by key.**
A project sets only what it changes, and Fleet's default fills the rest, as Kit and a Manifest work today.
A new default reaches every project, so Bridge has to show where each value came from.

### What the core keeps

- The Job layout he configures.
- The walk system.
  A walk arrives as a tab in the Job, and from the Job he can pop it out to a full window.
  In his words: "I still like the idea of a tab, but I should be able to pop it out when im in a job to a full window."
- Needs you, which becomes Needs you 2.0 with the Operator in it.
- The ledger, and how the Job layout reads it.
- The Job tools: the API Client, the Browser and the Terminal.
- Parts of the worktree, slot and bay system.
  Which parts is not settled.

### Everything else is a mod

**The Cockpit console, with its map and bays, is a mod, kept outside the core repository.**

**A mod outside the core is published to a registry, and installed and updated from it.**
In his words: "Someone can build a mod run it locally and then publish it somewhere that lets other install/update it".
Which registry is not chosen.

There are two kinds of mod.

| Kind | Changes | Today |
|---|---|---|
| Bridge mod | What he sees | Themes, layouts, Words, tweaks, Commands, tabs and instruments, in [Mod](concepts/mods.md) |
| Fleet mod | How Jobs and the agents in them run | Workflows are the first one: "i would imagine that workflows needs a way to mod fleet right? Thats a fundamental change to how jobs and the agents in them run". How one hooks in is not designed: **[fleet-mod-hook]** below |

Today's mod of kind `fleet` is a `fleet.toml` notice rule, which is far narrower than this.

Studio and its sketches wait on a rethink, `armada-x6w9`: "I think studio still can be a thing but it needs re-thought."
See [Studio](concepts/studio.md).

## The four pains it exists to remove

He was juggling five or more agent jobs and hit these, in these words:

| Pain | Answered by |
|---|---|
| *Which agents need me right now* | Needs you: every Job waiting on a person, called out. Needs you 2.0 puts the Operator's triage in it |
| *Oh, that agent didn't make a worktree* | Fleet leases the slot a Job writes in. An agent never decides where it works |
| *This agent is stuck because it doesn't know how to run checks properly* | `armada check` runs the repository's own Checks from its Manifest, so an agent calls one command and does not work out the runner |
| *This one got ahead of itself and did all of the work in one PR instead of three* | A Job's plan states its scope, and a workflow's steps show where it is. **Partly solved** — nothing blocks a Job, and no judge runs on its own |

**Armada is the orchestrator above agents.** It babysits them, and surfaces a
review only when it is confident the work is ready to be looked at.

## What a milestone may claim

**A milestone is one thing a person can complete, including its Bridge half.**
There is a moment where it is finished and they can do something they could not
do before.

A milestone naming a part of the system rather than an outcome absorbs work
forever without closing. Surface named the surfaces, and "everything I need to
know is in the app" is a standing bar rather than a finish line — every future
screen either meets it or does not. Its items each turned out to belong to an
event somewhere else, and what was left was never a milestone. It was retired.

**A milestone that ships an act with no surface to invoke it from has not
shipped the act.** It has shipped a route, and left a person with the API. The
half that is easy to file is the half that does nothing alone.

The cost of cutting this way is real: a milestone drawn around one person's
event touches more of the codebase at once than one confined to a crate, and is
harder to fan out across agents. That is the reason the earlier cut existed. It
is worth paying, because the alternative optimises for how work is dispatched
rather than for whether anything is finished.

## Four attempts got here

Each one was abandoned for a reason, and each reason is now a constraint. A
proposal that reintroduces one of them is answering a question already settled.

| Attempt | Why it was abandoned | What it became |
|---|---|---|
| Skills in a repository | Worked, but was not portable to other projects | The Manifest. A repository has its own setup, kept at the place a person chooses, and one Fleet serves every repository a person adds rather than being configured per project |
| A CLI | Did not surface the information he needed | Bridge, and why it is a board that is scanned rather than output that is read |
| Orchestrator agents with sub agents | **Having a conversation was not the tool he was looking for** | Dispatch and report is how work gets done, and reading a transcript is not. What a conversation produces is kept as structure — a Studio's nodes, a Job's plan, a Job's branch — never a transcript read as a claim. The [Operator](concepts/operator.md) is no conversation with tools: it is one call per arrival that holds none, and what it leaves is a Log of rulings |
| Armada v1 | Close | This |

**The third one explains more of the design than it looks like.** Evidence is
structured because prose is not a claim. Fleet decides because a self-report is a
signal. Every time a design reaches for "ask the agent", it is reaching for the
attempt that was already rejected.

## What Armada is not

**It is not a sandbox, and confinement is not the point.** An unattended agent's
launch bounds what it can reach. That is a floor and is deliberately not a fence:
`crates/adapters/src/harness.rs` records the measurement — `--allowedTools` is a
permission allowlist rather than a toolset, and it removed none of the built-in
tools in three spike runs. An agent launched this way can run a shell.

Real confinement means containers with nothing mounted, and that is a different
system that can be added later. **Time spent hardening the current arrangement
is time not spent on the seven steps above.**

The one confinement that does earn its place is `--strict-mcp-config`, because
without it an unattended agent comes up holding every MCP server the operator has connected
— measured at seven servers, ninety-five tools, personal accounts. That is not
hypothetical tightening; it is the v1 defect that made an unattended agent unusable.

The flag stays, and Kit does not weaken it: `#1275` lets a person connect an MCP server from Bridge, and a server reaches an unattended agent only where that person turned it on for Kit or for the Manifest.
It still comes up holding what it was given.
See [Kit](concepts/kit.md).

**It is an unattended agent's and not a Job's.** A Scout is unattended, and the flag is what
stops one reaching an operator's accounts with nobody watching. A person is that
operator, in a Job of their own, reading the reply as it is written, so a hosted
Job comes up holding what they hold, measured at nineteen servers on this
machine in
[spike 18](spikes/018-what-can-a-helm-session-do-in-each-permission-mode.md).
The two launches are rendered apart and a test holds them apart.

**It is not built for every kind of repository.** What Armada brings — its
carried workflows and the Checks a Job runs — assumes code with tests: a Check
a shell can run in a worktree, and a diff a Judge can read. A repository whose
correctness shows up another way needs evidence of its own kind, and Armada does
not bring it.

| Shape | Where its correctness shows up instead |
|---|---|
| A game | A scene or an asset behaving in an engine, not a readable diff |
| Firmware | A toolchain outside the tree, and hardware |
| Notebooks and data | Outputs and cells a text diff renders as JSON |
| Infrastructure as code | Only after `apply`, against real state |
| Prose | There is no behaviour to test |

**In such a repository the Checks can have nothing to run.** An empty Check registry expands
to nothing in `crates/config/src/resolve.rs`, and #847 is what tells a person so.

**A conversation is not a record.** The Operator runs no Job, and in conversation it holds only the read tools, Read, Grep and Glob, so it does no work in conversation.
A Job started on your words holds what a terminal job holds, writing in a slot on a branch like any Job — and what persists is the Studio's typed graph, the branch, the Jobs it started and the Operator's Log.
Nothing downstream reads the thread.
See the third attempt: what was rejected was reading a transcript as a claim, not talking.

**It is not about throughput.** Running more agents was never the problem.

## Before proposing work

Ask which of the seven steps it serves, or which of the four pains it removes.
If the answer is neither, say so and ask before building it — the owner defers
work; an agent does not defer it on his behalf, and does not start it on its own
judgement either.

Ask too whether it is core or a mod.
Where Armada is unreliable, the fix is in Fleet.
A surface outside the core is a mod, unless the core collaborators agree to bring it in.
Today that is the owner.

## Open questions

- **[fleet-mod-hook]** How a Fleet mod hooks into how a Job and its agents run.
  Workflows are the first case, and today they live in Fleet: a soft rail in the agent's prompt, steps Fleet detects, nudges when one is skipped.
  Shapes on the table: a declared file Fleet reads at fixed moments in a Job, as `fleet.toml` and Triggers are; a program Fleet calls at those moments with a cap, as tracker mods are; a long-lived process Fleet talks to over a stable protocol, as a Job talks to an MCP server; a WebAssembly module Fleet loads in a sandbox; or code linked into Fleet.
  He said it is not designed yet, and asked for ideas: what another tool's plugins cannot do is no limit on Armada's.
  On 5 Nov he chose the direction: one protocol.
  A Fleet mod is a long-lived process Fleet talks to over a stable protocol, and a built-in mod reads simple file mods and speaks it for them, so Fleet knows one hook.
  The protocol itself, its moments and what Fleet may ask a mod are what is still open.
