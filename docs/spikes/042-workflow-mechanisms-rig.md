# A rig for comparing how a workflow reaches a real agent

Status: built, baseline run once. Nothing here lands; it is a spike. Branch `spike/wf-rig`.

The question behind it: several spikes will each change how a workflow reaches the agent (steps seeded into its task list, hook refusals, the agent's own plan as the workflow, one agent per step) and need to be compared on the same small real Job. This page is the rig they share, what it records, the checks that keep it away from the owner's machine, and what it cannot do. The numbers from the first run are in *Baseline*.

## How to run it

Two commands, from any slot that holds this branch. The scratch directory has to be outside the repository.

```bash
docs/spikes/042-rig/fleet.sh "$SCRATCH"       # once, with run_in_background; it ends by exec-ing `armada serve`
docs/spikes/042-rig/run.sh baseline            # one fresh Job, one run
```

`SCRATCH` defaults inside `run.sh` to this Job's own scratch directory; pass another as the second argument or set `RIG_SCRATCH`. The Fleet's port is in `$SCRATCH/home/user/Library/Application Support/Armada/fleet.json`.

| File | What it is |
|---|---|
| `fleet.sh` | Builds the scratch home, the clone, its bare origin and a one-check Manifest, copies the mod, then serves. Safe to start twice: it says so when a Fleet is already up |
| `run.sh <variant>` | Starts a fresh Job with the brief, plays the person until the Job is done, quiet or past 40 minutes, then writes the verdict, a copy of the transcript and the timeline |
| `variants/<variant>.sh` | What differs between variants. `variant_prepare`, `variant_start_extra`, `variant_after_start`. Only `baseline.sh` exists |
| `person.py` | The person's acts, against the Fleet's own routes |
| `timeline.py` | The merged table |
| `yardstick.md` | The Job: its brief, the right answer, the two runner-ups |
| `seed-node-modules.sh` | The clone's `bootstrap` Command: copy-on-write copies of `node_modules` so vitest runs with no install |

Output of a run is `$SCRATCH/runs/<variant>-<utc time>/`: `brief.txt`, `session_id`, `acts.jsonl`, `verdict.txt`, `timeline.md`, `stopped_because`, and `transcripts/` (the agent's JSONL, this run's only). Transcripts stay in the scratch directory; the repository holds the timeline.

To add a variant, copy `variants/baseline.sh`. A variant that changes how steps reach the agent edits the mod under test by pointing `RIG_TREE` at its own slot when starting `fleet.sh` (the mod copy is `scripts/sync-mod` of that tree), or edits the clone in `variant_prepare`. `settings_set` in `baseline.sh` writes a key of the scratch Fleet's `settings.json`.

## How the agent is wired to the dev Fleet

```
run.sh ── POST /sessions/start ──────────────▶ dev Fleet (scratch HOME, own store)
       ── POST /sessions/message (the brief) ─▶   │ spawns, as Bridge's Jobs are spawned:
                                                  ▼
                       armada session-keep ── ARMADA_AGENT_BINARY = bin/agent ── claude -p --model sonnet
                                                  │   stream-json, --mcp-config the Fleet's door,
                                                  │   --settings (PreToolUse gate hook to /sessions/gate),
                                                  │   --plugin-dir <scratch home>/…/Armada/mod/armada,
                                                  │   --permission-prompt-tool, env ARMADA_SESSION, HOME=scratch
                                                  ▼
            the armada mod's hooks read $HOME/Library/Application Support/Armada/fleet.json and POST
            /sessions/report to the dev Fleet; record_plan, waiting_for, show_window go through the door
```

The point is that nothing is faked. `ARMADA_AGENT_BINARY` is the program Fleet launches as the agent for a hosted Job. `scripts/dev-fleet` sets it to a program that exits at once; `fleet.sh` sets it to a one-line wrapper that `exec`s the installed `claude`. So Fleet itself hosts the process, with the same arguments, environment, door, gate hook and mod it gives a Job started from Bridge. Starting the Job is the route Bridge uses, `POST /sessions/start`, then the first message, `POST /sessions/message`. The agent's own task list, plan and tool facts reach the dev store through the mod and the door exactly as they do for the owner's Jobs.

What had to be arranged for that to hold:

| Need | How |
|---|---|
| The mod reaches the dev Fleet and not the owner's | `HOME` of Fleet, and so of the agent, is `$SCRATCH/home/user`. The mod finds Fleet by reading `$HOME/Library/Application Support/Armada/fleet.json`. Fleet's own copy of the mod is made with `scripts/sync-mod <tree>` under that home |
| `claude` is signed in under a scratch `HOME` | It is not by default (the sign-in is in the login keychain, which is found through `HOME`). The scratch home has a symlink `Library/Keychains` to the real one. Nothing else of the real home is visible |
| A permission mode | Fleet starts the agent in `default` mode with `--permission-prompt-tool`, so anything not allowed is put to the person. The scratch `~/.claude/settings.json` allows reads, edits, the task tools, `git`, the read-only shell tools, vitest and `armada`, and denies `gh`, `bd`, `curl`, `wget`, `ssh`, `sudo`, `cargo`, installs, the web tools and every claude.ai connector. Anything else becomes a permission item that `person.py` allows unless the text names one of `gh bd curl wget ssh sudo`, `rm -rf`, `--force`, a push to main or a path under the real home |
| The agent stays in the clone | Fleet's write gate. A hosted Job writes only in a slot it leased in the clone; its first write leases one and the hook refuses that write once, as for any Job. Reads are not fenced, so the clone is a boundary of write, not of sight |
| Workspace trust | The scratch `~/.claude.json` marks the clone trusted, so the repository's own `.claude/settings.json` and hooks (`guard_write.py`, `guard_merge.py`, `guard_test.py`) apply as they do for a real Job |
| Fleet's own start-up work | The clone's Manifest is one file: one Check, one Command (`bootstrap`), `connectors: false`, no tracker and no `setup.seed`, so Fleet never runs a build, a browser install or a code-graph index |

## The person's acts

`person.py watch` polls `GET /sessions/one?session_id=` every three seconds and answers each item in the Job's `waiting_for`. Every act is appended to `acts.jsonl` and shows on the timeline as source `person`.

| Item | Route | Body |
|---|---|---|
| plan (`plan:<version>`) | `POST /sessions/plan/approve` | `session_id`, `version` |
| sketch | `POST /sessions/sketch/answer` | `session_id`, `sketch`, `drawn`, `how: approved` |
| document | `POST /sessions/document/answer` | `session_id`, `document`, `revision`, `how: approved` |
| question card, permission, any item with options | `POST /sessions/waiting/answer` | `session_id`, `item_id`, `choice` |
| an item the agent set with `waiting_for` | `POST /sessions/waiting/answer` | `choice` 0, or `text` "Go ahead with your best judgement." |
| delivery | `POST /sessions/waiting/answer` | `choice` 2, Keep local (no forge here) |
| walk | `POST /sessions/waiting/answer` | `choice` 0. Written from the route's notes and not yet exercised |
| a reply that asks | `POST /sessions/message` | "Go ahead with your best judgement.", once per reply |

It stops with `done` when the Job calls `all_done` or its row ends, `quiet` after seven minutes of nothing running and nothing waiting, and `timeout` at 40 minutes. Closing the Job (`POST /sessions/close`) is what `run.sh` does afterwards, which parks the slot without losing work.

## What a run records

`timeline.py` merges four sources into one markdown table, `time | source | event`, with the summary line underneath.

| Source | From | Shows |
|---|---|---|
| `agent` | the transcript JSONL under the scratch home | `TaskCreate`, `TaskUpdate`, `TodoWrite`, `Edit` and `Write` paths, Bash lines that commit, push, merge, test or call `armada`, every Armada MCP call (`record_plan`, `show_window`, `waiting_for`, `all_done`), the agent's replies, refused tool calls and hook errors, the messages Fleet and the person sent it. `--all-tools` adds the rest |
| `store` | the dev `armada.db`, read only | plan versions and approvals, `session_plan_tasks` marks, the agent's task list as Fleet saw it, notices to the agent (stop nudges, delivery choices), slot leases, the Fleet timeline events (steps, asks, turns), ledger changes, stops, `all_done` |
| `git` | the clone and its branches | commits made since the Job began |
| `person` | `acts.jsonl` | each act and the status Fleet answered with |

The summary line gives minutes, tool calls by name, how many tool rows Fleet kept, plan versions and approvals, plan task marks, agent tasks seen, step and gate events, notices and stops, the person's acts and the commits. A variant that adds a fact (a hook refusal, a seeded step) adds a `source` or an event class in `timeline.py`.

## The yardstick

`yardstick.md` holds the brief that is sent as the first message, the right answer, and the two runner-up picks. In short: armada-tyl, a red `scenario.test.ts` at the base commit because two scenario files both export a plain `TITLE`, a one or two file fix, a 4 second test, nothing to look at.

## Safety checks

Each was run, not assumed. All paths are under `$SCRATCH` unless said.

| The agent must not | Check | Result |
|---|---|---|
| Reach the owner's Fleet or store | `HOME` of the agent process read from `ps eww` on the live process | A path under `$SCRATCH/home/user`. The Job's rows and transcripts are in the scratch store and `~/.claude` of the scratch home |
| Push anywhere real | `git remote -v` in a leased slot, run by the agent | `origin` is `$SCRATCH/origin.git` for fetch and push. The clone's original `origin` was removed in `fleet.sh`, and `git config --get-regexp remote` holds no other URL |
| Reach GitHub with `gh` | Three layers. `gh` is denied by a permission rule; the first `PATH` entry is a shim that exits 1; and `HOME=<scratch> gh auth status` answers "not logged in", while the same command with the real home answers logged in | The agent's own `command -v gh` printed the shim path |
| Reach the owner's beads | `bd` is denied and shimmed the same way; the clone has no `.beads` and sits outside the owner's tree, so `bd where` has nothing to find. The agent's `bd where` was denied by the rule | Denied, not run |
| Use the owner's connectors | `connectors: false` in the clone's Manifest, plus deny rules on `mcp__claude_ai_*` and `mcp__plugin_productivity_*`. The first smoke Job showed the account's connectors (Notion, Gmail, Drive and others) attached before this was set | Fixed before any run that mattered |
| Read the owner's instructions | `HOME=<scratch> claude -p` asked for any text mentioning the owner's global headings | Answered NONE. One caveat: the mod's context breakdown names `/Users/<owner>/.claude/CLAUDE.md` as a memory file, which is a path the mod prints, not evidence that file was loaded |
| Generate load | No cargo, no workspace build, no `pnpm install`. The one Check is vitest on one file (about 5 s). `armada run bootstrap` copies `node_modules` with `cp -Rc` (copy on write, a few seconds) | Held |

Not fenced, and said plainly: reads outside the clone work, and `Bash(git *)` is allowed, so an agent that went looking could run git against another path. The rig relies on the agent not going looking, as the owner's own Jobs do, and on the write gate for writes.

## What the rig cannot do

| Missing | Stand-in |
|---|---|
| A pull request, `ci` and a merge | None are real. The brief ends at "commit and push the branch to `origin`" (a local bare repository). `run.sh` writes a verdict from the slot afterwards: the branch, uncommitted files, `git diff --stat origin/main HEAD`, whether `origin` has the branch, and the test run at the slot's tree. A variant about a pull request or merge gate has to stand in for it with a route or a hook of its own, and say so |
| `auto_merge`, review gates, the Judge's real step boundary | The scratch Manifest has no `steps`. A variant that needs them adds them to the clone's Manifest in `variant_prepare` |
| Bridge | Nothing draws the Job. Walks (`show_window`) arrive at the Fleet and wait; approval is the untested `walk` row above |
| The owner's workflows being off or on | The baseline turns `features.workflows` off in the scratch `settings.json`. With it on, the proposer reads the first message and picks the repository's `bug` workflow, tells the agent four steps and puts a delivery question to the person. That is the real mechanism the later spikes will compare against, and the first two attempts at the baseline hit it |
| The same mod as the owner's | The mod is the copy of this slot's `plugins/`, and the Fleet binary is a copy of the owner's newest kept build, so a mod and Fleet that disagree on a version only show as a version note |
| Repeatability | One agent run is one sample. The model, its effort (set to medium by the start so the proposer does not pick) and the brief are fixed; its path through the repository is not |

## Baseline

One run, `run.sh baseline`, full table in `042-rig/baseline-timeline.md`.

| Measure | Value |
|---|---|
| Wall time, first message to the Job ending | 0.9 minutes |
| Tool calls by the agent | 16: Bash 14, Edit 1, `all_done` 1 |
| Result | One commit pushed to `origin`, one file changed (`scenario.test.ts`, 5 lines added and 1 removed), the whole file green at the pushed commit (9 tests). It named the cause and took the first of the two right fixes in `yardstick.md` |
| Did it record a plan | No. The model judged a one-file fix too small, and nothing in the baseline asks for one |
| Did it make tasks | No `TaskCreate` or `TodoWrite`. Fleet's `session_build_tasks` and `session_plan_tasks` are empty |
| What Fleet's store saw | All 16 tool rows, the slot lease and release, the edit (`session.edited`), `all_done` and the end. No plan version, no task marks, no notices, no stops, no step or gate events |
| What the person did | Nothing. `acts.jsonl` was never written |
| The hook's one refusal | The first Bash line that writes (`armada run bootstrap`, by the repository's write gate) was refused once while Fleet leased a slot and moved the Job into it, 18 seconds in. The agent retried as told |
| Cost as Fleet measured it | 16,958 micros (about $0.017), 55k tokens of context |

