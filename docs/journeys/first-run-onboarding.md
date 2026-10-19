# First-run onboarding

**What it is:** "I'm brand new to the whole app": opening Armada for the first time and getting to a first Job on a folder.

Design fidelity: the start menu is drawn as mock walks. Analysis: Complete. UI/UX design: In progress.

---


**Trigger:** First launch after install, with Fleet serving no repository.

**Concepts touched:** Cockpit, Job, Manifest, Kit.

**Milestone:** Ship.
Nothing in this journey is gated: any folder starts a Job, and Setup, Kit and Machine setup and Doctor are not steps in front of it.
The Fleet side of each start is built; Bridge's start menu is not.

**What was kept and why:** the findings under *Kit and Machine setup* are reasoning about a flow, not renderings of it, and stay recorded as open design questions.
The five contract corrections stay because they record drift that was caught, and deleting them means the same drift returns unrecognized.

## Flow

| Step | You | Armada |
| --- | --- | --- |
| Start | Pick Open a project, New project or Import from Claude Code | The start menu over the live battlefield; [Cockpit](../concepts/cockpit.md), *With no repository*, has the three starts |
| Folder | Choose a folder, name a new project, or tick the folders Claude Code kept sessions for | Adds each folder; makes and `git init`s a new one; learns how each imported folder runs |
| First Job | Type in the box | The Cockpit with a new Job open on the folder, its box focused |
| Armada configuration | Nothing | A card at the top of the Job's thread names where Armada keeps how the repository runs, On this Mac picked |

**A folder with no Manifest takes a Job.** Fleet serves it under an implicit Manifest ([Fleet](../concepts/fleet.md), *Fleet starts with none*), and picking it opens its Cockpit.
Setup is opened from the repository when a person asks, as [Set Up a Project](set-up-a-project-manifest.md) describes.

**Armada configuration is how Armada remembers a repository runs.** What it keeps, the four places it is kept, and the card, the header glyph and the picker's row glyph are [Kit](../concepts/kit.md), *What a repository runs*.
Later Jobs on the repository show only the place's glyph in the Job's header.

**Not built in Bridge:** the start menu, the card and the header glyph.
Bridge on a fresh install opens the Add a repository dialog by itself, and a picked folder opens its Cockpit.

## Kit and Machine setup

The flow above does not walk Kit setup or Machine setup first.
What the setup is called, whether it is one step or two, and where first run offers it is open: see Open questions.

> **Finding — open question, not a settled design.** Kit setup probably should not open as a guided tour through four groups in fixed order.
> It likely **forks at entry**, because the two populations arriving here need opposite things.
> The reasoning below survives the discarded prototype; the surfaces do not exist.

| Path | What the user does | Why |
| --- | --- | --- |
| **Import from another machine** | Asked only where the existing Kit lives. Armada performs the import. Done. | Someone who already has a Kit does not need to be walked through settings they have already decided. The import is the whole step. |
| **Start fresh** | All four settings groups available, defaults already filled in, freely reachable in any order. | Defaults are good enough to proceed. Nothing gated inside the step, so the user can inspect what interests them and skip the rest rather than being marched through four screens. |

### The four settings groups

Content-level notes only. **No surface design exists for any of these.**

| Group | Note |
| --- | --- |
| **AI Behavior** | **Finding — open:** this group should probably show the **actual Claude files found on the machine** — Agent files, Skills, MCP servers, Sub agents, Plugins, Models — rather than abstract settings, with each file carrying its state relative to Kit (in Kit / drifted / not in Kit). How that is presented is undesigned. |
| **Resources & Budget** | Defaults filled in. Unchanged from the original design. |
| **Safety** | Command allowlist and Destructive-op list. **Finding — open:** likely inline editors rather than links out. |
| **Interface & Notifications** | Defaults filled in. Unchanged from the original design. |

**Moved out of onboarding:** Sync & Portability becomes its own top-level nav group, holding config-repo and pull-on-startup controls.
It is a machine-lifecycle concern, not a first-run one.
This one is a decision, not an open finding.

## Choosing a folder

Open a project asks for the folder at once, and Locate adds one afterwards; [Set Up a Project](set-up-a-project-manifest.md) owns Locate.

**Rejected:** a "recently opened" list of repo paths. At first run there is nothing recent, and inventing plausible-looking entries only makes the surface look busier without helping anyone.

## Contract corrections — drift to avoid

