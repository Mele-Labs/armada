# Fleet

**What it is:** The Rust daemon that hosts Jobs, leases worktree slots, keeps the merge line and reads the forge. Everything else reports; Fleet decides. launchd-supervised, api in-process, no engineer-facing surface of its own.

---

**Kind:** Process.

Formalizes Fleet — the Rust daemon that hosts the Jobs working in a repository and keeps what surrounds them. Companion to the main Armada brief.

## What it is

The execution engine underlying everything else in Armada. Fleet hosts Jobs, leases the slots they write in, runs Checks, watches pull requests and main, and writes the ledger (see [Job](session.md)).

Fleet has **no engineer-facing surface** beyond Doctor's module status. It is pure infrastructure; you never "go to Fleet" the way you go to the Cockpit or Helm. Its behaviour is specified across nearly every other concept document — what this document owns is the daemon lifecycle, and a map of where the rest lives.

## Daemon lifecycle

Every number below was measured on macOS 27.0 / 26A5406e, launchd 7.0.0.

**Fleet and the API are one process.** `api` runs in-process with `fleet`; there is no second daemon. Doctor stays readable when Fleet dies through `armada doctor --json`, a short-lived probe process Bridge spawns.

- **Startup.** Bridge bootstraps a **launchd job**, plist kept at `~/Library/Application Support/Armada/` — deliberately **outside** `~/Library/LaunchAgents`. launchd only auto-loads from the LaunchAgents directories, so a plist elsewhere is never seen at login: **"does not start at boot" becomes structural rather than configured off**. `KeepAlive={SuccessfulExit:false}`, `ThrottleInterval` 2, mode 644. launchd parents Fleet to PID 1 at spawn.
- **Quitting the app.** Does **not** kill Fleet. Jobs keep running, independent of Bridge, because Fleet is not Electron's descendant.
- **Reopening Bridge.** **Reconnects** to the running Fleet rather than spawning a duplicate. Bootstrapping is per-login-session, so Bridge bootstraps every login, idempotently — with a `launchctl print` pre-check, because bootstrapping an already-loaded job and bootstrapping a world-writable plist fail with the **identical** error, `Bootstrap failed: 5: Input/output error`.
- **Fleet crashes — signal or non-zero exit.** launchd restarts it automatically. Doctor's row flips fail → pass with no user action. **There is no cap and no backoff curve** — a flat `ThrottleInterval`, forever.
- **Fleet is wedged — alive, not answering.** `launchctl kickstart -k gui/$UID/com.armada.fleet`. **26 ms** to a new PID. This is what the "Restart Fleet" button does, and it means **skip the throttle wait**, not *recover*.
- **Fleet exits 0 deliberately.** launchd leaves it down by design. Kickstarting just makes it exit 0 again. Doctor must show the **reason**, not a restart button.
- **On any restart.** Reconciles against what is live, for every repository it serves. The servers a crashed Fleet left running are ended first, so the ports they held are free. A hosted Job's agent survived in its keeper and is reattached ([Job](session.md), *What a restart does to a job*). A slot still held by a Job that ended is parked. A Scout does not outlive the Fleet reading it.
- **While it reconciles.** The listener serves first and reconciliation runs in a task of its own. Health and every read answer at once. A command waits until reconciliation finishes, and the turn loop starts after it. A stop during it ends the task where it stands, as a crash would, and the next boot repairs what was left.
- **Uninstall.** Must `launchctl bootout`, not merely delete the plist. A loaded job survives deletion of its own plist — verified.

Two plist keys do not mean what they read. **`RunAtLoad=false` is a lie in the presence of `KeepAlive`** — both `true` and `{SuccessfulExit:false}` started the job the moment it was bootstrapped. **`Crashed:true` means signal-terminated, not failed** — `exit 1` left the job down, and a Rust panic exits 101.

### Two design constraints that follow, not preferences

**Fleet must `exit 0` on a permanent refusal.** A schema newer than the binary, an unparseable Machine config file — anything that will fail identically on the next attempt. Why: `KeepAlive={SuccessfulExit:false}` is what makes this work, and without it launchd crash-loops Fleet every `ThrottleInterval` until a human notices.

The Configuration setting *Fleet self-restart attempt limit/backoff* describes a cap and a backoff curve launchd does not offer, and is **not implementable as written** (see Open questions).

**Every agent Fleet starts is spawned with `setsid`.** launchd signals a job's whole process tree, so an agent spawned as a plain child of Fleet is **killed every time Fleet restarts** — measured, at both `kickstart -k` and `bootout`; a `setsid` child survived both. Silently, mid-turn, burning tokens against a real repo. A hosted Job's keeper is such a child, which is how its agent outlives a restart.

`setsid` buys protection against exactly one thing, a group-directed signal. Fleet itself needs none, since launchd parents it to PID 1 at birth and PID-1-parented children are immune. macOS ships no `/usr/bin/setsid`, so this is a `libc::setsid()` call in `Command::pre_exec`.

**These two ship together.** Detaching agents is what makes restart safe, and it is also why a Job's agent is still alive when Fleet comes back and has to be reattached.

### Process groups and cleanup

**Whoever spawns, owns.** `checks-runner` holds the groups it spawns during a Check run; Fleet holds the ones it spawns during the startup sweep, when no Check is running and no runner is involved. Why: the thing that started a process is the only thing that can be sure it is the one to stop it.

Neither reaches into the other, so make-a-group and kill-a-group is a utility both use rather than a service one owns.

#### `checks-runner` takes no live handle

**`checks-runner` takes injected data, never a live handle into Fleet.** It is a separate crate precisely for that, and a group handle is a live handle wearing a different name, so Fleet does not spawn on its behalf. Why: that coupling is what made v1's runner unportable.

#### The group id is persisted

**The group id is persisted**, alongside the port claim. A live handle cannot be the only record: if `checks-runner` dies with children still running, the group outlives its owner and nothing holds it. With the id stored, Fleet's restart reconciliation can kill an orphaned group in the pass it already runs.

If Fleet stays up while `checks-runner` dies, nothing sweeps until the next restart. Whether that sweep runs at start, on a timer, or both is open (see Open questions).

#### Where the platform difference belongs

Process-group semantics differ across platforms, and where that difference belongs is tracked in `../contracts/adapters.md`.

## Repositories

**One Fleet serves many repositories.** A person adds each by folder, or clones one from a URL into a folder they pick, and the repository picker at the top of the Cockpit's left column switches between All repositories and their Manifests. The rejected alternative was one Fleet per repository, which would have put a store and a listener per project in front of a person who wanted one.

> **Rule.** Two served repositories never declare the same Manifest id, and a folder already served is refused.
> Why: the id is how a Job, a record and a route name the repository, so a second holder would make that name ambiguous.

**Fleet starts with none.** A fresh install has nothing set up, so `armada serve` starts, binds and serves an empty list, and the person goes to find the first repository. The working directory is not a repository by default; a folder given to `armada serve` is added as `add_repository` would add it, and refused before the bind if it will not read. A route that needs a repository refuses plainly while none is served, and reconciliation and the agent door have nothing to act on. `armada check` and `armada run` are about the repository a person stands in, and still read it from there.

> **Rule.** Every change to the list is published as `repositories.changed`, carrying the list whole.
> Why: a repository added in one window, or a clone that lands after its dialog closed, has to reach every open Bridge, and a delta would be a second shape to keep in step with `list_repositories`.

