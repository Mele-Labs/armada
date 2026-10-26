# A repository's app, run from Bridge: `list_app_runs`, `start_app_run`, `app_run.changed` and six more

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**Additive only**: eight operations, one event kind, and the types they carry.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/app-runs.ts` and `events.ts`, written by hand.
Fleet's half is `crates/fleet/src/app_runs/serving.rs`, which converts what `crates/fleet/src/app_runs.rs` reads into `ipc`'s own types.
All nine are `Bridge only`: ▶ is a person's press, and an agent that wants a server already has `start_server`.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_app_runs` (`GET /app_runs?manifest_id=` or `?job_id=`) | `AppRunList`: one `AppRun` per run the file declares | Main checkout, or the worktree a Job holds. Runs that are not up are in it |
| `get_app_run_draft` (`GET /app_runs/draft`) | `AppRunDraft`: `root`, `place`, `processes`, `links?`, `setup?` | Saves nothing but the links it finds. Answered where runs are saved too: `processes` is only what none of them starts, and empty is a draft |
| `start_app_run` (`POST /app_runs/start`) | `NamedAppRun`; answers `AppRun`, 202 | The run already up is answered as it stands. A checkout that needs setting up answers at once with `setup` setting up |
| `stop_app_run` (`POST /app_runs/stop`) | `NamedAppRun`; answers `AppRun` | Each process waited for |
| `restart_app_run` (`POST /app_runs/restart`) | `NamedAppRun`; answers `AppRun`, 202 | Every process is a new instance |
| `save_app_run` (`POST /app_runs/save`) | `SaveAppRun` with `setup?` and `replaces?`; answers `SavedAppRun`, 202 | Written into Kit's copy, then started. A run Kit's copy already holds is replaced |
| `pin_app_run_port` (`POST /app_runs/pin`) | `PinAppRunPort`; answers `AppRun` | `fixed` on the process's port in Kit's copy. Nothing is restarted |
| `unpin_app_run_port` (`POST /app_runs/unpin`) | `UnpinAppRunPort`; answers `AppRun` | Back to leasing, and the links to the process end |
| `app_run.changed` (event) | `AppRun`, whole | After each `server.*` fact of a server a run lists, after a pin, when a link is found, and as the checkout's setup starts, moves to its next Command and ends |

**A run is its servers and nothing more.** A process is a `ServerState` read through, so its output is `observe_server`'s and is addressed by the process's `server_id`.
No second socket carries a run's log, and `app_run.changed` carries no line of one.
A restart is a new instance, so a viewer reads `server_id` again from the next event.
`AppRunProcess.log_path` is the absolute path of the instance's output file, so a Job handed it to fix a failure reads the whole of it.

**A run waits on its checkout's setup**, the Manifest's `setup.requires`, where the checkout is new to it or the files it was traced from changed.
`AppRun.setup` is `{id, log_path, state, commands, running?, failed?, ended?}`, and `state` is `setting_up`, `ready` or `failed`.
While it is `setting_up` no process has started, and `running` names the Command running now.
A Command that fails starts no process: `failed` names it and `ended` says how, the Command's line and the tail of what it printed.
`failed` with no `failed` Command is a run that would not start once setup passed, and `ended` says why.
`setup.id` is what `observe_server` takes, so setup's output is read as a process's is, under the name `setup`, and `setup.log_path` is its file, absolute.
Each setup is a new id. `setup` is absent where this Fleet has run none for the run on that checkout, which a checkout already set up never needs.
Stopping the run stops its setup.
Fleet keeps a mark per checkout under `.armada/setup/` beside the servers' logs, never in the tree: a fingerprint of the Commands and of the lockfiles and package manifests at the root and in each folder a Command names, and the paths the setup made.
A checkout is set up while the fingerprint holds and each path it made is still there, so a slot cleaned for its next lease is set up again.

**A draft proposes setup.** `AppRunDraft.setup` and `SaveAppRun.setup` are `{name, command}` each, run from the checkout's root in order.
A lockfile beside a `package.json` proposes that folder's install (`npm ci --prefix`, `pnpm install --frozen-lockfile --dir`, `yarn --cwd … install --frozen-lockfile`, `bun install --frozen-lockfile --cwd`), and a `requirements.txt` or `pyproject.toml` proposes a `.venv` made beside it and filled.
A setup Command takes no `dir`, so the folder is a flag or a path.
One whose line or name a Command already declares is not proposed again.
Saving writes each as a Command and adds it to `setup.requires` in Kit's copy, after what that copy lists.
Where Kit's copy lies over the repository's own `armada.yml`, Kit's `setup.requires` follows the repository's, each name once, and a copy that is not whole holds nothing else under `setup`.
A remembered command is drafted only where every path it names, its program or a file it reads, is tracked in the checkout or made by a proposed setup Command.
Every drafted process listens on a leased port named after its folder.

**A saved run is answered whole, so Edit the run reopens the confirm with it.**

| Field | Holds |
| --- | --- |
| `AppRunProcess.command` | What starts it, with `${port}` for its own port, as a draft spells it |
| `AppRunProcess.dir?`, `port_env?` | Its folder, and the variable its port is set in, as a draft carries them |
| `AppRunProcess.pinned_port?` | The pin, kept through a save that replaces the run |
| `AppRun.setup_commands?` | The Manifest's `setup.requires` as `{name, command}` each, the repository's included |
| `AppRun.in_kit` | Whether Kit's copy declares the run, so a save may replace it |