So the baseline is a short, clean run where the agent works from the brief alone and Fleet's store learns only what the hooks see. Any variant that wants a plan, tasks or step facts has to add them, and the comparison on this Job is mostly about whether they appear and what they cost, since the work itself took under a minute.

### The person's acts, checked on their own

The baseline asked for nothing, so `check-acts.sh` ran a throwaway Job told to record a one-task plan, ask one question and call `all_done`, editing nothing. The plan was recorded as version 1, `person.py` approved it through `POST /sessions/plan/approve` (200) within a second, Fleet sent the agent the `plan_approved` notice, the agent asked "Which colour?" through the ask tool, `person.py` answered it through `POST /sessions/waiting/answer` (200) and the Job called `all_done`. Delivery was answered the same way in the two aborted baseline starts. Sketch, document and walk answers are written from the routes' notes and have not run against a live item.

### What went wrong on the way

| What | Cause | Fix |
|---|---|---|
| The first two baseline starts followed the `bug` workflow and put a delivery question to the person | `features.workflows` is on by default; the proposer reads the first message. The first fix wrote `settings.json` in a nested shape | `settings.json` is one flat object of dotted keys. `baseline.sh` writes `"features.workflows": false` and waits five seconds, since Fleet reads the file after it sits still for 1.5 s |
| A smoke Job had the owner's claude.ai connectors attached | `connectors: false` was missing from the one-file Manifest | Added, with deny rules as a second layer |
| `armada run bootstrap` was needed in every slot | A hosted Job's slot is cut without running `setup.requires` (Drones get that) | The brief says to run it once; it is the same for every variant |
| The first verdict read nothing | The Job's `cwd` moves into its slot, and `all_done` gives the slot back | `verdict.sh` reads the scratch origin and checks the pushed commit in a throwaway clone |
| The guard hook refused a `git push` to `main` typed in this slot's shell | The repository's `guard_merge.py` reads the text of the command | `fleet.sh` pushes inside a script, which the hook does not read. It is a hook on this Job's own commands and has no effect on the Fleet under test |

The Fleet binary is a copy of the owner's newest kept build (`c185a65`), not one built from this tree, because building the workspace is not allowed here. The mod is this tree's `plugins/`.