**A folder with no `armada.yml` is served, for Scan to read.** It lists as a repository and not as a Manifest, and it gains its Manifest when Write puts one at its root. A folder that is not the root of a git repository is refused in one sentence naming it once and saying to choose the repository's root or clone it; git's own message goes to Fleet's log.

**Each repository keeps its own Manifest, workflows, records, worktrees and checkout runs.** A route acting on one names it — `?manifest_id=` where it has a Manifest, `?repository=` on Scan and its proposals — and an absent name is the first added that has a Manifest, or for Scan the first added.

**Restart and adding reconcile the same way.** Fleet remembers every repository it serves and serves them again on restart before reconciling. A remembered folder that is gone is said and stays remembered.

**Each repository's main checkout is held apart.** A Verify is kept per repository, so each run sheet shows its own, and one underway refuses runs in that checkout only; two repositories Verify at once. A server names the repository it runs in, by its Manifest id.

**What stays Fleet-wide:** the concurrency cap and headroom, the store, and the listener.

## Limits

### Concurrency cap

**The configurable concurrency cap bounds the width of a build**: `checks_runner::CheckWidth` is read off it. It is a Machine setting, changed from Bridge while Fleet runs, persists across restarts, and applies at the next Check; nothing already running is stopped.

### Machine headroom

**Fleet reads memory and disk headroom before each Check.** It reads CPU too, for Doctor, and never holds anything back for it: the operating system schedules CPU, and on 12 Sep 2026 other agents loading the machine to 27 on 10 cores left Fleet the only thing on it that yielded. The memory share and the disk floor are changed from Bridge while Fleet runs, persist across restarts, and apply at the next reading; nothing already running is stopped.

**Checks share one limit for the machine, and not every Check costs the same.** How many places run at once is a further limit, changed from Bridge the same way, and every Check run waits in one line for a place, across every repository, a Job's `armada check` included. Most Checks cost one place; a repository may declare `checks.<name>.places` heavier, so a browser suite does not cost what `format` costs and several no longer time each other out sharing the machine. A waiting Check is passed over at most a bounded number of times — `fleet::places`. Before each Check after the first on the machine, Fleet asks the same memory and disk; a short machine makes the next Check wait for a running one to finish, so it slows Checks and never stops them.

**Disk earns its place from a measured failure**, not from symmetry with memory: a volume filled during a parallel agent run and agents died at zero bytes free holding uncommitted work, with no warning of any kind. It is also the one held against an absolute floor rather than a share, because what a worktree costs in disk — a checkout plus a build — is a number of gigabytes rather than a fraction of whatever volume it landed on, which is why it is a settings row of its own rather than sharing the memory threshold.

**Quota is not a third.** The agent's rate-limit event carries a window and a status and no quantity, so there is no number to hold anything back against. See `docs/spikes/005-what-does-a-job-cost.md`. A run the limit already refused is held until the reset, which is not a gate: [Job](session.md), *A job that hit the plan's limit*.

**A machine that cannot be read lets the Check run.** A failed reading holds nothing back: a Fleet that holds every Check for ever because a command did not answer is a Fleet that looks dead, and the concurrency cap is still the cap.

### Job proposal

**Fleet makes the [Proposer](job-proposer.md) call when a Job is dispatched**, and the Job starts as soon as the proposal lands. What it proposes, and why it runs uniformly, are on that document. What Fleet takes from it: a title, a workflow or none, a model and an effort. It proposes no paths. [Job](session.md), *A Job's proposal*, has what the person can change.

### Write clashes

**Decided by the owner, 12 Oct 2026.** Two agents in one repository learn they collide from what they wrote, with no declaration needed. Nobody is paused, held or refused.

| Whose writes | Where Fleet sees them |
|---|---|
| A hosted Job | The write gate (`POST /sessions/gate`), for a call it lets through inside any slot the job holds, its subagents' included. The path is read from that slot |
| A terminal Job | The `armada` mod's `touched` fact: the path from the job's directory, never the content |

```
 write ─▶ table: path ─▶ holders             (memory, per repository)
           2+ holders on a path
              │  wait 30s, once per pair; a later write by either asks again
              ▼
 git: tree of each worktree as it stands, tree of their merge base,
      `git merge-tree`        clean ─▶ nothing is said
                              conflict ─▶ the files, to both
```

**A Job writes in every slot it holds.** Its holder is not the one slot its ledger row names: it is that slot and every other slot whose lease is the job's, leased for it by Fleet or by a subagent's `armada worktree lease`. Every slot of one side is checked against every slot of the other, and against main. Confirmed 15 Oct 2026: two Jobs edited the same lines for hours, each in a slot its subagent leased, and neither was told while the table read only the ledger's slot.

**A pair names only files both changed themselves.** A worktree cut from a newer main carries main's changes, and git sets them against an older holder's as if they were the newer one's. So a file git conflicts on is told to the pair only where each side's own diff since it left the base (`origin/<base>`, else `<base>`) holds it. A conflict with main's changes alone is *Main moving*'s, told to that holder only. Confirmed 15 Oct 2026: a Job holding a 31-hour-old branch in a second slot was told it clashed with another over files only main had changed.

**Git decides, not the path.** Each holder's worktree is read as a whole tree through a scratch index, so its own index is never written and a shell write no hook saw is in it. A file both edited in places that merge clean says nothing. Where git will not answer, the last reading stands.

**What each side is told.** The files that conflict, who the other is, and how to reach it. A Job hears it as a message from Fleet (`docs/concepts/session.md`, *What Fleet tells a Job*) and reaches the other with `SendMessage`. **Told once per set of files**: a changed set is told again, a cleared one is never said.

**Main moving.** Where Fleet reads the base's head and it has moved, every standing pair is asked again, since a holder may have merged main in or the other may have landed. Each live holder whose files main changed since their merge base is set against main's new tip, and a conflict is told to that holder only, with the commit on main that changed the file.

**A holder leaves the table with its clashes** when its Job ends. Any check also drops a holder whose worktree or branch is gone, or a terminal Job unheard for six hours.

**Memory only, filled from git on start.** A restart forgets the table. The first pass after a start credits each live Job with the files its worktrees changed since they left the base (`git diff --name-only` from the merge base to the tree as it stands), so a pair that clashed before the restart is checked again without either writing. `list_write_clashes` reads the standing clashes per repository, or per Job.

**Bridge marks both rows from that read.** One `swords` mark on a Job's row wherever Jobs are listed, and a line in the open Job naming the other holder and the files. Its tooltip is `Conflicts with` the other holder and the paths; a press opens that holder, and a clash against `main` is a picture with nothing to open. The Cockpit's map draws a standing clash as an amber hatched slab on the directory both write in, and its tooltip reads *Two drones are changing it*, or *Changed elsewhere too* where the map holds one. The read is poll-only, since Fleet publishes no event for a clash. Bridge asks once per served repository when Fleet connects, again 35 seconds after the board last changed (just past the 30 seconds Fleet waits before it asks git), and again every 35 seconds while a clash stands so that a cleared one goes. A Fleet that answers 404 has none to report and is marked as having none. `apps/desktop/src/renderer/src/fleet-clashes.ts`.