The discarded prototype drifted from the design contract in five ways. **Recorded here so the same drift is not reintroduced by the next attempt** — this table is the most durable output of that pass.

| Drift | Correct behaviour | Rule |
| --- | --- | --- |
| Primary CTAs scaled up (larger type, taller, heavier padding) to signal importance | `--accent` fill at the standard 32px control height, `--text-sm` weight 500, one per view | Button spec + 4px grid. Emphasis is fill, not size |
| Primary buttons carried leading icons (magnifier on Scan, download on Import) | Label only | Iconography → Actions: text buttons carry no icon |
| Badges read `IN GUILD` / `DRIFTED` / `NOT IN GUILD` | Sentence case | ALL CAPS is legal only in `--text-2xs` table headers at 0.04em |
| Field labelled "Where is your project?" | A noun label, e.g. "Project location" | No Wh- sentence openers; they survive only as panel headings |
| Surfaces styled on the Nocturne palette (blurple accent, outlined primary, 7px radii) | Armada tokens: `--bg-*` slate ground, `--accent` #4A9EDB filled primary, `--radius-md` 5px | Hard rule 1. No value that is not a token |

## Still to reconcile

- **Kit-file state labels** (in Kit / drifted / not in Kit) are not statuses, so they sit outside the badge table on Iconography.
  Under that page's rule for anything unlisted, the default is no icon — see Open questions for the specific question this raises, tracked in the Iconography contract rather than duplicated here.

## Doctor

[Doctor](../concepts/doctor.md) holds a first-run gate that blocks dispatch until Doctor reports green.
This flow has no such gate, and the two pages disagree: whether the gate stands is not decided here.

## What is already decided and landed

- **What the Fleet panel says for each of Fleet's runtime states, outside onboarding.** Three states, each named out loud with a mono clause carrying the fact that identifies it — running, not running, unreachable — decided in full and recorded on the design system contract's Component → token mapping, Fleet panel.
  What the panel reads specifically during first run, before Fleet is reachable, is a separate, still-open question — see Open questions.
- **The install and distribution strategy for the Fleet binary.** Bundled with Bridge, adopted by Fleet at a safe moment: one artefact, one signing and notarisation pipeline, no second updater.
  The running daemon restarts into the newly shipped binary when no Jobs are active, or on the engineer's explicit instruction.
  This does not remove version skew, it moves it from Bridge-version-versus-Fleet-version to installed-Fleet-versus-running-Fleet, which is exactly what the protocol handshake and its fallback window exist to cover.
  Left to decide, and smaller: what happens if a safe moment never arrives because Jobs are always running — either Fleet nags, or it adopts at some bound anyway.

## Open questions

- **[guild-init-fork-at-entry]** Should Kit setup fork at entry between an import path and a start-fresh path?
  Recorded under Kit and Machine setup above as a Finding. The two populations arriving at first run need opposite things — someone with an existing Kit needs only to be asked where it lives, while someone starting fresh needs the full four-group walkthrough with defaults pre-filled. This is reasoning that survived the discarded prototype; no surface exists for it.

- **[ai-behavior-group-shows-kit-files]** Does the AI Behavior settings group show the actual Claude files found on the machine, rather than abstract settings?
  Recorded under "The four settings groups" above. Each file would carry its state relative to Kit — in Kit, drifted, or not in Kit. How that is presented is undesigned.

- **[safety-group-inline-editors]** Are the command allowlist and destructive-op list edited inline during onboarding, or linked out to elsewhere?
  Recorded under "The four settings groups" above.

Four further open items bear on this journey and are recorded, not duplicated, where each is the citable home.
`kit-machine-setup-surface-naming` and `kit-import-missing-files`, both above on Guild Setup & Configuration, cover respectively what the Kit and Machine setup surfaces are called and whether first-run setup is one journey and one step, or two, and what happens on the Import path when the imported Kit references Claude files that do not exist on this machine.
`status-bar-onboarding`, in `../contracts/design-system.md`, covers whether the Fleet panel reads the same three runtime states during first run as everywhere else, or gets its own reading before Fleet is reachable — the slug predates the panel and stays as written, since a citation resolves by name.
`kit-file-icons`, in `../contracts/iconography.md`, covers whether the three Kit-file states (in Kit / drifted / not in Kit) get icons, or stay label-only.

## Related

Guild Setup & Configuration · Set Up a Project (Manifest) · Check System Health · [Job](../concepts/session.md) · [Cockpit](../concepts/cockpit.md).
