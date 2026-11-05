# What is Armada's core, and what is a mod?

**Said 2026-11-05.** His words:

> I think the core of what we have built here is fleet, and if fleet does what it should we have a powerhouse that sits on your machine that you can potentially build any bridge over.
> You could even integrate it anywhere you want.

> So all of the cool things that I like about it can just be mods that you can install.
> Like the battlefield and everything could just be mods that are not maintained in the core repo.

Trust comes before speed, and reliability problems live in Fleet.
`docs/scope.md`, *The core is Fleet*, carries the rule; this file keeps what he said and chose.
He approved the page "Armada's core is Fleet", revision 24, on 5 Nov.

## What the core is

| Piece | He said |
|---|---|
| Projects | "A workspace that lets you select not just repos but specific workspaces in a repo as part of that project. You can customize and tweak any of the settings for that specific project like how fleet, operator, jobs and even the UI behave. Each project can have its own issue tracker, PR reviews and task system." None of it is in the repository yet |
| Operator | "A second brain to help you triage everything from jobs and what is going on in your head" |
| Jobs | "The work you're actively doing." |

He keeps the Job layout he configures, the walk system, Needs you (to become Needs you 2.0 with the Operator in it), the ledger and how the Job layout reads it, the Job tools (API Client, Browser, Terminal) and parts of the worktree, slot and bay system.

The Cockpit console, the Pixi map with its bays, was "an amazing experiment" and becomes a mod outside the core repository.

## What he chose

| Asked | He chose | The cost he took |
|---|---|---|
| How a project's settings meet Fleet's defaults | Override key by key, as Kit and a Manifest do | A new default reaches every project, so Bridge has to show where each value came from |
| Where mods outside the core live | A package registry: "Someone can build a mod run it locally and then publish it somewhere that lets other install/update it" | A publish step for each release. Which registry is not chosen |
| How a walk is shown | A tab in the Job he can pop out: "I still like the idea of a tab, but I should be able to pop it out when im in a job to a full window." | The pop-out is not designed |
| How a Fleet mod hooks in | One protocol: a Fleet mod is a long-lived process Fleet talks to, and a built-in mod reads simple file mods and speaks it for them, so Fleet knows one hook | One more process per program mod to keep alive. The protocol itself is open, `fleet-mod-hook` in `docs/scope.md` |

There are two kinds of mod: a Bridge mod changes what he sees, and a Fleet mod changes how Jobs and their agents run.
On workflows: "i would imagine that workflows needs a way to mod fleet right? Thats a fundamental change to how jobs and the agents in them run".
Today's mod of kind `fleet`, a `fleet.toml` notice rule, is far narrower than this.

An adversarial review found one protocol holds with three changes: a stated default when a mod does not answer in time, pinned program mods he installs himself, and checks staying inside Fleet. It also called the full version premature beside the file reader plus one moment. The review is on the approved page. He did not adopt its changes; they wait for when the protocol is designed.

## Not placed yet

Pocket; workflows, as a Fleet mod or deleted; Studio, which "still can be a thing but it needs re-thought" (`armada-x6w9`); and sketches.
Also open: which parts of the slot and bay system stay.

## What this supersedes

- **The Cockpit as the app.** No decision file recorded it. `docs/scope.md` said it and changes with this file; `docs/concepts/cockpit.md` and `docs/concepts/bridge.md` change with the work that moves the Cockpit out.
- **`2026-10-15-sessions-absorb-jobs.md`, in part.** The Sessions page went because "We have everything in the cockpit". Jobs stay, and their home is the core Bridge.
- **`2026-10-22-a-jobs-field-is-the-whole-repository.md` and `2026-10-14-a-session-builds-a-ring.md`, in where they live.** The Field and the build ring are drawn on the battlefield, so they go with the console into a mod.

Left standing: the bay grid (`2026-10-04`), the Studio decisions of 28 Sep to 2 Oct, which wait on `armada-x6w9`, and the Trigger decision of 7 Oct, which waits on workflows.
`2026-11-05-how-he-wants-armada-to-work.md` stands; its concept with no name yet is the Project here.