**A file of a clash is read when a person opens it from the Bays panel.** `read_clash_file` (`GET /clashes/file?session_id=&other_kind=&other_id=&path=`, Bridge only) answers `ours`, the asking Job's change to that one path, and `theirs`, the other holder's, each what `git diff` prints from the merge base of the two sides' heads, with its `diff --git` header and `@@` hunks. For `main`, `theirs` is main's change since the Job's branch met it. A side that left the file as it was is an empty string. **Asked of git at the press and kept nowhere**: a patch is too large for a table that lives in memory. It reads the same worktrees through the same scratch indexes as the check, so an uncommitted or untracked write is in it and no index, branch or file of the Job is written, and the pair of worktrees it reads is one the check would name for that file (each side changed it itself, and git conflicts on it). It is refused with a 409 `fleet.clash_not_standing` where the pair no longer clashes or either has left the table, and a 422 `fleet.clash_path_not_in_clash` for a path the clash does not list. Neither is a 404, which is what a Fleet without the route answers; Bridge says so in a sentence and draws no diff. There is no size cap on the wire and Bridge draws the first 2,000 lines. `crates/fleet/src/clashing/file_read.rs`.

### The territory

**What the flight deck's map is drawn over** ([Cockpit](cockpit.md), *The map*): each served repository's directories, and where each agent has changed it. Two reads, both Bridge only, both git's and neither an agent's word.

| Read | Serves | From | Kept |
|---|---|---|---|
| `list_repository_areas`, `GET /repository_areas` | Each served repository's directories one and two names deep, with the files at or under each, and the first 100 file names of each directory that has no directory listed inside it | `git ls-tree -r --name-only` of the base branch's head, counted and named | Against the head it was read at: an unchanged head asks git for the head alone |
| `list_slot_touches`, `GET /slot_touches` | For each hosted Job holding a slot, the paths changed in it, by repository | The slot's tree as it stands, through a scratch index, against the tree of its merge base with the base branch | 20 seconds, as a whole |

**Areas are capped at the 40 largest** by files, counting each top directory and each directory inside one, and the answer says how many were left out. A top directory sorts before the directories inside it, so none is listed without its top. A file at the top of a repository is in no directory. What git does not track, and what `.gitignore` names, is not in the tree.

**A slot's changes are everything its branch holds that the base does not**: what it committed since it met the base, what is edited or deleted and not committed, and what is not tracked yet. What the base changed since the slot was cut is not the holder's. The slot's own index, branch and files are never written, which is why the read goes through a scratch copy of the index as *Write clashes* does. A holder is listed with at most 200 paths, sorted, and the count left out.

**Who holds a slot is `list_worktrees`' answer**, so these add no pool read. A slot held by a process Fleet does not host, a free one, a stranded one, and a subagent's slot are not read, and a holder with no change in its slot is absent. A slot git cannot read is left out of that read, not reported as unchanged.

**Memory only.** A restart reads again. Bridge reads both when Fleet connects, again 35 seconds after the board last changed, and again every 35 seconds while any holder has changed something, so a holder that stops goes. A Fleet with neither route answers 404 and Bridge takes it as none. `apps/desktop/src/renderer/src/fleet-territory.ts`.

### What a Session calls outside the repository

