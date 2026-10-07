# Fleet

**What it is:** The Rust daemon that schedules, gates and verifies Jobs, and the only actor that writes a state transition on either a Job or a Drone. Everything else reports; Fleet decides. launchd-supervised, api in-process, no engineer-facing surface of its own.

---

**Kind:** Process.

Formalizes Fleet — the Rust daemon that manages the set of Drones working on Jobs. Companion to the main Armada brief.

## What it is

The actual execution engine underlying everything else in Armada. Fleet schedules, gates, verifies, and drives every Job and Drone state transition (see [Job](job.md) — Ownership Split, [Drone](drone.md) — Core Principle).

Fleet has **no engineer-facing surface** beyond Doctor's module status. It is pure infrastructure; you never "go to Fleet" the way you go to Job Board or Helm. Its behaviour is specified across nearly every other concept document — what this document owns is the daemon lifecycle, and a map of where the rest lives.

## Daemon lifecycle

Every number below was measured on macOS 27.0 / 26A5406e, launchd 7.0.0.

**Fleet and the API are one process.** `api` runs in-process with `fleet`; there is no second daemon. Doctor stays readable when Fleet dies through `armada doctor --json`, a short-lived probe process Bridge spawns.

- **Startup.** Bridge bootstraps a **launchd job**, plist kept at `~/Library/Application Support/Armada/` — deliberately **outside** `~/Library/LaunchAgents`. launchd only auto-loads from the LaunchAgents directories, so a plist elsewhere is never seen at login: **"does not start at boot" becomes structural rather than configured off**. `KeepAlive={SuccessfulExit:false}`, `ThrottleInterval` 2, mode 644. launchd parents Fleet to PID 1 at spawn.
- **Quitting the app.** Does **not** kill Fleet. Jobs keep progressing, independent of Bridge, because Fleet is not Electron's descendant.
- **Reopening Bridge.** **Reconnects** to the running Fleet rather than spawning a duplicate. Bootstrapping is per-login-session, so Bridge bootstraps every login, idempotently — with a `launchctl print` pre-check, because bootstrapping an already-loaded job and bootstrapping a world-writable plist fail with the **identical** error, `Bootstrap failed: 5: Input/output error`.
- **Fleet crashes — signal or non-zero exit.** launchd restarts it automatically. Doctor's row flips fail → pass with no user action. **There is no cap and no backoff curve** — a flat `ThrottleInterval`, forever.
- **Fleet is wedged — alive, not answering.** `launchctl kickstart -k gui/$UID/com.armada.fleet`. **26 ms** to a new PID. This is what the "Restart Fleet" button does, and it means **skip the throttle wait**, not *recover*.
- **Fleet exits 0 deliberately.** launchd leaves it down by design. Kickstarting just makes it exit 0 again. Doctor must show the **reason**, not a restart button.
- **On any restart.** Reconciles SQLite job state against live OS processes, for every repository it serves. A Job marked running with no matching process is flagged `interrupted`, and Fleet then restarts its step, signed as Fleet's own act. A Drone that is still alive is adopted instead, and a Drone that is there and cannot be adopted is ended and then treated as a gone one, so its step is restarted unless a stop holds — all in [Drone](drone.md), with the `setsid` constraint below. Also sweeps worktrees for terminal Jobs past retention.
- **While it reconciles.** The listener serves first and reconciliation runs in a task of its own, since it can re-run a Job's whole gate from a pending-evidence row and took 6m47s doing so. Health and every read answer at once. A command waits until reconciliation finishes, so none lands between two of its Job moves, and the turn loop starts after it. A stop during it ends the task where it stands, as a crash would, and the next boot repairs what was left.
- **Uninstall.** Must `launchctl bootout`, not merely delete the plist. A loaded job survives deletion of its own plist — verified.

Two plist keys do not mean what they read. **`RunAtLoad=false` is a lie in the presence of `KeepAlive`** — both `true` and `{SuccessfulExit:false}` started the job the moment it was bootstrapped. **`Crashed:true` means signal-terminated, not failed** — `exit 1` left the job down, and a Rust panic exits 101.

### Two design constraints that follow, not preferences

**Fleet must `exit 0` on a permanent refusal.** A schema newer than the binary, an unparseable Machine config file — anything that will fail identically on the next attempt. Why: `KeepAlive={SuccessfulExit:false}` is what makes this work, and without it launchd crash-loops Fleet every `ThrottleInterval` until a human notices.

The Configuration setting *Fleet self-restart attempt limit/backoff* describes a cap and a backoff curve launchd does not offer, and is **not implementable as written** (see Open questions).

**Every Drone is spawned with `setsid`.** launchd signals a job's whole process tree, so a Drone spawned as a plain child of Fleet is **killed every time Fleet restarts** — measured, at both `kickstart -k` and `bootout`; a `setsid` child survived both. Silently, mid-Job, burning tokens against a real repo — a Drone killed by a restart it had nothing to do with, which is the opposite of the one killing Fleet does do: a cap, deliberately, on a Drone that is burning without converging.

`setsid` buys protection against exactly one thing, a group-directed signal. Fleet itself needs none, since launchd parents it to PID 1 at birth and PID-1-parented children are immune. macOS ships no `/usr/bin/setsid`, so this is a `libc::setsid()` call in `Command::pre_exec`.

**These two ship together.** Detaching Drones is what makes restart safe, and it is also what creates the orphaned-but-alive reconciliation case.

### Process groups and cleanup

**Whoever spawns, owns.** `checks-runner` holds the groups it spawns during a Job; Fleet holds the ones it spawns during the startup sweep, when no Check is running and no runner is involved. Why: the thing that started a process is the only thing that can be sure it is the one to stop it.

Neither reaches into the other, so make-a-group and kill-a-group is a utility both use rather than a service one owns.

#### `checks-runner` takes no live handle

**`checks-runner` takes injected data, never a live handle into Fleet.** It is a separate crate precisely for that, and a group handle is a live handle wearing a different name, so Fleet does not spawn on its behalf. Why: that coupling is what made v1's runner unportable.

#### The group id is persisted

**The group id is persisted against the Job**, alongside the port claim and the Drone PID. A live handle cannot be the only record: if `checks-runner` dies with children still running, the group outlives its owner and nothing holds it. With the id stored, Fleet's restart reconciliation of dead processes against live Jobs can kill an orphaned group in the pass it already runs.

If Fleet stays up while `checks-runner` dies, nothing sweeps until the next restart. Whether that sweep runs at start, on a timer, or both is open (see Open questions).

#### Where the platform difference belongs

Process-group semantics differ across platforms, and where that difference belongs is tracked in `../contracts/adapters.md`.

## Repositories

**One Fleet serves many repositories.** A person adds each by folder, or clones one from a URL into a folder they pick, and the rail's project picker switches between All repositories and their Manifests. The Board follows the picker, and [Job Board](job-board.md) owns why. The rejected alternative was one Fleet per repository, which would have put a store and a listener per project in front of a person who wanted one.

> **Rule.** A Job belongs to the repository whose Manifest it was created against, found through `owner_manifest_id`.
> Why: every path a Job writes to — its worktree, its records, its main checkout — is that repository's, and a lookup that fell back to another would work a Job in a tree it never ran in.

> **Rule.** Two served repositories never declare the same Manifest id, and a folder already served is refused.
> Why: the id is the whole of how a Job names its repository, so a second holder would make that name ambiguous.

**Fleet starts with none.** A fresh install has nothing set up, so `armada serve` starts, binds and serves an empty list, and the person goes to find the first repository. The working directory is not a repository by default; a folder given to `armada serve` is added as `add_repository` would add it, and refused before the bind if it will not read. A route that needs a repository refuses plainly while none is served, and reconciliation and the agent door have nothing to act on. `armada check` and `armada run` are about the repository a person stands in, and still read it from there.

> **Rule.** Every change to the list is published as `repositories.changed`, carrying the list whole.
> Why: a repository added in one window, or a clone that lands after its dialog closed, has to reach every open Bridge, and a delta would be a second shape to keep in step with `list_repositories`.

**A folder with no `armada.yml` is served, for Scan to read.** It lists as a repository and not as a Manifest, and it gains its Manifest when Write puts one at its root. A folder that is not the root of a git repository is refused in one sentence naming it once and saying to choose the repository's root or clone it; git's own message goes to Fleet's log.

**Each repository keeps its own Manifest, workflows, records, worktrees and checkout runs.** A route acting on one names it — `?manifest_id=` where it has a Manifest, `?repository=` on Scan and its proposals — and an absent name is the first added that has a Manifest, or for Scan the first added.

**Restart and adding reconcile the same way.** Fleet remembers every repository it serves, serves them again on restart before reconciling, and reconciles each over its own Jobs. A remembered folder that is gone is said and stays remembered; a Job whose repository is not served is left as it stands, and is reconciled when that repository is added.

**Each repository's main checkout is held apart.** A Verify is kept per repository, so each run sheet shows its own, and one underway refuses runs in that checkout only; two repositories Verify at once. A server names the repository it runs in, by its Manifest id.

**What stays Fleet-wide:** the concurrency cap and headroom, the store, and the listener.

## Scheduling and gating

### Drone dispatch control

**Every Job-level Drone dispatch requires explicit human approval, strictly one by one.** This is the primary control on Fleet's autonomy, not a resource question. **Nothing weakens it, including an incident.**

