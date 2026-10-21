# Armada's concepts

One document per concept, carried out of the design workspace. Each
opens with a one-sentence **What it is** and then states the concept in full —
its shape, its rules, and what is still undecided about it, filed under that
document's own `## Open questions`.

**These are reference for what the product is, not a build order.** A
milestone or a capability decides what gets built; these documents answer a
different question: *this is what the thing means, and what governs it.*
Where two concepts touch, each restates only what it owns and links to the
other rather than repeating it — read the one that is missing rather than
assuming it is restated here.

| Concept | What it is |
|---|---|
| [account.md](account.md) | The GitHub account a machine signs in to, the profiles it joins, and what each profile syncs |
| [bridge.md](bridge.md) | The Electron command-center shell — Armada's only engineer-facing surface |
| [cockpit.md](cockpit.md) | Bridge's first tab: the flight deck, with every Job, the calls waiting on a person, the worktree slots and the merge queue on one screen |
| [doctor.md](doctor.md) | The health-check surface — a passive per-module grid that gates nothing |
| [autopilot.md](autopilot.md) | A repository working its own backlog: one instruction, caps, and every issue it picked, started or set aside |
| [file-panel.md](file-panel.md) | Any file Bridge opens, in the one side panel in a code editor: where it may come from, how it saves, what stops a save |
| [fleet.md](fleet.md) | The Rust daemon that hosts Jobs, leases worktree slots, keeps the merge line and reads the forge |
| [helm.md](helm.md) | The conversational orchestrator agent that reasons across a Fleet |
| [job-proposer.md](job-proposer.md) | The Proposer: the model call that reads a Job's request and proposes its title, workflow or none, model and effort |
| [judge.md](judge.md) | The semantic, veto-only tier of verification, run on the owner's press |
| [key-bindings.md](key-bindings.md) | The keys each act answers as this person has them: the registry's, with Settings → Keyboard shortcuts laid over them and kept by Fleet |
| [kit.md](kit.md) | The tool set you bring — Skills, MCP, sub agents, Commands, the allowlist |
| [layout-mods.md](layout-mods.md) | A mod whose `layout.json` reorders, hides and sets the opening choice of Bridge's Cockpit panels and places, by id |
| [log-envelope.md](log-envelope.md) | The field contract every log line carries across Fleet and Bridge |
| [machine.md](machine.md) | How this installation behaves: resources, timing, notification routing |
| [manifest.md](manifest.md) | Per-project config, backed by `armada.yml` |
| [mods.md](mods.md) | A folder on this machine that changes how Bridge looks, which Fleet reads and checks and which can never stop Fleet |
| [pocket.md](pocket.md) | The owner's phone app and the Phone Gateway in front of Fleet: what it shows, what it pushes, what keeps it closed |
| [pull-review.md](pull-review.md) | Bridge's Reviews tab: the pull requests waiting on a person, a page for each, and the review he writes on one |
| [requests.md](requests.md) | An HTTP request sent through Fleet by the person or a Job's agent, from collections of saved requests and environments whose secrets are kept in the Keychain and never read back |
| [retro.md](retro.md) | What got in the way while one Job ran, and whose way, written on a press |
| [scout.md](scout.md) | A read-only agent a person starts from a Studio, which comes back as a Finding |
| [session.md](session.md) | An agent job a person runs, kept as a row so Fleet can say which jobs are open and which one holds a branch, a pull request or a slot |
| [sketch-pane.md](sketch-pane.md) | The scene an agent draws with `show_sketch` and the owner's pad shares, and how a sketch is read, played and marked |
| [studio.md](studio.md) | A typed graph of what one stretch of work produced, kept per repository |
| [themes.md](themes.md) | The colours Bridge is drawn in: Dark, Light, a catalogue of terminal colour schemes, and the themes mods add |
| [tracker.md](tracker.md) | The adapter through which Fleet lists a repository's open issues |
| [trigger.md](trigger.md) | A saved Command, skill or prompt tied to a moment in a workflow's steps, set by a repository or this machine |
| [workflow.md](workflow.md) | The template a Job may follow: ordered steps as a soft rail |