A `save_app_run` whose `name`, made letters, digits and `_`, is a run Kit's copy declares replaces that run rather than being refused.
The run, each server only it lists, and their ports are taken out of Kit's copy and written again from the save, and a server kept by name keeps its pin.
`setup` on that save is the checkout's whole setup: Kit's `setup.requires` becomes the names the repository does not already require, and a Kit setup Command the save leaves out is removed.
A Command the repository declares is left as the repository's where the save sends the same line, and refused by name where it sends another.
The old run's processes and setup are stopped before it starts again from what was written.
A run only the repository's own `armada.yml` declares has `in_kit` false, and a save under its name stays refused with `fleet.app_run_draft_refused`.

**`SaveAppRun.replaces` names the run an edit replaces by its key**, `AppRun.name`, so Edit can rename a run or replace one whose key is not its label made safe.
With it, that run is the one taken out and written again under the key `name` makes, and no run is matched by `name`.
A key that is no run is `fleet.not_an_app_run`; one only the repository's own `armada.yml` declares, or a new key another run already has, is `fleet.app_run_draft_refused`, and nothing is written.
The setup stopped before the start is the old key's.

**The run's field for how it opens is `opens_in`, not `kind`.** `Event` is tagged by `kind`, and `app_run.changed` carries the run flat beside it, as every event carries its row.
`opens_in` is `browser` where a server of the run listens on a leased port and offers a link, and `app` otherwise.

**The checkout is a tagged `AppRunCheckout`**: `main`, or `job` with `job_id`, `branch?` and `bay`.
An event names only a path, and Fleet finds the Job from its record of leases.
A path that is neither a repository's main nor a Job's worktree gets no event: a worktree somebody cut by hand is `start_server`'s, not a run's.
A Job's checkout is read from that record and never from a path in the request.

**`${port}` is the one spelling of the leased port a person sees.** The Manifest's own is `${port.NAME}`, and Fleet converts at the wire in both directions: a draft's `${port.web}` is sent as `${port}`, and a saved command holding `${port}` becomes a port named for its process.
A command without it listens on none.
`port_env` is the one piece of a draft that a person does not edit and the save must not lose, so it is handed back as it was read.

**A link is one process expecting another on a port it does not get**, `{from, to, port, found_in, brief}`, on `AppRun.links` and `AppRunDraft.links` and absent when there is none.
A frontend whose dev proxy names `127.0.0.1:8731` while the backend is leased another number is the case.
`found_in` is a file path relative to the repository's root or `log`, and there is at most one link for a pair.
`brief` is the text for Bridge to open Dispatch with, naming `ARMADA_PORT_<NAME>` for the port `to` listens on; Fleet starts no Job.
Fleet reads the folder of each process (`vite.config.*`, `.env*`, `package.json`, `next.config.*`, `webpack.config.*`, `setupProxy.js`) for a `localhost`, `127.0.0.1` or `0.0.0.0` port, and matches it to a port another process is known on: pinned, a literal in its command or config, or where a Job's dev server in that folder came up.
A running process that prints a refused connection to a port a sibling is known on, and is not on now, adds a `log` link and `app_run.changed` says so.
**Links are read from the files on a request and never on an event**, and an event carries what that read found, kept per checkout, so a bay that fixed its copy is not told the main checkout's.
A link is run memory's `port_link` kind, written to the place the repository's run memory is kept (`this_mac`, `synced` or `cloud`) and never to `armada.yml`, so a `repository` place holds them in Fleet's own store only.
An ended link is a `port_link` fact written again as not succeeded and newer, never a deleted row, because a place that keeps a file merges the old fact back over a deleted one.
It ends when the main checkout's file no longer holds it, and when the process it points at is unpinned.

**A pin is `fixed` on the port a process listens on**, a new key on a `ports:` entry from 1024 to 65535, written into Kit's copy and carrying the `env` and `container` the repository gave it, since Kit replaces a port by name.
`AppRunProcess.pinned_port` says it.
A pinned process takes exactly that number on every checkout, and a start where something else holds it is refused as `fleet.pinned_port_taken`, naming the number and the server and checkout that hold it, or none where it is a program Armada did not start.
Fleet's own servers are asked before the probe, because one that has started has not always bound yet.
A pin or unpin that cannot be made is `fleet.app_run_pin_refused`.

**`last_used` is what this Fleet has seen since it started.** It is the newest start of any process of the run on that checkout, and absent is not "never".

**The event stream.** Better or worse for the bounded-stream risk: slightly worse, and not in a way that grows with agents.
Each press adds one `app_run.changed` per run per server fact, which is two or three per process, and a person presses rarely.
It adds no per-client queue and no log line to `/events`.
Every event is whole, so one missed is replaced by the next, and a Bridge that fell behind reads `list_app_runs` on the resync that follows a `Missed`.
Inside Fleet it adds one unbounded in-process queue of server facts, drained by one task, in the shape the rail's and the timeline's taps already have.
The task reads after the server's own event is sent and never under the row's lock, so the run it publishes is at least as new as the fact that woke it.
Finding a link from output adds one more unbounded queue beside it, fed by a tap on the pass that already reads each server's log for `observe_server`.
The tap keeps a line only if it names a refused local port, and tells it once for each port of each instance, so a program that refuses every second sends one.