A Job marked `urgency: incident` has its approval surfaced sooner — it interrupts rather than queues and takes the scheduling tiebreak — and is approved exactly like any other. The Production Support Incident variant varies thresholds, not the approval path.

**The configurable concurrency cap bounds how many Drones run at once**, and admission refuses past it: a Job approved while the cap is spent stays `queued` and the Board says `waiting_on_resources`. It was informational and display-only until Throughput, when the second working slot arrived and a number that bounded nothing became the number that bounds this.

**Admission is the only thing that starts a Drone**, which is what makes the bound a bound rather than a default. Every act that puts a Job back to work goes through it — approving at a human gate, asking for changes, restarting a step, overruling a verdict — so a person cannot push Fleet past its cap by pressing a button on a Job that is already approved. None of those acts is refused when the cap is spent: the decision lands, and the Job waits at `queued` saying why. [Job](job.md), Recovering an escalated Job, has the two that were exceptions until they were not.

**It bounds Drones and never approvals**, which is the whole of how the two sit together. The cap decides how many *approved* Jobs run at the same time; it never decides that a Job is approved, never batches approvals, and is not a way to approve several at once. The gate above is untouched by it.

Sub-dispatches inside an already-approved Job need no separate approval — see [Workflow](workflow.md), Dispatch Approval, Two Levels. Away-from-desk pre-authorized batches are supported.

#### Sub-dispatch and the Job proposer are two routes, and they do not converge

Both turn one thing into several Jobs, and the difference between them is the approval gate — which is why folding them together was rejected rather than deferred.

| | Job proposer | Sub-dispatch |
|---|---|---|
| Who asks | A person describing work | The Drone of an approved Job, on its dispatching step |
| What comes back | Top-level Jobs at `awaiting_approval` | Children at `awaiting_approval`, stamped with the pass that proposed them |
| Who approves | A person, **each Job in turn** | A person, **once, on the plan**, with every child it proposed — `approve_wave` |
| What was read | The request, as it was typed | The children themselves, and the plan under `.armada/artifacts/` they came from |

**Neither path gets past the gate any more.** Until spike 022's slice 6 the sub-dispatched case was the one exemption from it: a child entered `queued` because a person had approved the plan it came out of. Now a child waits at `awaiting_approval` like any proposal, and what differs is the act that releases it: one press of the parent's plan releases the whole wave, and a child is never approved alone (`.claude/decisions/2026-09-30-approving-an-epics-plan-releases-its-wave.md`). They still reach different constructors — `create_top_level` and `create_proposed_member` — and neither takes a status.

**The saving that would have justified converging them is not there.** What the two share is drafting a proposal into a Job, and they already share it: both go through the same refusals for a blank title, a workflow nothing holds and a Manifest that is not this one. What differs is everything about who decided and what they read.

### Concurrency gating (resources)

**Fleet reads memory and disk headroom before spawning each Drone.** It reads CPU too, for Doctor, and never holds a Job back for it: the operating system schedules CPU, and on 12 Sep 2026 other agents loading the machine to 27 on 10 cores left Fleet the only thing on it that yielded. The concurrency cap, the memory share and the disk floor are changed from Bridge while Fleet runs, persist across restarts, and apply at the next admission turn; nothing already running is stopped. If capacity is unavailable the Job queues and shows "waiting" on the [Job Board](job-board.md). This determines resource eligibility only; the approval gate above determines whether a Drone starts at all.

**Checks share one limit for the machine, and not every Check costs the same.** How many places run at once is a fourth limit, changed from Bridge the same way, and every Job's gate, every Drone's own run, fix drafts and proofs after a merge wait in one line for them, across every repository. Most Checks cost one place; a repository may declare `checks.<name>.places` heavier, so a browser suite does not cost what `format` costs and several no longer time each other out sharing the machine. A Drone's own run takes the next free place ahead of a gate, and nothing waiting is passed over more than a bounded number of times — `fleet::places`. Before each Check after the first on the machine, Fleet asks the same memory and disk; a short machine makes the next Check wait for a running one to finish, so it slows Checks and never stops them.

**It is the same predicate the concurrency cap is asked through**, so a Board cannot say a Job is blocked while Fleet is starting it, and the reason is recomputed at every read rather than stored — headroom frees on its own, so a written-down reason is wrong from the moment it is written.

**Disk earns its place from a measured failure**, not from symmetry with memory: a volume filled during a parallel agent run and agents died at zero bytes free holding uncommitted work, with no warning of any kind. It is also the one held against an absolute floor rather than a share, because what a Job costs in disk — a worktree plus a build — is a number of gigabytes rather than a fraction of whatever volume it landed on, which is why it is a settings row of its own rather than sharing the memory threshold.

**Quota is not a third.** The agent's rate-limit event carries a window and a status and no quantity, so there is no number to hold a Job back against. See the spike on what a Job costs.

**A machine that cannot be read admits.** A failed reading holds nothing back: a Fleet that queues every Job for ever because a command did not answer is a Fleet that looks dead, and the concurrency cap is still the cap.

**A person's act is never refused for a machine.** An approval, a restart, an override and a request for changes all leave the Job at `queued` whatever the machine holds; admission is the only thing that starts a Drone, so a Job a person just re-queued waits exactly as any other queued Job does.

**A free worktree slot in the Job's own repository is asked the same way, per Job** — *Worktree slots* below. `get_capacity` does not name it, because it is a repository's rather than the machine's.

**Which of the three reasons is holding a Job is fleet-wide, not per row.** The Board has one label for all of them; `get_capacity` says which one it is — the cap, memory or disk.

**That poll only covers Jobs that have not started.** A Job that exhausts CPU or memory while already running has nowhere to queue back to and escalates as `resource_exhausted`.

### Budget gating (what the Job has spent)

**Fleet reads what a Job's Drones have already cost and how often they have turned, before starting another one on it.** Over either ceiling, the Job stays `queued` and reads `over_budget`, with `budget_hold` saying which of the two. It is the third reason a Job waits and it is asked through the same one-answer arrangement the other two are, so a Board cannot say a Job is over budget while Fleet is starting it.

**It reads before "waiting on resources" when both hold.** Headroom frees on its own and a spent budget does not; a person told their Job is waiting for the machine would go and watch something that is already on its way, while the thing actually holding it needs them.

**It refuses the next dispatch. It does not stop a Drone that is spending.** What a run cost arrives on the final line of a Drone's session and nowhere else, so there is no mid-session figure to interrupt on. What that catches is a runaway *sequence* of Jobs, which is the shape a runaway has.

**The cap is per Job and the spend is per Drone.** A Drone belongs to a step, so a four-step Job is four Drones that never meet, and the sum lives in the record because nothing else outlives all of them. The dollars are notional — what the run would have cost at list price — and the turn count is the steadier signal beside them; [Machine](machine.md) carries both numbers and the measurement behind them.

A **port span that cannot be re-claimed during a scope revision** is the case with a graceful path and does not escalate: the revision fails rather than the Job, which continues on its pre-revision claim and never leaves `running`.

### Freeze gating (what the repository has said)

**A frozen gating Manifest holds a Job at `queued`, reading `frozen`.** It is asked first after `paused` (a Job a person paused reads that, and lifting a freeze would not start it), before a dependency, a budget or headroom: while it holds, none of those would start the Job either, and only a person lifts it. `frozen_by` beside the label names which Manifest, and admission and the label ask one predicate, `fleet::freezing`.

**A running Job is held at its next step boundary, not stopped.** The step it is on finishes; once it passes its gate the Drone stands down on the same `running -> queued` edge a dispatching parent takes, and re-admission puts a fresh Drone on the next step when the freeze lifts.

**Nothing lands while it holds.** The delivering step is where the branch goes out, and it is not entered; the `auto_merge` sweep does not merge; a person's merge press is recorded and carried out by the first sweep after the freeze lifts.

**A person's act is never refused for it**, for the reason above: an approval, a restart and an override each land where they always do, and where a step follows, admission holds the Job at `queued`.

### DAG scheduling

**Fleet schedules by dependency graph in topological order**, on top of the approval and resource gates. **The tiebreak between ready peers reads a Job's `urgency` field**, which is what the concurrency/priority tiebreak setting names.

The dependency model itself is a property of a Job — see [Job](job.md). A Job whose members are Jobs is scheduled over the same graph, one edge per member — see [Landing](landing.md).

**An edge releases on the upstream's terminal status, and not every terminal releases it.** `completed_success` makes the dependent dispatchable; `superseded` unblocks it and surfaces it with the dependency marked unsatisfied, since the work landed outside the Job rather than not landing; any other terminal escalates it as `dependency_failed`, so a person decides rather than one failure terminating a chain unattended.

An edge carries no strength of its own — every edge gates identically and the variation is in that terminal status.

**A cycle is unstatable rather than detected.** An edge may only name a Job that already exists, and a Job's edges are written once, at creation — so every edge points at a strictly older Job and there is no acyclicity check to keep in step. A proposal naming a peer Fleet does not hold is refused where it enters, beside the workflow id and the Manifest id it is refused with.

**Escalating stops at the first dependent.** Nothing below it is cancelled or moved: `escalated` is not terminal, so a Job waiting on the one that was just escalated is still waiting on a Job that may yet run.

### Scope revision mid-Job

**Rescope-and-respawn.** Fleet terminates the Drone, re-resolves configuration against the new declared set, and spawns a fresh Drone **on the same worktree and branch** — work survives, session context does not, since Facts and Evidence live on the Job rather than on the Drone.