**A call that leaves the repository is one `session.called` entry on the timeline, one for each call and not a burst** (17 Oct 2026). A `gh` command, a page fetched, a web search, or a tool of an MCP server that leaves the machine. The entry carries `call.service` (`forge`, `tracker`, `web`, or the server's own lowercase name), `call.name` and `call.line`: the act (`gh pr view`, a page's host, `search`, a tool's name) and never an argument, a query, a path or a body. `crates/ipc/operations/get_fleet_timeline.toml` has the entry's fields; `docs/practices/protocol.md`, *The Fleet's timeline*, has how it is kept.

**Classification is the adapter's** (`crates/adapters/src/outside.rs`): the tool names are the harness's words and the commands the forge's, so Fleet carries the `Reach` it is handed and names no service. Armada's own servers and `gitnexus` never leave the machine and enter nothing. A `gh` command is `session.called` and not `session.ran`, so it is in no burst.

**A terminal Session's `gh` commands are seen, its MCP and web calls are not yet.** The mod tells a command through its `ran` fact (`docs/concepts/session.md`, *What the mod tells*) and has no fact for the others. The Cockpit draws what these entries hold as outposts ([Cockpit](cockpit.md), *Outposts*).

### What Fleet knows after the merge, and what it does not

**Armada opens pull requests, and the decision to merge stays a person's.** A person's press on a Job's pull request is Fleet's own act ([Job](session.md), *Acts on a pull request*); a merge made on the forge is found on a later read of main.

**The repository is fast-forwarded or left alone, and never anything in between.** A checkout on some other branch, a working tree carrying somebody's uncommitted change, a repository with no remote, and a history that will not fast-forward are all refusals — this is the one thing Fleet writes into the repository a person is standing in, and `--autostash` is not on offer because nobody asked for a rebase.

**It is also caught up whenever main moves, whoever merged.** The reading of main (*What Fleet knows about main's CI*) compares main's tip on the forge with the base the checkout has, in process (the checked-out commit on the base, the local base branch on any other), and fast-forwards on the same refusals when it lacks it. **A checkout on another branch is not a refusal here**: its local base branch is fast-forwarded with nothing checked out moving, and nobody is told. The owner is called only for uncommitted work on the base and for conflicts: a file main also changed to something else, or a history that will not fast-forward. **A file that is main's own content, byte for byte and in mode, is not a conflict** (a file deleted here and on main is the same): once the change an owner was keeping uncommitted has reached main by another road, the pull over it takes nothing from anyone, so where every uncommitted file is either untouched by main or main's own, and at least one is main's own, Fleet runs `git pull --ff-only --autostash` itself under the merge end, after reading the hold again, and raises no call. It is never run over a file that truly clashes, never forced, and a pull git refuses leaves the hold standing. Files main never touched alone stay the owner's call, with the card's Run or Pull request below. Jobs and the merge queue land most pull requests, and a catch-up that waited for a merge Fleet performed itself left the owner's checkout days behind in October 2026. A checkout it would not move is kept in memory with what holds it: the branch it is on where that is not the base, its uncommitted tracked files and which of them main also changed, and how far it is behind and ahead. It is served on the merge line's `hub` as `checkout`, and cleared once the checkout has main's tip.

**The one pull a held checkout is offered is Fleet's to run, and Fleet checks it again.** Where the checkout is on the base, carries uncommitted tracked files, none of which main also changed, and is ahead of nothing, `git pull --ff-only --autostash` cannot lose a file or stop on a conflict, and the card offers it. Run on that card is `catch_up_checkout`, which names the repository by its Manifest and no command: Fleet reads the hold again as it is now, refuses with a code and a sentence unless it is still that one, then runs the pull under the merge end, as its own catch-up does, and answers with git's exit code and words. The checkout is read again afterwards so the hub's `checkout` clears. Copy puts the same command on the clipboard, for a terminal. A stash git could not put back is not a success even though git exits clean for it: the answer is code 1, the files are in conflict, and the stash stays where git left it. Fleet never drops, pops or applies one.

**The uncommitted files can go up as a pull request instead, and the checkout is not touched.** Under the same gate as the pull (on the base, ahead of nothing, an uncommitted file, none that main also changed to something else), `checkout_to_pull_request` names the repository by its Manifest and no path. Fleet reads the hold again, refuses with the pull's own codes, and makes a commit from objects: its parent is the tip `origin` holds of the base, its tree is that tip's with each listed path's working-tree content (a file gone from the working tree is removed), and it is pushed to `armada/checkout-<hex>` and opened as a pull request titled by the path or `<n> files`, with auto-merge on. The working tree, the index and `HEAD` are only read, under the merge end so a pull's stash cannot hide a file from the commit. The commit itself refuses a path main changed to something else since the hold was read, and leaves out a path that already equals the tip. The answer is the pull request's number and whether the forge took auto-merge (a forge that refused it still has the pull request open). Fleet keeps the number in memory beside the hold and serves it as `checkout.pull_request` until the hold clears; the files stay listed, because here they are still uncommitted. A second press is `fleet.checkout_already_sent` (409). Once the pull request is on main, the catch-up above finds every file main's own and pulls by itself. A restart of Fleet forgets the number, and the hold then offers the press again until the pull request lands.

**An uncommitted file's change is read by Fleet, for a file the hold lists.** `checkout_diff` names the repository by its Manifest and a path, and answers the unified diff of that tracked file against HEAD in the served checkout (`git diff HEAD -- <path>`, the path taken literally, never as a pathspec). Fleet reads the hold it keeps from the sweep, the same list the card was drawn from, and refuses any other path: `fleet.checkout_not_held` (409) where there is no hold, `fleet.checkout_path_not_held` (409) for a path the hold does not list, `fleet.repository_not_served` (422), and a 500 `fleet.checkout_diff_unread` with git's words where git would not print it. A file that no longer differs answers an empty diff. **Fleet bounds the answer**: the first 2,000 lines and 256 KiB of it, whichever comes first, cut at a line end (`CHECKOUT_DIFF_LINES` and `CHECKOUT_DIFF_BYTES` in `crates/fleet/src/catching_up.rs`). A cut answer carries `cut: true`, and the card ends its diff with `First N lines`, N being the lines drawn. Git still prints the whole change into Fleet; the bound is on what is sent and drawn.

**A form's save to the Manifest never lands in that checkout.** It is committed from objects on top of `origin/<base>`, pushed to `armada/manifest-<id>` and opened as a pull request with auto-merge on, and the checkout's tree, index and `HEAD` are not read or written. Fleet keeps that edit in memory, one per repository: a second save commits on the same branch, the live keys and the reading a form is drawn from follow its text, a save of the raw file stacks on that branch too, as a commit of the whole text that must load as a form's edit must, and it is dropped once the checkout has its commit, or the forge reports the pull request merged and the checkout has main's tip (a squash or rebase merge makes a different commit, so its own commit never arrives), or the forge reports it closed unmerged, which the reading of main checks. A push whose pull request will not open deletes the branch it pushed. Where the forge would not turn auto-merge on, Fleet keeps that on the pending edit and marks the pull request's row on the hub `merge_by_hand`, which Bridge draws as a call, a glyph on the merge queue's chip and a line under the form's Save, each with Merge and Review pressed by repository and number (`merge_pull_request_by_number`, `review_pull_request`), since no Job holds the pull request; the mark goes when the forge stops listing the pull request as open. A tracked file only: an ignored or untracked `armada.yml`, a repository with no `origin` and a Manifest with no base are written to disk as before. An edit left uncommitted in the checkout was what blocked the catch-up on 14 Oct 2026.

### What Fleet knows about main's CI

**Fleet reads whether `main` is green or red on the forge's CI, for every repository it serves that names a `base:`.** It goes on the forge's facts, so it works on a repository whatever its CI: the failing job's name as the forge reports it, that job's log, and the merge that turned main red. A repository whose Manifest names no base is not read, since Fleet would be guessing which branch main is.

**It adds Check meaning only where a CI job maps to a Check.** A job maps when its name is a Check's name, or when that Check lists it under `ci_jobs` ([Configuration](../contracts/configuration.md), *Which CI jobs a Check answers for*). A job that maps to nothing is the ordinary case and is kept under the forge's name alone. A failing test is read out of the job's log by the same nextest and vitest reading the gate uses, and where the log names none there is no test.

| What Fleet asks | When |
|---|---|
| Where main stands on the forge | One ref lookup per repository per minute, one repository a reading, rotating. The reading has a loop of its own, so a Job's Checks never hold it up |
| Whether the checkout has main's tip | Every reading, in process. A `git pull --ff-only` only where it lacks it, and one `git fetch` of the base when that is refused |
| The jobs that ran on that commit | When the head has moved, and again while any job has not finished. A settled commit is not asked again |
| A failed job's log, its last 256 KiB | Once per failed job per commit |
| The pull request that merged the commit | Once per commit while this process lives, and only for a red |
| The repository's open pull requests, each with its `ci` | One listing a visit, the newest 100, on the same loop and rotation |
| The pull requests merged into the base | One listing a visit, beside the open ones, the newest 30. Recently landed shows five |
| The CI run on the head and the newest five merge commits, and on older ones only until one has finished | One ask a commit, then again only while a job is unfinished, or while nothing has run on a merge under ten minutes old; a settled run is kept in memory and not asked again. Five asks the first visit after a start, usually none after |

**Main's state is the newest commit whose run has finished.** Fleet reads the run on the head and on each of the newest merged pull requests' commits, in first-parent order and back as far as the first one that has finished, and the first one that is red or green decides. Newer commits still running do not: a green on an intermediate commit clears a red even while later ones run, and says so as `checking` on a green. A red's failed jobs are the deciding commit's, **one entry a job name**: a commit the merge queue ran and main then ran again has each job twice, and the forge adapter keeps the newest run of each, so a job the queue's run failed and main's run passed is not a failure, and "Broke in" names the oldest red commit before the last green that failed what the deciding one fails. A commit nothing ran on proves nothing either way, and where nothing in the window has finished a known red is kept, held.

**A red is held while a newer commit's CI is running.** A fix may already be in that run, so Fleet keeps the red's facts and the commit they were read at (`red_commit`), names the running commits and their pull requests (`checking`), refuses `fix_main` with `fleet.main_checks_running`, and does not pick the red up. When the run ends green everything clears. When it ends red on the jobs the red already had, the red is back and the pickup runs; on a job it did not have, it is a new red naming the newer merge.

**The reading is kept, so a restart loses nothing.** One row per repository holds the commit, green, red or running, when Fleet first read it red, the failed jobs, and the merge. The merge is the pull request's number and the Job that held the pull request or its branch in the ledger, in any state, given back included; a direct push has neither, and the forge's silence leaves both empty rather than guessed. When main goes red or green again Fleet says so on the turn it read it (`Turned::main_changed`), and acts on it in the same turn: a green ends every take of it. A red whose pull request a Job merged is claimed for that Job and the Job is told, or, if it cannot be, for a fork of it, and the take is recorded so the hub names it and no second fix is started while it holds. `docs/capabilities/merge-line.md`, *When main goes red* and *When a Job's pull request turns main red*, have the ways a red is taken.

**It is served as the merge line's `hub`**, protocol 23.41, 23.42 and 23.44, with the open pull requests: a pull request is `waiting_on_main` when every check that failed on it also fails on main, and its own failure otherwise. `docs/capabilities/merge-line.md`, *The hub*, has the shape and what Bridge draws. The open pull requests are held in memory, listed again on a repository's next visit after a restart. A failed job's log is not kept: it is asked of the forge when a person presses the job, through the merge line's Check log.

**A forge that does not answer keeps the last reading, and the hub says how old it is.** Each repository's visit makes five asks: the head, the open pull requests, the merge queue, the merged listing and the runs on the commits it decides main from. `forge_read_at` is the instant of the last visit in which all five were answered, which `main.read_at` is not: that moves only when the head does. `quiet_since` is the instant of the first visit in which any of them was not, kept until a visit is answered whole, and absent while the forge answers. Both are in memory and a restart forgets them. A repository whose forge has never answered since the start is served as a hub holding `quiet_since` alone. A log or a culprit asked only after main changes is not one of the five. Every visit moves `forge_read_at`, so the merge line is published once a visit.

### Telling the owner of a pull request

**Nobody watches a pull request for failures. Fleet reads each open one once an interval and tells its owner when any of these happens.** One call a repository returns, for each open pull request, its checks on the newest commit with their log addresses, whether it conflicts with the base, and where the merge queue holds it. The merged pull requests are the listing already read for the hub.

| Happened | Fires when |
|---|---|
| Checks failed | The check that gates the pull request (`ci` where there is one, else every check) failed on the newest commit |
| Conflicts | The forge says it cannot merge into the base |
| Left the queue | It was in the merge queue at the last reading and is not now |
| Unmergeable | The merge queue marks its entry unmergeable |
| Merged | It merged in the last six hours |

**A held pull request is read by its number when the list leaves it out (decided for #2459, 16 Oct 2026).** Each list stops at 100 (the watch's the 100 updated last, the hub's the 100 created last), and a repository can have more open. So after each list Fleet reads, by number and one call each, every pull request that has a standing claim in the repository and is not in it: a standing `pr` row of a Job. It is added to the list as one entry, so a pull request in both is told once, and the hub shows it in Bridge. A pull request nobody holds is never read by number: paging every open one costs two or three calls a round to watch pull requests nobody owns.

**A job that hung is not a failure.** One that ended cancelled after about the whole 45 minutes a job is given is started again once for that commit, through the forge adapter, and nothing is said. Hung again, it is told like any failure.

**Each notice is told once, and a restart does not repeat it.** The store keeps (pull request, commit, cause, recipient) and the merge queue's last reading. A new commit that fails is news; the same one read again is not. **A recipient that cannot be told now is asked again at the next reading** and nothing is kept for it: a terminal Job whose mod is not asking.

**An owner is the Job whose `pr` row holds the pull request's number, else the Job holding its branch.** `docs/concepts/session.md`, *What Fleet tells a Job*, says what it receives.

**A Job counts only where the pool agrees (decided for #2239, 13 Oct 2026).** Among the Jobs, the owner is the one the pool confirms works the branch: it stands in a slot whose lease is its own, the branch is checked out there, and the branch row stands. Where several are confirmed, the one whose `pr` row holds the number. Where none is, the Job whose standing `pr` row is oldest, else the one whose standing `branch` row is. One Job, not every row: a Job that read the pull request from a directory in a slot it had left was told its checks failed while another Job worked it. `docs/concepts/session.md`, *Whose slot it is*.

### Whose pull requests Fleet reads

**Fleet tells, checks and draws a call for the person's own pull requests, and for no one else's.** Mine is the pull request's author login equalling, in any case, either the GitHub login Armada is signed in as for the repository (the profile bound to it, `SigningIn::login_for`) or the login the forge's command line is signed in as on this machine (`gh api user`, asked until it answers and then kept). **Where neither login is known every pull request is mine**, and so is one whose forge named no author: a rule that cannot be read does not go quiet. Fleet puts the result on each open pull request in the hub as `mine`, absent meaning mine.

| Pull request | Merge line | Call cards | Messages to owners, merged notices | Timeline |
|---|---|---|---|---|
| Mine | As ever | As ever | As ever | As ever |
| Not mine, in the merge queue | Shown quietly: its place, a neutral tone, never the ask or bad tone | None | None | None |
| Not mine, not in the queue | Not shown | None | None | None |

**The queue is where a team shares a line, so it stays whole.** Someone else's pull request in the queue is a fact about when yours lands. Outside it, it is theirs to watch. The pull watch reads every open pull request with its author on one call, so the rule costs no second ask.

### The pull requests waiting on a person

**`get_review_lists` (`GET /review_lists`) reads four lists from the forge when somebody looks, and nothing polls them.** Fleet keeps no copy of other people's pull requests, so a reply is as old as the forge's answer, and Bridge reads again when the Reviews tab opens and on Refresh. The lists are review asked of the person by name, asked of a team they are on and not of them by name, assigned to them, and opened by them. Every one is searched for the person's login, the first of *Whose pull requests Fleet reads*, across every served repository at once: **one forge call a list, whoever opened the pull request and whether or not a Job holds it**. A row of a repository Fleet does not serve is dropped. A pull request on several lists is a row on each, and a team row names the first team asked that the person is on.

**A list is capped at 50, newest-updated first, and a list at 50 has more behind it.** A list the forge would not answer is empty and named in `silent`, so one failure does not blank the others and Bridge keeps what it last held for that list. All four silent, or no login known, is a 500 `fleet.review_lists_unreadable`.

**`resolve_review_thread` (`POST /review_threads/:repository`) resolves a conversation on a diff, or opens it again.** It is the forge's own act, taken at once and seen by everyone watching, so it is a person's press and never part of the pending review. A 409 `fleet.review_thread_refused` carries the forge's sentence, and a repository Fleet does not serve is a 422.

**A page carries its `head`, its `body` and its `threads`, and a write with `comments` is one review.** `get_pull_request_detail` reads the threads in a second forge call, the first 100 with their comments; a page whose threads the forge would not give is a 500 and not a page without them. `write_pull_request` with `comments` opens a pending review at the commit the person read, adds each line, file and reply comment, and submits it with the verdict, so the forge sends one notification. A comment the forge refuses deletes the pending review and posts nothing: a 409 with the forge's sentence.

### Agent runs about a pull request

`docs/concepts/pull-review.md`. **`start_review_run` (`POST /review_runs/:repository`) asks an agent, read-only, about one pull request, and answers at once.** The reply is a kept result as it stands, the run already going for the same key, or a new run still `queued` whose work goes on a task of its own, so a caller that stops waiting does not stop the run. `get_review_runs` reads every run kept or going for a pull request, and `review_run.changed` carries a run whole each time its state moves. A name no definition has, of the wrong kind, is a 422 `fleet.review_unknown_tool`, and a blank question a 422 `fleet.review_blank`.

**One engine serves every kind.** It reads the definition (repository over Kit over Armada) or takes the question, reads the pull request's title, description, head and diff from the forge, leases a bay of the repository's pool with the head fetched from the forge's ref and checked out on a branch of the lease's own, makes one model call with the read tools only (`Reading::checkout` on the bay: no shell, no edit, no network), and gives the bay back, forced clean, whatever happened. The call is the Judge's client on `models.review` and the proposer's wait, since a run reads a repository and writes a long answer. **A bay at a different commit than the one the forge named at the press ends the run unread**, since a result is kept for the commit it read.

**A result is kept per commit in Fleet's store** (`review_runs`), keyed by manifest, pull request, head, kind and name, with the cost the vendor billed. The cost is the run's own. A failed run is kept with its reason and is never answered as a result. Rows the store says are going that no task of this process holds are ended failed, and their bay given back, the first time anyone asks about that pull request.

**Definitions are read from the folders every time** (`list_review_tools`), so what a person saved is what the next press asks. `save_review_tool` writes one file into Kit's folder or the repository's `.armada/review-tools/`, refusing a blank name or prompt, a name with no words, and an id the scope already defines unless asked to overwrite. It is `Helm only` for agents: what is written decides what every review asks an agent.

**Each listed row names the Job that holds the pull request**, from *Who owns a pull request*, so Bridge can send a note there.

### Telling jobs the build they now run

**One person restarts Fleet and Bridge, from one job, and every other job that asked for the same restart still thinks it is waiting.** Fleet reads the build on file once an interval and, when it is a commit this process has not settled, tells the jobs that care. The commit is `restart-commit`, which `scripts/restart` writes after the new Fleet answers, so it is read on the sweep and never at boot. Nothing is said while `restart-build.status` says `running`.

| A live job in a served repository | Told |
|---|---|
| Waits on the person to run `scripts/preview --restart` or `scripts/restart --main`, and that is the build now running | The build, and for each branch it holds whether it is in it, as the answer to that wait, so it wakes once |
| Holds a branch the build newly holds, holds newer commits of, or dropped after it was told it was in | The build, and those branches only |
| Anything else: a branch where it was at the last notice, a branch git cannot place in this build, a wait for the other build | Nothing |

**A rebuild that leaves a job's branches where they were says nothing.** Every branch-holder was told on every rebuild until 15 Oct 2026, and the notice arrives as input, so each job spent a turn answering "no action needed", in other repositories' jobs too. What each job was last told of each branch is kept in `build_branch_notices`: the newest commit of it the build held, or `out`.

**The command is read as Bridge reads it**: either script by any path, its own flag, and `--adopt` in any order. Anything else, `--dry-run` included, is a shell run and not a restart. A wait for the other build than the one now running is left standing. A second wait for the same build is answered `Restarted: preview at 1234567`.

**A branch is in the build, not in it, or in it up to a commit.** Counted from the branch's own commits past `origin/main`, so a newer main merged into it is not counted as its own: all of them in the build, none, or some, naming the newest the build holds and how many since it are not. `main` and a branch with nothing of its own are not mentioned.

**Each notice is told once per build, and a restart does not repeat it.** The store keeps (build commit, job). A job that cannot be told now is asked again at the next reading and nothing is kept for it, and its wait stands until it is.

### Restarting Fleet

**Restarting Fleet is a `launchctl` call from Bridge, not an API command.** `restart_fleet` cannot be served by the process being restarted. Bridge already owns bootstrapping the launchd job, so it owns restarting it, and the operation is a `child_process` call rather than a protocol operation. It is the Restart Fleet button on the notice that says Fleet and Bridge do not match: `kickstart -k` restarts the job onto whatever `armada` is installed, hosted Jobs are reattached at boot, and a Fleet whose pid launchd does not hold is refused rather than doubled.

**Restarting onto another build is an API command, and the restart is not Fleet's.** `change_fleet_build` starts `scripts/restart-build` detached from Fleet and answers 202; `get_fleet_build` reads the outcome from the files the scripts leave. The Fleet panel's Select and button are it (`docs/practices/running-locally.md`, *Restarting from Bridge*). It is a different act from the one above: that one restarts the installed binary and builds nothing, and this one builds `main` or the preview and reopens Bridge.

**`kickstart -k` does not bypass the throttle.** Issued inside a live crash-restart window it returned immediately, but the new instance took 19.0 s to appear at the 10 s default. `ThrottleInterval` 2 brings that to 2.6 s.

## Worktree slots

**A repository keeps a pool of permanent, warm worktrees and leases them out**,
to agents, so a build starts from the last one's
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
| Lease | Takes the first free slot and points it at the branch. **A branch that exists here, or on `origin`, is checked out at its own tip and never moved**; a name nothing has is cut from the freshly fetched base with no upstream. Removes everything untracked except `target`, `node_modules`, `.gitnexus` and whatever `setup.seed.paths` names. A slot made for the first time has `setup.seed.paths` cloned into it, copy-on-write where the volume does it |
| Release | `armada worktree release` is refused while the tree has anything uncommitted, or HEAD has commits the lease's branch, the remote and the base all lack. A commit on the branch is enough and a push is optional: HEAD is detached where it stands, the branch keeps its commits, and the build stays |
| Merged | When the pull request from a branch a slot is leased on merges, Fleet tells the Job holding the slot one line, that the slot is free to lease for new work or give back. **When that Job's turn then ends with nothing queued, Fleet gives the slot back** if HEAD is still the merged branch, or detached on a commit the base has, the tree is clean and every commit is on a remote or the base. Another branch checked out is new work, and the slot stays. A dirty tree or commits on no remote or base keep it too, and the holder is told once for each, naming it. The branch is never deleted. Told once, and kept in the store, so a restart repeats nothing. Only a hosted Job's slot is given back; a terminal Job is told and gives its own back |
| Park | Commits everything uncommitted, untracked files included and ignored ones not, to the slot's branch under a `WIP:` message, then releases. Never pushes. A clean slot is only released. Never refused for where HEAD points: a checkout on another branch than the lease names is committed there, and one on no branch or on the base, holding files or commits no branch has, gets a `kept/slot-<n>-<holder>` branch cut at HEAD (a number after it where the name is taken); a clean one holding nothing is only released. Refused while a take or release is under way. It answers with the commit and the paths it took |
| Lease an existing branch | Puts a slot on a branch that exists, at its tip, so work parked there continues. Waits while every slot is held, as Lease does. Refused for a branch that does not exist here or on `origin`, and for one another checkout already has, naming where. A slot whose holder is gone and which is clean is taken back for its own branch; one holding uncommitted files is not. `armada worktree lease --existing <branch>` |
| Status | Every slot, the branch checked out in it, who holds it and for how long. **The branch is the one git has checked out, not the one the lease was cut for**: where a job moved off it, both are said (`follow-up (leased as feature)`), and `detached` stands for a checkout on no branch. Bridge's Cleanup draws the same reading as one tile per slot, with whether every `setup.seed.paths` entry is on disk in it (warm) and how many commits the base has that it does not |
| Add | One more slot, numbered lowest-unused and not made until a lease makes it |
| Remove | The slot named, and only a free or unmade one, never the last. A made one's checkout goes by `git worktree remove`, which refuses one holding anything uncommitted; held, stranded and busy slots are refused by name. The other slots keep their numbers |
| Rescue | On a stranded slot only, on a person's press or on Fleet's own where the last holder is gone: starts a Scout that reads the slot's checkout, and keeps its Finding against the slot. Then the person chooses Scrap or Stash below |
| Scrap | Discards the uncommitted files, puts the checkout back at the base and frees the slot. Bridge's confirm names the uncommitted files and the commits that exist only in the slot. The branch is deleted only where the base holds every commit on it; otherwise it is kept, and the answer says so |
| Stash | Commits the uncommitted files to the slot's branch, pushes the branch to `origin` under its own name, and frees the slot. Refused on a checkout on no branch or on the base, and where there is no `origin` |
| Close, open | A closed slot is never leased until it is opened. A holder keeps one closed under it until its lease ends, and it stays closed after |

**Bridge's Cleanup list is kept from file events, with a deadline.** Fleet keeps the list and
reads it again behind Bridge's asking: for the slots the file system named (a slot's git
directory, its lease record, the base branch's ref, the work tree of a free slot, never a held
slot's, whose builds write thousands of events a minute), and for every slot every 10 seconds.
**The list is never wrong for longer than 10 seconds**, which is the owner's bound (15 Oct 2026);
a change an event announces shows in under three. What no event announces is a holder dying,
another process changing the pool's size, and a branch deleted from under a free slot's commits.

**A lease takes the slot's branch first and writes its record last** (16 Oct 2026). A made slot
read in between has files and no record, so it would read as stranded with no holder: a reading
made while a take or a release holds the slot's lock says busy instead. The record is watched so
the read after it is corrected, and a slot lease or release Fleet enters on the timeline is
answered from a read begun after it: Bridge reads the list again on that fact and on nothing
else, so a list from before it would stay on the screen.

**The 10-second read asks `git status` of a free slot only where the watch cannot vouch for
the last answer** (16 Oct 2026). `git status` was the expensive part of it: about four a minute
per repository here, and up to 71% of a core each in `lendinghome-monolith`. HEAD, the refs and
the record are read in process every time. A free slot's status is asked again where its work
tree was not watched before this read, or its watch heard it, or events were lost (an error, or
a rescan flag), or the stream restarted (a slot was added to it or let go of, and the events of
that moment are the ones it drops), or something changed under `target`, `.gitnexus` or
`node_modules` that the repository does not ignore, or the repository's `info/exclude` changed,
or this adapter acted on the pool. A pool the file system would not be watched for asks every
time. Nothing is asked on a timer beyond these, so a watch that silently stopped reporting would
leave a free slot reading as it did until one of them; the global ignore file is not watched.
`docs/spikes/032-what-moves-when-a-slot-changes.md` has the measurements.

**An agent's lease is held for a process, recorded beside the slot as its pid
and start time.** The command that leases exits at once, so a lock held open
could not be the holder; the `flock` on `slot-<n>.lease` only makes one take or
release at a time. The holder is the first process above the command that is not a shell
— an agent's job, or the terminal a person typed in — so a subshell or a
pipeline between the two does not become the holder. **A job Fleet hosts is
the exception**: Fleet ends and resumes its process when it moves slots, so
`armada worktree lease` there holds for the job's id, read from
`ARMADA_SESSION`, and is given back when the job closes. **A terminal agent's lease also records the agent job it ran under**, read from the environment the agent sets in each command it runs, so the pool says which Job holds a slot a process holds. A lease taken by hand, or before this was written, names none.

**Every lease and release is also written to the slot's history**, `slot-<n>.history` beside the record, by the pool in whichever process writes or clears the record, so one an agent took with `armada worktree lease` is in it though Fleet never heard. It is kept thirty days, and `get_slot_history` serves it with the checkouts the slot's own `HEAD` reflog holds (`docs/practices/protocol.md`, *A bay's history*).

> **Rule.** A slot whose holder is gone is taken back only when its tree is
> clean and every commit on it is on its branch, the remote or the base. Otherwise it stays held, and a lease
> waiting for a slot names it.
> Why: a slot is reused, and reuse must never be what throws work away.

> **Rule.** A lease never moves a branch that exists, here or on `origin`,
> away from its work: it checks the branch out at its own tip, and only a name
> nothing has is cut from the base. **The one move is forward**: the lease
> fetches the base first, and a branch whose tip the base already holds is
> fast-forwarded to it, so an agent never has to update a slot it was leased.
> Why: resetting a branch to the base orphans its commits, and on 13 Oct 2026
> a lease did exactly that to a pushed branch in a slot whose holder was
> gone. It survived only because it was pushed.

**A person's changes to the pool are this machine's, and outlive Fleet.** Add,
remove, close and open write `.armada/slots/pool` beside the slots' own
records, which is never committed; `armada worktree lease` and Fleet both read
it. Once a slot is added or removed, that list stands in for `setup.worktrees`,
which stays the size a fresh machine starts at. Bridge's Cleanup offers each act
in the panel of the slot's tile, and `change_slot_pool` is the act on the wire.

### Cleaning up from the grid

**Cleanup is one grid of tiles, and a press on a tile opens the panel that manages that worktree.** A bay is a tile. There is no list below the grid and no act on more than one tile: `armada clean --everything` is the one bulk act, and nothing on a screen reaches for it.

| In the panel | Offered |
|---|---|
| What it holds | One row per reason Fleet holds the worktree, in git's words: uncommitted changes, unmerged commits with what they are not on and their tip, a locked worktree, an unknown base branch, a failed `git status`. A worktree holding nothing shows no row |
| Release | On a slot an agent job holds, which offered only Close slot before. The confirm names the holder, with its pid on a tooltip, and the branch, then commits the uncommitted files to the branch as a WIP commit, releases the slot and keeps the branch (a detached or base checkout gets a `kept/` one, and the receipt names it). The holder shown is sent, and a slot re-leased since is refused with `fleet.slot_holder_changed`. The job's process is not touched. **A bay a Job Fleet hosts holds is released the same way, with the Job's id as the holder** (`HostedSession` in the bay's reading): Fleet parks it as a close does, which ends everything still running in the slot, so a Job whose turn is running is refused with `fleet.slot_session_working` (*Pull Request Panel Design is working*) and nothing is ended. An idle Job's process is let go as a tune lets it go, its slot and branch rows on the ledger are given back, and its next message resumes it without a bay. A Job Fleet does not host is not this path |
| Close, Reopen, Remove, Rescue | A bay's own, as above. Its Finding, and Scrap, Stash and Pick up, are in the same panel |

**What an act did, and what Fleet refused, is said in the panel and never on the tile.**
A tile says its state and its figures, each named on hover.

### Rescuing a stranded slot

**A stranded slot is work its holder left and nobody has looked at.** Rescue
sends a [Scout](scout.md) to read it, and the bay offers a way into what it
found, which opens in Bridge's trailing sheet. Scrap, Stash and Pick up are in that
sheet, and are Fleet's acts, run on the press that asks. The Scout has no tool
that writes, so it never does any of them.

> **Rule.** A Scout reads a stranded slot on a person's press, and on Fleet's own
> once a stranding where the last holder is gone and none could be told. Its
> Finding is kept against the slot, not a Studio.
> Why: a stranded slot's holder is gone, so nothing else owns the Finding, and
> it has to survive a Bridge reload to be there when the person decides. The
> owner decided on 13 Oct 2026 that a gone holder is not waited on for a press.

> **Rule.** A stranded slot's last holder is told once, when it is a Job
> that is still open, and no Scout reads the slot over it.
> Why: that Job knows the work and can commit, push and release it, which
> a Scout's reading cannot.

**The last holder is the Job the slot was held for.** A Job's lease names it by id. A slot a
process held names the Job whose process tree that process was in, which
Fleet wrote on the lease record while the process lived. A holder the record
cannot name, as a terminal Fleet does not host, leaves none.

**Gone is every holder that is not an open Job.** That is a Job that
ended, a Job Fleet no longer holds and a slot with no last holder. A Job
Fleet cannot reach counts as gone.

> **Rule.** Fleet acts on a stranding once, and a Fleet restart does not repeat
> it.
> Why: a second message to a Job that is already finishing, or a second
> Scout for the same work, spends a turn and decides nothing.

**One Scout reads at a time.** Fleet starts the next stranded slot's Scout when
the one reading ends, so a Fleet that comes up on several does not start a model call
for each at once. What Fleet did is kept in `slot_strandings`, one row a slot for the
stranding it names (its branch and when its lease began), and the bay shows when its
holder was told.

> **Rule.** No act runs while a Scout is reading the slot, and a Finding
> is of the commit the slot was at. A slot that has moved off it shows none.
> Why: the Finding describes one state of the work, and an act on another
> would be decided on a description of something else.

**Each commit on the slot's branch says where else it exists.** Fleet asks git per commit: on a
remote branch (`git branch -r --contains`), else on the local base (`git merge-base
--is-ancestor`), else only here. The Finding lists the ones that exist only here first, and the
Scrap's confirm names those.

**Pick up starts a Job on the work.** It stashes as Stash does, which commits the
uncommitted files to the slot's branch and pushes it, and frees the slot. Fleet then
starts a Job titled `Continue <branch>` in the repository and sends it a first
message of the branch and the Finding's items, in the voice of Rescue.

> **Rule.** Pick up is offered on an Unfinished Finding, or one with no verdict, and
> Fleet refuses it on Scraps.
> Why: leftovers have nothing to continue, and a Finding with no verdict is one the
> person cannot tell.

`armada worktree` and its forms are in `../practices/running-locally.md`,
*Leasing a worktree*.

## Ports

Each repository's main checkout claims a contiguous span of ports, held while Fleet runs; servers draw from it. Fleet's own listener claims a single port the same way, out of the same range.

**A workspace Verify claims a span of its own, sized from that workspace's `armada.yml`, for the life of the Verify.** Its commands resolve `${port.NAME}` against that span laid over the main checkout's: a name the root alone declares is the root's number, and a name both declare is the workspace's. The span is kept under a key naming the workspace's directory that no repository's own key can equal, so a repository served at that same directory keeps its span through the Verify, and a row a crashed Verify left is reconciled at boot, released at shutdown, and replaced by the next Verify of that workspace.

> Why its own span, not the root's grown: a span is contiguous, so growing one re-picks it, and that moves a port a server in the main checkout is already bound to.

### The range

`Port range base` and `Port range ceiling` are settings. The ceiling is **detected at daemon start from the platform's ephemeral port floor, minus one** — `net.inet.ip.portrange.first` via sysctl on macOS, `net.ipv4.ip_local_port_range` in `/proc` on Linux — with `32767` as a fallback if the read fails rather than as the value.

A constant stores the answer instead of the rule. `32767` is one below Linux's ephemeral floor; on macOS, whose floor is 49152, it reserves roughly sixteen thousand ports the only supported platform will never use.

**Never guess high.** A ceiling above the real floor hands out spans the kernel will also assign, which is the collision class the ceiling exists to prevent. Detection supplies the default; an explicit setting still wins, so a lower cap survives and Doctor has a value to show.

This is the first place Armada reads a kernel parameter. Where that dependency belongs is tracked in `../contracts/adapters.md`.

### Claiming

A claim names a checkout or Fleet's own listener, so who holds which span is a query rather than an inspection of directories.

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

Fleet holds a Manifest's server Commands — the ones with `serve`, which [Manifest](manifest.md) defines. A person starts one from Bridge; a Job starts one through Fleet's MCP tool.

> **Rule.** Fleet runs at most one instance of each server Command per checkout, and hands a second request the running one.
> Why: the agent that asks second must not start another on the same port.

A server runs in a checkout of the repository `?manifest_id=` names — its main checkout, or a worktree of it that `start_server`'s `checkout` names — and uses **that checkout's own span**. It stops on Stop, when it exits, or when Fleet stops; the span itself is released only when Fleet stops, after teardown.

> **Rule.** A port span is claimed per checkout, never per repository.
> Why: a repository has more than one checkout and each declares the same `ports:`, so one span between them is the number two servers bind. On 22 September 2026 a worktree took the main checkout's port and the address kept answering, with another branch's app behind it.
**Fleet holds its servers in memory, so they stop when Fleet stops too.** A restarted Fleet holds none, and a server that outlived the process holding it would keep its port with nothing left to hand it to the next Job or stop it. Beside each running server's log Fleet keeps a record of its process group, removed when the server stops.

> **Rule.** After a crash, Fleet kills the servers it left running when it next starts, confirming each is still the process it recorded.
> Why: a leftover server holds its port with nothing to stop it, and a pid the system has since given another process must never be killed.

#### Which checkout answers, and how far behind it is

Every answer that names a server says which checkout serves it — its path, and its branch where Fleet can read one. A name and a port are not enough: another checkout of the same repository declares the same number, and before each took a span of its own the address answered whichever bound first. On 22 September 2026 five agents were told to watch one address and one of them photographed another branch's app.

> **Rule.** A held server carries the checkout it runs in on every answer and every event.
> Why: the failure is not the collision — it is that the address keeps answering, so nobody looks.

When Fleet brings the main checkout forward, every server held on that checkout is told how many commits landed under it, and says so.

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

## Where Fleet's behavior is actually documented

Each piece of Fleet's behavior lives where it is specified:

- Memory and disk resource gating before a Check — Limits, above
- A Job's state, ledger rows, plan and rail — [Job](session.md)
- A Judge's reading of a Job, on the person's press — [Judge](judge.md); a workflow's steps as a soft rail — [Workflow](workflow.md)
- Secrets brokering (an agent never holds secrets directly) — [Kit](kit.md), [Manifest](manifest.md), Secrets
- VCS push, pull request and merge, the sole actor touching Git credentials — [Job](session.md), *Acts on a pull request*
- Schema migrations applied on startup — `../contracts/system-architecture.md` section 5, [Kit](kit.md), Upgrade
- Structured JSON logging — `../contracts/system-architecture.md` section 4
- Own health status — [Doctor](doctor.md), Fleet module
- What a repo may declare about ports, Check timeouts and teardown — [Manifest](manifest.md)

## Open questions

- **[fleet-checks-runner-sweep-timing]** Does the sweep for orphaned process groups left behind by a dead `checks-runner` run at Fleet start, on a timer, or both? If Fleet stays up while `checks-runner` dies, nothing sweeps until the next restart under the current design.
- **[fleet-time-limits-on-a-loaded-machine]** Should Fleet's time limits allow for a machine it no longer protects from CPU load? Headroom leaves CPU to the operating system, so a Check can start on a machine other work has saturated and run slower than the limits were chosen against. The Check budget and the command budget are fixed durations, and neither reads load, so a slow run and a stuck one look the same to them. Nothing has measured a Check under a loaded machine. What decides it: whether a Check is ever stopped for time while its work is still moving, and whether the answer is longer limits or limits that stretch with load.
- **[fleet-port-range-degradation-visibility]** How does a person learn the port range is losing spans? An undeclared supervisor-held process holds its port past release, the probe refuses that span, and the range loses it — nothing currently surfaces that degradation.
- **[fleet-self-restart-limit-setting]** What happens to the *Fleet self-restart attempt limit/backoff* setting, given that launchd cannot implement a cap or a backoff curve? The setting describes behavior the supervisor does not offer.