**Narrowing proceeds unchallenged. A person's widening returns to the dispatch approval gate; a Drone asking for one is answered by the [Judge](judge.md), and the Job never leaves `running`.** Why: scope is not a permission system — a declaration never bound writes, and it exists so that drift is detectable — so *does this belong to the step this Drone was given* is a Judge's question. A refusal escalates, which is where a person comes in. [Change a Job's scope](../journeys/change-a-jobs-scope.md).

Known cost: permissions intersect across the declared set, so a respawned Drone can come back less capable than the one that asked to widen. A judged widening respawns nothing, so the Drone that asked keeps its own toolset and the intersection reaches the next one.

### Job proposal

**Fleet makes the [Job proposer](job-proposer.md) call on every dispatch path**, single-Workspace repos included, and surfaces what comes back at the dispatch approval gate above rather than acting on it. What it proposes, and why it runs uniformly, are on that document.

What Fleet takes from it: a workflow, a title, and how many Jobs the request is. **Not `write_targets`** — the proposer does not propose them, so a Job reaches the gate with `write_targets` null, and null is not empty. Which paths the work touches is the first step's, declared by a Drone that has read the code.

This sentence used to say the opposite, and the overlap warning below was written on it. It was wrong from the day the proposer shipped.

### Write-scope overlap

**Surfaced, never serialised.** Where two unfinished Jobs claim the same paths, Fleet says so on each one's detail — naming the other Job, its status, and the paths both reach. Nothing is held back and nothing is refused: approving anyway is allowed and is the common case.

**Merges taking turns onto the base is a different thing from serialising work.** Only the step onto the base waits in line; [Merge line](../capabilities/merge-line.md) holds it.

**The working Drones are told too, and only within one repository.** When a Job first claims a path another claims, both Drones hear which Job and which paths; when one lands, the other hears what it changed there. News is spaced so a busy repository does not interrupt a Drone every few seconds, and a Job with no live Drone hears it in its next opening brief. `docs/contracts/agent-prompt.md`, The peer turn, has the wording. It is the same comparison as the warning on detail, and it holds nothing either.

**Overlap is not ordered; a declared need is.** The warning is deliberately not a lease. Why: `write_targets` is a declaration and a Drone's worktree is a whole-repo checkout, so a hold over declared paths would serialise the Jobs that declared honestly and miss the one that wrote somewhere it never named — which is the collision nobody saw coming.

**It compares what two Jobs claimed, and a claim is not a write.** The Job that never names a path and writes there anyway produces nothing here, and cannot: that is the same whole-repo checkout the paragraph above turns on. The check that reads a real diff is the per-step drift check, and it measures one step against its own plan.

**A Job's claim has two possible authors, and the warning says which.** `write_targets` is what the requester stated before anything ran, and it is null on every Job the proposer drafted. A step's declared plan is what the Drone working it said, having read the code, and it is what a running Job actually has. Both are compared; each named path says which author it came from on each side. The latest run of a step replaces its earlier runs, because calling the scope tool again is how a Drone corrects its plan. Neither list is authoritative over the other; [Change a Job's scope](../journeys/change-a-jobs-scope.md) holds what each is for.

**A Job that has claimed nothing is not compared at all**, and that answer is distinct from "compared and found nobody". Every proposer-drafted Job is in the first state at its approval gate, so **the overlap is ordinarily first visible once both Jobs are running** rather than on the card of the second one to be approved. Naming an overlap before the second Job's paths are known would need the proposer to guess them, which was measured and rejected — see [Job proposer](job-proposer.md).

**Every unfinished Job, not only the running ones.** The pair is one fact and it has to read the same from either side; naming only the running peers would have made two Jobs' detail views disagree about whether there is a collision. The other Job's status travels with the warning, so a person can see which of the two is already writing.

The remedy needs no new state: `depends_on` already sequences Jobs and already parks the waiting one at `blocked_by_dependency`. **Taking it is not built** — there is no operation that writes an edge onto a Job that already exists, and this page says above that a Job's edges are written once, at creation. That write-once property is what lets DAG scheduling above skip a topological sort, so an operation that breaks it is not a small one; `#231` is where that is settled. What a person has today is the two gate answers they already had.

### Declared needs

**Decided by the owner, 2 Oct 2026, built 5 Oct 2026 (#1059).** A need is a path and what is needed there, in the declarer's words: `crates/store/src/migrations.rs`, *a new migration*. No repository declares kinds up front, because the file is the resource. Overlap above stays a warning; a declared need is the part that is ordered. `.claude/decisions/2026-10-02-a-plan-leases-its-numbers.md` has the reasoning.

**One order, shared with `armada need` and with sessions.** A need is a row of `ledger_attachments`, the session ledger's table (`docs/capabilities/needs.md`), kind `need`, held by the Job. Fleet writes it, and serves `armada need` from the same table, so a Job, a session that reported the need through the intake and a terminal on its own branch see the same line, and none can be ahead of another without having declared first. A terminal's holder is the Job whose branch it is, else the session standing on it, else the branch alone. The files under `armada-needs/` that this replaced are read once at Fleet's first start and left on disk.

| | |
|---|---|
| **Declaring** | A Drone adds `needs` to `declare_scope`, the call that corrects its scope, or to a task of `record_plan` or `add_task`. A plan's task carries its needs by declaring them as the plan is kept; the row is the record, so nothing is added to a plan's task |
| **First goes first** | Declaring again records nothing. A Drone says what it took by calling again with `took` on the need |
| **A later declarer is told** | Which branch is ahead and what it took, in the peer turn, **at once** rather than after the spacing, and in the next opening brief where no Drone is on the Job. `docs/contracts/agent-prompt.md`, *The peer turn* |
| **Landing follows the order** | A press to merge is refused while a need ahead of the Job's on the same file stands, as `fleet.merge_waiting_behind`, naming what it waits behind. The sweep that merges for `auto_merge` asks again each rotation |
| **Spent or given back** | When the Job reaches a terminal status: spent if it landed, given back if it was dropped, both written in `record`, the one place every terminal status passes. A need held by a branch alone is given back by whatever reads next after the branch is deleted |
| **Its own slot and branch** | A Job holds them as rows too, written where Fleet leases the slot and records the branch, so `who_owns` names a Job that holds a slot and a Job that moves slots gives the old one back |

**`merge_by: forge` and `merge_by: push` hold alike**, because it is Fleet's own press that asks the forge to merge under `forge`, and Fleet that merges under `push`. A person pressing the forge's own button bypasses it, and Fleet does not see that press: the work lands out of order, and the need is spent when the Job is noticed landing. Decided for the build, 5 Oct 2026; the owner's open question had been what a need means under `forge`.

**Nothing expires by time.** A need that stalls holds every Job behind it, the cost the owner took. A person gives it back with `armada need --release <path>`, run from the branch; an act on a Job's detail that does the same is not built.

**A task that is dropped does not give its need back.** A need is the Job's, not the task's, so it stands until the Job ends or a person releases it.

### A test broken on main

**A Drone that hits a test already failing on main says so, and Fleet checks before anything is drafted.** Through `draft_fix` it names the Check, the test and the files the test lives in, which Fleet checks are files in main's checkout; Fleet then runs just that test against a checkout of main, with the command the Check's `one_test` declares. The call answers once that run has started, and what it came to reaches the Drone as a later turn, as an asked run's report does. Only a failure there drafts the fix, and the fix waits at the approval gate like any proposal. A pass there means the failure is the Drone's own, and nothing is drafted. [Manifest](manifest.md), Running one test by name, holds the key.

**The fix claims the test, so the same breakage is fixed once.** A claim names the repository, the Check and the test. A second Drone reporting that test is told which Job is fixing it, and nothing new is drafted. The claim ends when the fix's pull request merges or closes, or when the fix Job ends without one, and forgetting the fix removes it; the Job that reported it is kept by id rather than linked, so forgetting the reporter first leaves the claim standing.

**A Job that fails on a claimed test is pointed at the fix, without asking.** Where a Check the claim names fails in another Job of the same repository, at the gate or in an asked run, and what it printed carries the claimed test's name, Fleet points that Job at the fix and its Drone is told which Job is fixing the test, in the peer turn. A Drone that calls `draft_fix` on a claimed test is pointed at it the same way. When the fix lands, every Job pointed at it is told; when it ends without landing, they are told nobody is fixing the test now. Both Jobs' detail show the pointer.

**Bounded the way an asked run is, one directory over.** A Drone waits on one Check run at a time, a step asks for at most one fix, and the checkout of main is shared by every Job on the repository, so one run is out there at a time.

**It passes nothing.** A Drone told a test is someone else's still has its own step decided by its Checks, and a fix drafted from its report still takes a person's approval.

### A test another Job is fixing

**Decided by the owner, 2 Oct 2026: Fleet keeps the Job off the test, rather than only telling a person.** While a claim stands, the test's files are outside the write scope of the Job that reported it and of every Job pointed at the fix. The fix itself is not held. #1673.

**Which files.** The files the reporting Drone named in `draft_fix`, then whatever the fix has declared it will change: its `write_targets` and its steps' plans, the claims the write-scope overlap above compares. A claim from before #1673, or one Fleet drafted itself from a repeated failure, names no files and holds only what the fix declares. Both Jobs' detail carry the list as `held_off`.

**Where a held Job meets it**, and none of these takes the Drone's word:

| Where | What happens |
|---|---|
| The opening brief | A block names the files and the fix, for every Drone the Job puts on, a task's included |
| The Drone's launch | An edit to each file is denied on the argument list, as a git verb is |
| `declare_scope` and `request_scope` | A path under a held file, or a directory over one, is refused with its own answer, and no Judge's lift reaches it |
| The gate | A change to a held file fails the step under its own row, `held_off`, whatever wrote it — a shell command the launch's deny never saw included. No lift reaches it |
| A Drone already working | Told by the fix report or the peer turn, which name the files |

**The hold outlives the merge.** Fleet merges the base into a Job's branch only as a Drone is put on it (*Catching a branch up*, below), so when the fix lands every held Job's copy is still as broken as it was. The claim is given back at the merge as before, and what it held stays held off each Job until that Job's next catch-up takes the base. Then the files are the Job's again, for any reason of its own, and that Drone is told the fix is already in its copy. A catch-up git could not replay keeps the hold. A fix that ends without landing frees the files at once.


### Catching a branch up

**Rebasing a Job's branch is Fleet's, always.** Never the Drone — it has no git and `docs/concepts/drone.md` says outright it cannot be trusted to manage its own state — and never nobody, which is what "the base moved and the step never noticed" is.

| | |
|---|---|
| Clean rebase | **the worktree the Drone is in is updated in place** — same path, same branch, same work, the base moved underneath it |
| Conflicted rebase | **the Drone is asked to resolve the conflicts** before it continues |
| A rebase that will not replay | the branch is left exactly where it was, and the Drone is told so and told it is not theirs to fix |

**Nothing is created and nothing is discarded.** A rebase is not a new worktree: `Vcs` has no removal at all, a Job's earlier steps' work lives on the worktree it is on, and a restart exists precisely so that work survives. "Bring the branch up to date" and "keep the worktree" are the same sentence, not two competing ones.

**Every moment that starts, resumes or advances a step catches up first.** There are seven, and **six of them are a spawn**:

| | who is told, and how |
|---|---|
| A mechanical step boundary | the Drone being spawned for the next step, in its opening brief |
| A boundary a person approved | the same |
| A boundary a person overruled | the same |
| A first dispatch | the Drone being spawned, in its opening brief |
| A restart of a stopped step | the Drone being spawned, in its opening brief |
| An override where the step's Drone has gone | the Drone being spawned, in its opening brief |
| The step that sends the work out, being entered | the Drone being spawned, in its opening brief — and the catch-up is what the branch is pushed from, so a conflicted one is not pushed and gets no pull request |

**This table used to divide by whether a Drone was there to be told**, and the first three rows read "the live Drone, in the turn carrying the verdict". A Drone belongs to a workflow step ([Drone](drone.md)), so a step boundary ends one and starts another — there is no live session at a boundary and nothing to inject a turn into. The division collapsed, and what is left is one shape: **every catch-up rides an opening brief.** The last row used to read *a finished Job — nobody*, because the branch went out when the Job ended and there was no Drone to tell. It goes out when the step that declares `delivers` is entered now, and that is a spawn like any other.

**The conflict is therefore always the new Drone's opening work.** Refusing the act instead would put a person at a merge conflict inside a Drone's worktree, which is the one job the Drone is already in the right place to do.

**The step's baseline is read after the rebase, never before.** A rebase writes content: a clean one replays the branch onto a base that itself moved, and a conflicted one leaves markers in the files it could not merge. A baseline taken before it credits the step with git's output, and a Drone that resolved nothing then passes `diff_nonempty` on what it was handed.

**The baseline is taken once, when the step first begins, and kept in the store.** A requeue, a retry and a Fleet that restarted all put a Drone back on a worktree that already holds the step's uncommitted work, so a baseline read again would count that work as inherited and the step could never pass. Every later entry loads the stored one instead, and a gate that rules at boot on a submission the last Fleet never ruled on is measured against it too. The row goes when the step advances, so a step a later one sends work back to starts afresh. A step with no stored baseline fails `diff_nonempty` rather than passing it.

**A rebase on a re-entry is carried across.** Fleet reads the worktree just before the catch-up and again after it, and what differs between the two is the rebase's: those paths take their new entry in the stored baseline, so markers and merged files are inherited, and the step's own work in every path the rebase did not touch keeps counting.

### Network loss mid-Job

**The Drone and Job auto-retry on reconnect** and resume where they left off. A Fleet restart flags `interrupted` and then restarts the step — see the daemon lifecycle above.

### What Fleet knows after the merge, and what it does not

**Armada opens a pull request, and a merge Fleet did not perform is only ever knowable by asking.** The decision to merge stays a person's: `auto_merge` says what Fleet may carry out on its own, and under `never` nothing merges until a person asks for it — see [Manifest](manifest.md), *Auto-merge and review gate*. A merge asked of Fleet is Fleet's own act, so what merged is proved on the spot; a merge made on the forge is found on a later sweep.

Fleet asks about **one** pull request per sweep and rotates, because the turn interval is 250ms and asking the forge is a process — an open pull request needs asking rarely and a merged one never again.

**An open one is asked a second question on that same turn**, and only an open one: who has reviewed it, what the forge's own checks report, and what anybody wrote on it. That is one more process per sweep on the turns that land on an open pull request, on the rotation that already exists rather than on a loop of its own — a second loop over the same set would double the cost of the same question and disagree with the first about a pull request that settled between them. The reading is therefore a whole rotation stale at worst: ten open at a minute apiece means an approval is seen up to ten minutes after it lands.

**A forge approval is a person's signal on a diff, and Armada treats it as nothing more.** It is not a Judge verdict, it passes no Check, and it moves no Job — a Job at `awaiting_review` when the reading arrives is at `awaiting_review` after it. The forge's own checks are counted apart from Armada's Checks for the same reason: a Check is named in `armada.yml` and Fleet ran it against a worktree it made, and folding the two together would let a surface say work had been verified that nothing verified. What is written down is a line in the Job's log, and only when the reading changes.

**A comment on a pull request is untrusted input.** It is written by whoever can see the pull request. How many there are reaches the Job's log on the sweep and what they say does not — the road a comment's text travels ends at a file in a Drone's worktree, never at a log and never at a prompt directly. The sweep's own one-call budget reads the pull request's conversation and the note beside each review. A comment left on one line of the diff is a second query: a person opening the comments or pressing on them reads those comments whole, and the sweep asks it only to count them.

**Since `#661`, a changed reading also wakes Bridge, without a reopen.** The same sweep that decides whether to write the Job's log line also compares a stronger signature — every remark's handle, a hash of its words, and every reviewer's verdict, with the comments on lines of the diff read the same way — against what it read the sweep before. Where that changed, Fleet publishes `job.remarks_changed`, naming the Job, and Bridge re-asks `get_remarks` for whichever Job's comments are open on screen. The event carries no comments itself: `get_remarks` already answers what changed, and it also brings in the comments left on individual lines of the diff. A line-comment read that goes unanswered keeps the last answer in the signature, so a silence never wakes Bridge. Nothing fires on the first sweep to read a given pull request — a Fleet that just started has no earlier reading to compare against, and firing there would publish one of these for every open pull request it holds the moment it came up.

**The title and the comment count are kept on the record, so they outlive the merge.** Everything else the sweep reads is remembered only while the pull request is open, and forgotten when it settles or Fleet restarts. The title is written when Fleet opens the pull request, then on every read, the settling read included. The count is what the forge shows: the conversation, the reviews that say something, and the comments on lines of the diff, which cost the sweep one more forge call on a turn that finds the pull request open. A turn where either read goes unanswered keeps the last count, and the count stops moving when the pull request settles. A pull request that settled before the rotation ever found it open has a title and no count. `get_job` serves both and never asks the forge for either.

**How fresh a comment appears rides the same rotation named above, restated for this path.** A pull request is re-read once per sweep interval, and the rotation reaches one open pull request per interval — so with ten open at once and a sixty-second interval, a comment can sit for up to ten minutes before Fleet even reads it, and `job.remarks_changed` follows on that same sweep. That is not fast against a handful of concurrent Jobs and gets slower as more are open at once; whether the interval or the one-per-sweep shape should change to keep pace is the owner's call and is not made here — this only states the bound.

**The issue a Job came from has a rotation of its own on the same interval** (spike 022, answer 5). Every Job in flight whose request linked an issue is asked about in turn, one issue read an interval whatever their number, so each is asked once every interval times how many there are. An edit after Fleet read the issue is kept, written into the Job's log once, and served as `origin_moved_at` on each criterion read from it; the Job keeps the words it froze. **A second cursor rather than a place in the pull requests' rotation**, so the pull requests' cadence above does not slow by the number of issue-linked Jobs.

**A person picks which comments a Drone should act on, and it is Fleet asking rather than a Drone.** Not every comment is a change request — some are questions, some are agreement, some are about something else — and a Drone handed all of them tries to satisfy all of them. So the comments are served when somebody asks for them, that person picks, and the ones they picked are written whole into a file in the Drone's worktree — no size a person's choice can be too large for, `#648`. **That is the road a note already travels**, entered a second time rather than built again: a pointer to the file goes onto the record where a person's typed note goes, the Job takes `awaiting_review -> queued`, and the same block reaches the Drone, naming the file, the count and who wrote them.

**Nothing decides what a Drone is told except a person and the forge.** The press names what the forge calls each comment, and Fleet reads the pull request again to find out what they say — so a comment edited in between is handed over as it now reads, and no client can put words in a prompt or in the file. A comment goes into that file with every line of it behind a marker, which is what makes where it starts and stops Armada's to state rather than the comment's; `../contracts/agent-prompt.md` carries the rule.

**A comment already handed to a Drone is never handed to one again.** The forge has no memory of what Armada did, so a comment stays on a pull request reading exactly the same forever, and one a Drone ran against and did not fully satisfy looks identical to one nobody has touched. Armada's own record is the only thing that tells them apart, and a press naming one is refused by name. A press naming a comment the pull request no longer has is refused whole: acting on the part of a set that survived, without saying so, is the divergence choosing exists to prevent.

**Nothing is written back onto the pull request.** Armada's own record — `record_remarks_taken_up` — is what tells a comment already handed to a Drone apart from one nobody has touched; the forge never hears about a press.

**A person presses to merge, and Fleet performs it.** That is the fourth answer at a human gate, beside approving, requesting changes and rejecting, and it is the one act Armada takes that writes into a repository Fleet did not make — so it is the loudest line in the Job's log, written before the write happens. It is not a machine deciding: `auto_merge: never` reserves that, and the press is a person. What it buys over merging on the forge is everything in the table below, at once instead of on a rotation. Whether Fleet asks the forge to merge or pushes the merge commit itself is the repository's `merge_by` — see [Manifest](manifest.md), *How work lands*.

**A refused merge says which of the reasons it was, and nothing retries.** A protected base, a conflict, a check the forge requires that has not passed, a pull request that is not open, no tool on the machine, a base that kept moving past the branch under `merge_by: push`, the Job's Checks going red on that base merged in, and a refusal Armada has no word for are eight answers rather than one, because they send a person to eight different places. The Job is left at its gate where the press found it. Under `merge_by: push` a base that moved is not a refusal on its own: Fleet merges it into the branch and gates it again first — [Manifest](manifest.md), *How work lands*.

| What one ask answers | What follows |
|---|---|
| Somebody merged it | The Job's record says so, the row says so, and the repository every worktree is cut from is brought up to what merged |
| It was closed and never merged | The record says so, and nothing else moves — nothing arrived on the base |
| It is still open, against a base that has since moved | The forge is asked to compare it afresh, **once**, because it pins the comparison at the commit the pull request was opened from and renders other people's commits as this Job's work until something moves it |
| Nothing on this machine could say | Nothing is written down and it is asked again later |

**The repository is fast-forwarded or left alone, and never anything in between.** A checkout on some other branch, a working tree carrying somebody's uncommitted change, a repository with no remote, and a history that will not fast-forward are all refusals — this is the one thing Fleet writes into the repository a person is standing in, and `--autostash` is not on offer because nobody asked for a rebase.

**What merged can be proved, and only where the repository asks.** A Manifest that names Checks under `after_merge` has them run once against the tree the fast-forward left — see [Manifest](manifest.md), *Proving what merged*. A Manifest that names none runs nothing, which is every Manifest by default: what this costs is a build and test run on the machine somebody is working on, for an event nobody is waiting for, and `../practices/rust.md` section 8 names a hook that rebuilt on merge as the cause of v1's four-minute cold build.

**The run belongs to the commit and its record is keyed by the commit**, in a table with no `job_id` column and no foreign key to `jobs`. Two Jobs merging within a minute are two merges into one commit; the second finds it already proved and starts nothing. What is keyed by a Job is only the line saying it happened, which goes into the log of whichever Job's turn started the run — beside the fast-forward's own line, and for the same reason.

| The state after the fast-forward | What is proved |
| --- | --- |
| The repository moved on, or already had it | Its Checks run against the commit `HEAD` is now on, unless that commit has already been proved |
| It was left alone | Nothing. There is no updated tree, and a run against the checkout would be reporting on somebody's uncommitted work |
| A run is already out | Nothing. One suite at a time on one machine — two would contend for the same cores and neither would be measuring anything |

**Nothing depends on the answer, and that is a constraint rather than a caveat.** The work is already merged. A red cannot fail the Job, cannot reopen it — `completed_success` is terminal — and rolls nothing back. A merge that breaks main is raised by a person, and the response is a new Job pointing back through `subject`. See [Job](job.md). Whether Fleet should watch the merges it did perform is open (see Open questions).

### What Fleet knows about main's CI

**Fleet reads whether `main` is green or red on the forge's CI, for every repository it serves that names a `base:`.** It goes on the forge's facts, so it works on a repository whatever its CI: the failing job's name as the forge reports it, that job's log, and the merge that turned main red. A repository whose Manifest names no base is not read, since Fleet would be guessing which branch main is.

**It adds Check meaning only where a CI job maps to a Check.** A job maps when its name is a Check's name, or when that Check lists it under `ci_jobs` ([Configuration](../contracts/configuration.md), *Which CI jobs a Check answers for*). A job that maps to nothing is the ordinary case and is kept under the forge's name alone. A failing test is read out of the job's log by the same nextest and vitest reading the gate uses, and where the log names none there is no test.

| What Fleet asks | When |
|---|---|
| Where main stands on the forge | One ref lookup per repository per sweep interval, one repository a turn, rotating |
| The jobs that ran on that commit | When the head has moved, and again while any job has not finished. A settled commit is not asked again |
| A failed job's log, its last 256 KiB | Once per failed job per commit |
| The pull request that merged the commit | Once per commit while this process lives, and only for a red |
| The repository's open pull requests, each with its `ci` | One listing a visit, the newest 100, on the same interval and rotation |
| The newest five pull requests merged into the base | One listing a visit, beside the open ones |
| The CI run on each of those five merge commits | One ask a commit, then again only while a job is unfinished, or while nothing has run on a merge under ten minutes old; a settled run is kept in memory and not asked again. Five asks the first visit after a start, usually none after |

**A red stays red until a green.** A newer commit still running does not end it, and a commit that fails on top of a red is the same red unless it fails a job the red did not have. A commit nothing ran on is not a green and does not end a red.

**A red is held while a newer commit's CI is running.** A fix may already be in that run, so Fleet keeps the red's facts and the commit they were read at (`red_commit`), names the running commits and their pull requests (`checking`), refuses `fix_main` with `fleet.main_checks_running`, and does not pick the red up. When the run ends green everything clears. When it ends red on the jobs the red already had, the red is back and the pickup runs; on a job it did not have, it is a new red naming the newer merge.

**The reading is kept, so a restart loses nothing.** One row per repository holds the commit, green, red or running, when Fleet first read it red, the failed jobs, and the merge. The merge is the pull request's number and the Fleet Job that opened it, if one did; a direct push has neither, and the forge's silence leaves both empty rather than guessed. When main goes red or green again Fleet says so on the turn it read it (`Turned::main_changed`), and acts on it in the same turn: a red whose merging pull request is a Job of ours that has ended is sent back to that Job, once per red per Job, and a green ends every Job's take of it. `docs/capabilities/merge-line.md`, *When main goes red*, has the three ways a Job comes to have a red.

**It is served as the merge line's `hub`**, protocol 23.41, 23.42 and 23.44, with the open pull requests: a pull request is `waiting_on_main` when every check that failed on it also fails on main, and its own failure otherwise. `docs/capabilities/merge-line.md`, *The hub*, has the shape and what Bridge draws. The open pull requests are held in memory, listed again on a repository's next visit after a restart. A failed job's log is not kept: it is asked of the forge when a person presses the job, through the merge line's Check log.

### Restarting Fleet

**Restarting Fleet is a `launchctl` call from Bridge, not an API command.** `restart_fleet` cannot be served by the process being restarted. Bridge already owns bootstrapping the launchd job, so it owns restarting it, and the operation is a `child_process` call rather than a protocol operation.

**`kickstart -k` does not bypass the throttle.** Issued inside a live crash-restart window it returned immediately, but the new instance took 19.0 s to appear at the 10 s default. `ThrottleInterval` 2 brings that to 2.6 s.

## Worktree slots

**A repository keeps a pool of permanent, warm worktrees and leases them out**,
to agents and to Fleet's Jobs alike, so a build starts from the last one's
`target/` instead of from nothing. Each slot is a checkout at
`.armada/slots/slot-<n>`, beside `.armada/worktrees/` and never inside it, and
the Manifest's `setup.worktrees` says how many there are — [Manifest](manifest.md), *How many
worktrees a repository leases* — until a person changes the pool on this machine.

> **Rule.** The pool is the cap. With every slot held, a lease waits and says
> so; it never cuts another tree.
> Why: on 2 Oct 2026 twenty-six agent worktrees each built the workspace cold
> on a machine already loaded past 20 on 18 cores.

| Act | What happens |
|---|---|
| Lease | Fetches the base, takes the first free slot, points it at a new branch cut from the base with no upstream, and removes everything untracked except `target`, `node_modules`, `.gitnexus` and whatever `setup.seed.paths` names. A slot made for the first time is cloned from the warm seed, as a Job's worktree is |
| Release | `armada worktree release` is refused while the tree has anything uncommitted, or HEAD has commits the lease's branch, the remote and the base all lack. A commit on the branch is enough and a push is optional: HEAD is detached where it stands, the branch keeps its commits, and the build stays |
| Park | Commits everything uncommitted, untracked files included and ignored ones not, to the slot's branch under a `WIP:` message, then releases. Never pushes. A clean slot is only released. Refused on a checkout on no branch, on the base, on a branch the lease does not name, and while a take or release is under way. It answers with the commit and the paths it took |
| Lease an existing branch | Puts a slot on a branch that exists, at its tip, so work parked there continues. Waits while every slot is held, as Lease does. Refused for a branch that does not exist and for one another checkout already has, naming where. `armada worktree lease --existing <branch>` |
| Status | Every slot, its branch, who holds it and for how long. Bridge's Cleanup draws the same reading as one tile per slot, with whether every `setup.seed.paths` entry is on disk in it (warm) and how many commits the base has that it does not |
| Clean | `armada clean` names each slot a Job holds and leaves it, branch and all. `--force` releases a completed or kept Job's slot under the same refusals as Release, and also refuses commits on neither the remote nor the base, since it then deletes the branch; a Job that has not ended keeps its slot |
| Add | One more slot, numbered lowest-unused and not made until a lease makes it |
| Remove | The slot named, and only a free or unmade one, never the last. A made one's checkout goes by `git worktree remove`, which refuses one holding anything uncommitted; held, stranded and busy slots are refused by name. The other slots keep their numbers |
| Rescue | On a stranded slot only, and only on a person's press: starts a Scout that reads the slot's checkout, and keeps its Finding against the slot. Then the person chooses Scrap or Stash below |
| Scrap | Discards the uncommitted files, puts the checkout back at the base and frees the slot. Bridge's confirm names the uncommitted files and the commits that exist only in the slot. The branch is deleted only where the base holds every commit on it; otherwise it is kept, and the answer says so |
| Stash | Commits the uncommitted files to the slot's branch, pushes the branch to `origin` under its own name, and frees the slot. Refused on a checkout on no branch or on the base, and where there is no `origin` |
| Close, open | A closed slot is never leased until it is opened. A holder keeps one closed under it until its lease ends, and it stays closed after |

**An agent's lease is held for a process, recorded beside the slot as its pid
and start time.** The command that leases exits at once, so a lock held open
could not be the holder; the `flock` on `slot-<n>.lease` only makes one take or
release at a time. The holder is the first process above the command that is not a shell
— an agent's session, or the terminal a person typed in — so a subshell or a
pipeline between the two does not become the holder.

> **Rule.** A slot whose holder is gone is taken back only when its tree is
> clean and every commit on it is on its branch, the remote or the base. Otherwise it stays held, and a lease
> waiting for a slot names it.
> Why: a slot is reused, and reuse must never be what throws work away.

> **Rule.** A lease refuses a branch that already exists with commits on
> neither the remote nor the base.
> Why: a lease cuts its branch fresh from the base, and resetting one that
> holds work would orphan it.

**A person's changes to the pool are this machine's, and outlive Fleet.** Add,
remove, close and open write `.armada/slots/pool` beside the slots' own
records, which is never committed; `armada worktree lease` and Fleet both read
it. Once a slot is added or removed, that list stands in for `setup.worktrees`,
which stays the size a fresh machine starts at. Bridge's Cleanup offers each act
in the panel of the slot's tile, and `change_slot_pool` is the act on the wire.

### A paused Job gives its slot back

**A Job a person pauses holds no slot.** Fleet ends its Drones, parks its work
on its branch with the pool's Park (a `WIP:` commit, never pushed), and gives
the slot back, so a stopped-for-now Job does not sit on one of the pool's few
checkouts. [Job](job.md), *Pausing a Job*, has what the Job reads as meanwhile.

| Act | What happens |
|---|---|
| Pause | A running Job's Drones end first, since parking under a live writer would commit half a write; then Park, then the marker and slot are written together and the Job goes `queued`. A gate holds no Drone, so it parks first and a refusal changes nothing. Refused while its Checks run, on a status that cannot pause, on a Job already paused, and when the pool or git refuses the park, with the pool's reason. Where the Drone had to go before the pool refused, the Job is left `escalated` on `would_not_start` |
| Resume | Leases the Job's own branch at its tip into **whichever slot is free**, which need not be the one it left, so its worktree path is read from the record again. A queued Job is let into the line and admission leases it; a gate Job leases at once, or waits for the first slot a turn finds free, after admission has filled the queue |

> **Rule.** A person's act on a paused Job is refused as `fleet.paused` before it
> reads a worktree, and Kill still works on it.
> Why: it has no worktree to act on, and a derived path nothing is at would read
> as a worktree that is gone.

**Fleet pauses a parked Job itself when work is waiting for a slot.** The owner's
words of 5 Oct 2026: stopped and parked Jobs "are holding onto worktree slots ...
the work can't start because the stopped jobs are holding the leases." He chose
Pause and Resume plus this, with a grace window and an off switch. A pass in the
turn, after admission, finds each waiter and pauses one Job for it through the
same path a person's Pause takes, as Fleet: the park, the marker with `by: fleet`,
`job.paused` with `actor: fleet`, and a line in the victim's log naming the waiter.

| | |
|---|---|
| **A waiter** | A queued Job that was never started or was re-queued, with no pause marker, clear to run in every way a start asks (dependency, children, budget, freeze, volume) and for room under the Drone bound and the machine's memory, whose own repository's pool has no free slot |
| **A victim** | A Job in the same repository's pool at `awaiting_review`, `awaiting_repair` or `escalated`, holding a slot the pool says it holds, working no Drone, not paused, with no redirect note waiting, and still for the grace window |
| **The order** | The Job that has been longest at its status first, ties by Job id. One victim per waiter per turn |
| **Never** | `completed_success`, `running` or a piloted Job, which are not in the list above |

**Still, for the grace window**: fifteen minutes since the Job's last event, read
from the Job's own event log. Fleet cannot see what Bridge shows, so a Job a
person has just opened is left alone. A person's resume, and a park the pool
refused, count as a move at that moment, so a resumed Job is not taken back at
once and a refusal is not asked again every tick. Both are held in memory, so a
restart forgets them. `setup.auto_release` turns the pass off and
`setup.auto_release_grace_minutes` sets the window — [Manifest](manifest.md),
*Pausing a parked Job for waiting work*.

> **Rule.** Fleet never resumes a Job it paused, and a person's Resume waits for
> a free slot without forcing another Job out.
> Why: a Job comes back when a person acts on it, and a resume that paused a
> third Job to make room would be a loop the owner has no way to watch.

**What a person finds.** The Job reads at the status it had, with a paused chip
whose `by` is `fleet`. Resume puts it back where it was, review rows and steps
untouched, once a slot is free. A pool that is full and a waiter whose slot an
agent takes between turns pauses another Job the next turn, one per waiter, so
the pass is bounded by the waiters and never by the ticks.

### Cleaning up from the grid

**Cleanup is one grid of tiles, and a press on a tile opens the panel that manages
that worktree.** A bay is a tile; so is each Job's worktree that stands outside the
pool, drawn after the bays. A Job that holds a slot is one tile, joined by job id,
so its panel carries the slot's acts and the worktree's together. There is no list
below the grid and no act on more than one tile: `armada clean --everything` is the
one bulk act, and nothing on a screen reaches for it.

| In the panel | Offered |
|---|---|
| What it holds | One row per reason Fleet holds the worktree, in git's words: uncommitted changes and how long ago the Job last moved, unmerged commits with what they are not on and their tip, a locked worktree, Jobs that need it, an unknown base branch, a failed `git status`. A Job that has not finished shows its status badge under the Job's name. A worktree holding nothing shows no row |
| Clear | The reclaim, where the worktree is on disk and the Job has ended. **Uncommitted files are committed first**, untracked in and ignored out, to the Job's branch as a WIP commit naming the Job, and never pushed. **On a bay it then releases the slot**: HEAD is detached and the directory stays. **Outside the pool it then runs `git worktree remove`**, which is safe because the files are on the branch. A branch holding that commit is kept; one with nothing uncommitted is deleted only where the base has all its commits. The confirm lists the commit, the files, the release or removal and the kept branch, with no counts, and the receipt reads *Committed to X, slot released*. A detached HEAD, the base branch, another branch than the lease names or a busy slot keeps the refusal, said in git words; a locked worktree and a Job that has not ended are refused as before. The Job's own Clear says the same | A Job that has not ended is offered none of these acts, because Fleet refuses them with `fleet.not_reclaimable` |
| Delete branch | Only once the worktree is gone, because Fleet refuses it with a 409 while the directory stands. The confirm names the branch, its tip and the commits not on the base, which stay reachable only from that tip, and the tip is what is sent |
| Forget Job | Only once the worktree and the branch are gone, and with a confirm: the record has no undo, and the worktree and branch are not touched |
| Release | On a slot an agent session holds, which offered only Close slot before. The confirm names the holder, with its pid on a tooltip, and the branch, then does what Clear does for a Job's: commit the uncommitted files to the branch as a WIP commit, release the slot, keep the branch. The holder shown is sent, and a slot re-leased since is refused with `fleet.slot_holder_changed`. The session's process is not touched |
| Close, Reopen, Remove, Rescue | A bay's own, as above. Its Finding, and Scrap, Stash and Pick up, are in the same panel |

**What an act did, and what Fleet refused, is said in the panel and never on the tile.**
A tile says its state and its figures, each named on hover.

### Rescuing a stranded slot

**A stranded slot is work its holder left and nobody has looked at.** Rescue
sends a [Scout](scout.md) to read it, and the bay offers a way into what it
found, which opens in Bridge's trailing sheet. Scrap, Stash and Pick up are in that
sheet, and are Fleet's acts, run on the press that asks. The Scout has no tool
that writes, so it never does any of them.

> **Rule.** A Scout reads a stranded slot only on a person's press, and its
> Finding is kept against the slot, not a Studio.
> Why: a stranded slot's holder is gone, so nothing else owns the Finding, and
> it has to survive a Bridge reload to be there when the person decides.

> **Rule.** No act runs while a Scout is reading the slot, and a Finding
> is of the commit the slot was at. A slot that has moved off it shows none.
> Why: the Finding describes one state of the work, and an act on another
> would be decided on a description of something else.

**Each commit on the slot's branch says where else it exists.** Fleet asks git per commit: on a
remote branch (`git branch -r --contains`), else on the local base (`git merge-base
--is-ancestor`), else only here. The Finding lists the ones that exist only here first, and the
Scrap's confirm names those.

**A Job's slot that the pool would not take back is rescued the same way.** A Job that was
killed or failed with work only in its slot cannot give the slot back, and the pool records why.
The bay shows that reason, Rescue reads the slot as it does a stranded one, and Scrap and Stash
each end the Job's claim on it, so the slot is free after.

**Pick up proposes the work again.** It stashes as Stash does, which commits the
uncommitted files to the slot's branch and pushes it, and frees the slot. Fleet then
sends the proposer a request of the branch and the Finding's items, and the Job it
proposes waits at the approval gate with its worktree cut from that branch, not the
base. The person approves it as any other.

> **Rule.** Pick up is offered on an Unfinished Finding, or one with no verdict, and
> Fleet refuses it on Scraps.
> Why: leftovers have nothing to continue, and a Finding with no verdict is one the
> person cannot tell.

> **Rule.** A Job proposed from a branch keeps it as where its worktree starts, and
> the approval can change it.
> Why: it is the landing's `from_ref`, which the approval already sets, so the branch
> is a starting value and never a lock.

`armada worktree` and its forms are in `../practices/running-locally.md`,
*Leasing a worktree*.

### A Job's slot

**A Job leases its slot when it is first dispatched, and its worktree is that
slot from then on.** The slot is recorded with the Job and looked up, never
derived — `../contracts/system-architecture.md`. Its branch is still
`armada/<handle>`.

> **Rule.** A Job's lease is held by the Job's id, never a process.
> Why: Fleet restarts often, and a pid holder would make every Job's slot read
> as abandoned after one.

> **Rule.** A slot a Job holds is never taken back for a dead holder. It is
> given back only when the Job reaches a terminal state, or, for a
> `completed_success` Job, when a person clears it.
> Why: `awaiting_review`, `escalated` and `interrupted` Jobs still need their
> work; a person may answer them days later.

> **Rule.** A completed Job holds its slot until a person clears it, from the
> Board's Clear, by deleting its record, or with `armada clean --force`. The
> sweep never gives it back.
> Why: the owner's decision of 2 Oct 2026, so Show again and anything else
> reading a finished Job's tree keeps working.

| The Job | Its slot |
|---|---|
| Waiting to start, every slot held | It stays `queued`, and the Board says `waiting_on_resources` — the same predicate admission asks |
| `running`, `awaiting_review`, `escalated`, interrupted | Held |
| `completed_success` | Held until a person clears the Job, and `armada worktree --status` reads `done`. Cleared, it is released by the pool's rules |
| `completed_failed`, `killed` | Released the moment it ends. Where the pool refuses for uncommitted files, they are committed to the Job's branch as a WIP commit first, never pushed, and the Job's log says which commit and files. A park git refuses (detached HEAD, the base branch, another branch than the lease names, a busy slot) keeps the slot, the log says why, and `armada worktree --status` reads `kept` |
| `rejected`, `superseded` | Released by the pool's rules. Refused for a dirty tree or commits on no branch, it stays held, the Job's log says why, and `armada worktree --status` reads `kept` |
| Ended, its slot kept | Released again by the sweep once every safety test passes, or by a person with `armada worktree release <path>` or `armada clean --force`. A completed Job's is not swept |

> **Rule.** A Job never loses its slot quietly. One whose recorded slot is
> held by another, given back, or gone is escalated as `no_worktree`, naming
> the slot and why; Fleet never leases it a second.
> Why: the earlier steps' work was in that slot, and a fresh one would start
> the Job over without saying so.

**A new slot is seeded from the warm base the way any lease's is; a reused one
keeps the build its last lease left**, and the Job's log says which. A Job cut
before the pool has no slot recorded and keeps `.armada/worktrees/<handle>`
until it ends.

**A finished Job's worktree is gone once its slot is given back.** What reads
a finished Job's tree — Show again, a reclaim — finds it while a completed Job
holds its slot, and for any other end only while the pool kept it.

**Completed Jobs nobody clears can fill the pool.** With every slot held, the
next Job waits at `queued` as `waiting_on_resources`, and `armada worktree
--status` names the `done` slots. Clear the finished Jobs on the Board, which
gives each slot back, `armada worktree release <path>` one by hand, or
`armada clean --force` every one that is clean and landed. A plain `armada
clean` names them and leaves them.

## Ports

A Job claims a contiguous span of ports for the life of its worktree. Each repository's main checkout claims one too, held while Fleet runs; the proof run after a merge and any server started with no Job draw from it. Fleet's own listener claims a single port the same way, out of the same range.

**A workspace Verify claims a span of its own, sized from that workspace's `armada.yml`, for the life of the Verify.** Its commands resolve `${port.NAME}` against that span laid over the main checkout's: a name the root alone declares is the root's number, and a name both declare is the workspace's. The span is kept under a key naming the workspace's directory that no repository's own key can equal, so a repository served at that same directory keeps its span through the Verify, and a row a crashed Verify left is reconciled at boot, released at shutdown, and replaced by the next Verify of that workspace.

> Why its own span, not the root's grown: a span is contiguous, so growing one re-picks it, and that moves a port a server in the main checkout is already bound to.

### The range

`Port range base` and `Port range ceiling` are settings. The ceiling is **detected at daemon start from the platform's ephemeral port floor, minus one** — `net.inet.ip.portrange.first` via sysctl on macOS, `net.ipv4.ip_local_port_range` in `/proc` on Linux — with `32767` as a fallback if the read fails rather than as the value.

A constant stores the answer instead of the rule. `32767` is one below Linux's ephemeral floor; on macOS, whose floor is 49152, it reserves roughly sixteen thousand ports the only supported platform will never use.

**Never guess high.** A ceiling above the real floor hands out spans the kernel will also assign, which is the collision class the ceiling exists to prevent. Detection supplies the default; an explicit setting still wins, so a lower cap survives and Doctor has a value to show.

This is the first place Armada reads a kernel parameter. Where that dependency belongs is tracked in `../contracts/adapters.md`.

### Claiming

A claim names a Job, the main checkout or Fleet's own listener, so who holds which span is a query rather than an inspection of directories.

**A bind-and-connect probe gates every hand-out**, and it is load-bearing rather than defensive. Why: teardown that silently failed, teardown never declared, and a process outside every tree are indistinguishable to the store.

### Fleet's own listener

**Fleet claims the port it serves on.** It was a constant, and the constant is why two Fleets could not run on one machine: both bound the same number and the second stopped at `Address already in use`, which is what stopped a development Fleet running beside the owner's.

> **Rule.** Fleet claims its listener port before it binds, and publishes the port read back from the bound listener.
> Why: the claim is what stops a second Fleet taking the same number, and reading the port back from the listener rather than from the claim is what stops the runtime file naming a port nothing is listening on.

> **Rule.** The claim is taken after Fleet has refused a repository it cannot serve, and given back when Fleet stops.
> Why: a claim taken before a refusal would have to be given back, which is the reason the bind already happens after it.

**Two Fleets on one machine are two homes, so neither can read the other's claim.** A store is found through `HOME`, so what keeps their ports apart is the probe rather than a row either of them can see. A Fleet starting over a claim its own crash left behind takes that port up again where the probe agrees nothing is on it, and gives the row back and picks another where something is.

Starting a second Fleet over a **live** one is a different act and stays refused; the runtime file is what refuses it, and this section is about two Fleets with different homes and different stores.

### Servers

Fleet holds a Manifest's server Commands — the ones with `serve`, which [Manifest](manifest.md) defines. A person starts one from Bridge; a Drone starts one through Fleet's MCP tool.

> **Rule.** Fleet runs at most one instance of each server Command per Job, and hands a second request the running one.
> Why: a Job's Drones change at every step, and the one that asks second must not start another on the same port.

> **Rule.** A Job's servers are stopped when the Job ends, before its span is released.
> Why: release is gated on teardown, and a server is an in-tree process the group kill reaches.

A server started with no Job runs in a checkout of the repository `?manifest_id=` names — its main checkout, or a worktree of it that `start_server`'s `checkout` names — and uses **that checkout's own span**. It stops on Stop, when it exits, or when Fleet stops; the span itself is released only when Fleet stops, after teardown.

> **Rule.** A port span is claimed per checkout, never per repository.
> Why: a repository has more than one checkout and each declares the same `ports:`, so one span between them is the number two servers bind. On 22 September 2026 a worktree took the main checkout's port and the address kept answering, with another branch's app behind it.

> **Rule.** A Job's worktree is reachable by naming the Job, never by naming its path.
> Why: a Job's server is held for the Job and stopped when it ends, which a path cannot say — and a second span beside the one the worktree already holds is the collision again, inside Armada.

**Fleet holds its servers in memory, so a Job's servers stop when Fleet stops too.** A restarted Fleet holds none, and a server that outlived the process holding it would keep its port with nothing left to hand it to the next Drone or stop it with the Job. Beside each running server's log Fleet keeps a record of its process group, removed when the server stops.

> **Rule.** After a crash, Fleet kills the servers it left running when it next starts, confirming each is still the process it recorded.
> Why: a leftover server holds its port with nothing to stop it, and a pid the system has since given another process must never be killed.

#### Which checkout answers, and how far behind it is

Every answer that names a server says which checkout serves it — its path, and its branch where Fleet can read one. A name and a port are not enough: another checkout of the same repository declares the same number, and before each took a span of its own the address answered whichever bound first. On 22 September 2026 five agents were told to watch one address and one of them photographed another branch's app.

> **Rule.** A held server carries the checkout it runs in on every answer and every event.
> Why: the failure is not the collision — it is that the address keeps answering, so nobody looks.

When a Job this repository owns merges and Fleet brings the main checkout forward, every server held on that checkout is told how many commits landed under it, and says so.

> **Rule.** A server whose checkout moved on says how far behind what it serves is, and is never restarted for it.
> Why: restarting under somebody mid-look is worse than telling them. Telling is the floor; pressing is theirs.

A server that falls over on its own leaves its address behind, and the next binder takes it within a second. Fleet probes the port as the server ends and says on the row where something else is answering there now.

**A Fleet holds the Manifest it resolved at startup.** A server added to `armada.yml` since is declared by the repository and unknown to this process, so the refusal names when Fleet last read the file and whether `commands` changed in that read — the list on its own reads as the repository's answer when it is this process's memory of it.

### Compose

Armada resolves the repo's compose files, rewrites every published port into the claimed span, and feeds the whole document on stdin, never to disk. The Docker adapter does the rewrite.

**Refuse any published port Armada did not place.** Rewriting is what makes refusal possible. Under environment interpolation Armada never parses the document and so can refuse nothing.

Two failures follow — a service nobody parameterised binds its base port in every worktree and reads as flakiness rather than as a missing line, and a correctly parameterised file still collides, because the fallback value *is* the collision port and fires whenever the variable is absent.

Traps, all measured: compose overrides append rather than replace, and the fix is silently ignored below Compose 2.24.4, which is why the resolved document is transformed in memory. Container-side ports must be distinct across services. Unknown, unparseable and range-valued published entries are all `bad_config`.

### Teardown, then release

Teardown is partitioned by process tree rather than offered as alternatives.

| Covers | Mechanism |
| --- | --- |
| Anything Armada spawned that stayed in its tree | Process-group kill. No declaration, no adapter |
| Containers | The Docker adapter |
| Anything handed to another supervisor | A declared Command, on the Manifest |

In-tree means a `setup.run` that backgrounds a dev server, a Check that leaves a child, a Command that forks. Another supervisor means `launchctl`, a systemd unit, or a deliberate double-fork.

A group kill cannot reach containers: `docker compose up -d` returns immediately and the containers are children of the Docker daemon, a different tree.

**Release is gated on teardown.** An undeclared supervisor-held process holds its port past release, the probe refuses that span, and the range loses it. How a person learns the range is degrading is open (see Open questions).

### An interrupted Job holds its span

**Nothing releases automatically.** An interrupted Job's worktree is never swept, and a claim's lifetime is its worktree's lifetime.

**Nothing releases and reclaims the span either.** A Drone is `setsid`, so a Drone whose Fleet died may have survived and may still be bound to those ports — releasing a span a live process holds is the double-booking the mechanism exists to prevent. It also breaks number stability, and puts Fleet in the business of deciding an interrupted Job is over.

The exhaustion message names how many spans interrupted Jobs hold and points at Alerts. **Spans accumulate rather than run concurrently**, so headroom against slow accumulation is the headroom that matters.

#### Answering the escalation is what releases it

**What ends the hold is the Job reaching a terminal.** A claim's lifetime is its worktree's lifetime, an interrupted Job sits at `escalated`, and its worktree is not swept while it stays there. A person answering that escalation — kill, redispatch or Pilot — takes the Job terminal; retention then sweeps the worktree and the span goes with it.

The act is answering an escalation rather than anything about ports, so no surface, state or timer exists for this and none is needed. The bind-and-connect probe still gates the re-issue, which is what makes the release safe where a `setsid` Drone outlived its Fleet.

#### Surfacing a held span

**A held span stops being silent before exhaustion.** Past the held-span reminder threshold Fleet surfaces the hold. It releases nothing and changes no state — the alternative is learning about slow accumulation only when the range runs out.

Where it renders is open — see Open questions — and the Doctor precedent argues for a health strip rather than an Alerts row, since a held span is a standing condition and not a queued decision.

#### A timer would decide the Job is over

**No timer releases a span.** Why: a timer would put Fleet in the business of deciding an interrupted Job is over, which is the person's call and the reason `escalated` halts autonomous action at all.

## Where Fleet's behavior is actually documented

Each piece of Fleet's behavior lives where it is specified:

- Memory and disk resource gating before spawning a Drone — Scheduling and gating, above
- Owns every `Job.status` and `Job.workflow_status` transition — [Job](job.md), Ownership Split
- Evidence verification (Mechanical Check → Judge Check) — [Drone](drone.md), [Workflow](workflow.md)
- DAG scheduling and cross-workspace work — [Job Board](job-board.md), [Manifest](manifest.md), Cross-Workspace Jobs, [Landing](landing.md)
- Secrets brokering (Drone never holds secrets directly) — [Kit](kit.md), [Manifest](manifest.md), Secrets
- Live re-evaluation of the allowlist, the budget cap and dispatch freeze at every gated checkpoint, versus Skills, MCP, Agent files and Commands, which are frozen into a Drone at spawn — [Drone](drone.md), What's Frozen at Spawn vs. Live
- Auto-merge enforcement, VCS push/PR/merge, the sole actor touching Git credentials — [Manifest](manifest.md), auto_merge and review_gate
- Rebasing a Job's branch onto its base, and who reads a conflict — Catching a branch up, above
- Schema migrations applied on startup — `../contracts/system-architecture.md` section 5, [Kit](kit.md), Upgrade
- Structured JSON logging per Job — `../contracts/system-architecture.md` section 4
- Own health status — [Doctor](doctor.md), Fleet module
- What a repo may declare about ports, Check timeouts and teardown — [Manifest](manifest.md)

**Fleet reads no Job shape, because none is stored.** A Job writes in one place or several, and lands on its own or with members before and after it; what Fleet branches on is `write_targets`, `gate_manifest_ids[]` and `dependencies`, never a category. See [Manifest](manifest.md), Cross-Workspace Jobs, and [Landing](landing.md).

## Open questions

- **[fleet-checks-runner-sweep-timing]** Does the sweep for orphaned process groups left behind by a dead `checks-runner` run at Fleet start, on a timer, or both? If Fleet stays up while `checks-runner` dies, nothing sweeps until the next restart under the current design.
- **[fleet-time-limits-on-a-loaded-machine]** Should Fleet's time limits allow for a machine it no longer protects from CPU load? Admission leaves CPU to the operating system, so a Drone can start on a machine other work has saturated and run slower than the limits were chosen against. `PROVISIONAL_LIVENESS`, `PROVISIONAL_CHECK_BUDGET` and `PROVISIONAL_COMMAND_BUDGET` in `crates/armada/src/serve.rs` are fixed durations, and none of them reads load, so a slow Drone and a stuck one look the same to them. Nothing has measured a Drone or a Check under a loaded machine. What decides it: whether a Job is ever stopped for time while its work is still moving, and whether the answer is longer limits, limits that stretch with load, or a Drone's own progress counting as life.
- **[fleet-watch-merge-after-self-merge]** Should Fleet watch main after a merge it performed itself? Fleet's model of the world otherwise stops at the merge — nothing watches main afterwards, and a merge that breaks main is raised by a person today, with the response being a new Job pointing back through `subject`.
- **[fleet-port-range-degradation-visibility]** How does a person learn the port range is losing spans? An undeclared supervisor-held process holds its port past release, the probe refuses that span, and the range loses it — nothing currently surfaces that degradation. Related: past the held-span reminder threshold Fleet surfaces a held span, but where that renders is also undecided; the Doctor precedent argues for a health strip rather than an Alerts row, since a held span is a standing condition and not a queued decision.
- **[fleet-fix-pointing-reads-a-cut-output]** How is a Job pointed at a fix when what its failed Check printed was cut before the claimed test's name? Pointing looks for the name in what Fleet holds of the output: at the gate the captured stdout and stderr, capped at 64 KiB by `checks_runner`'s `CAPTURE_LIMIT`; in an asked run the report's tail, at most 100 lines and 8 KiB. A suite that names the failing test early and prints a long tail after it is missed, and that Job's Drone is never told the fix exists — though it can still call `draft_fix` and be pointed. What decides it: whether a missed pointer is seen in practice, and whether reading more of the output is worth its cost on every failed Check.
- **[fleet-self-restart-limit-setting]** What happens to the *Fleet self-restart attempt limit/backoff* setting, given that launchd cannot implement a cap or a backoff curve? The setting describes behavior the supervisor does not offer.
