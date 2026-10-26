# A repository's app, run from Bridge: `list_app_runs`, `start_app_run`, `app_run.changed` and four more

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**Additive only**: six operations, one event kind, and the types they carry.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/app-runs.ts` and `events.ts`, written by hand.
Fleet's half is `crates/fleet/src/app_runs/serving.rs`, which converts what `crates/fleet/src/app_runs.rs` reads into `ipc`'s own types.
All seven are `Bridge only`: ▶ is a person's press, and an agent that wants a server already has `start_server`.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_app_runs` (`GET /app_runs?manifest_id=` or `?job_id=`) | `AppRunList`: one `AppRun` per run the file declares | Main checkout, or the worktree a Job holds. Runs that are not up are in it |
| `get_app_run_draft` (`GET /app_runs/draft`) | `AppRunDraft`: `root`, `place`, `processes` | Saves nothing. No `processes` is a repository nothing was learnt of |
| `start_app_run` (`POST /app_runs/start`) | `NamedAppRun`; answers `AppRun`, 202 | The run already up is answered as it stands |
| `stop_app_run` (`POST /app_runs/stop`) | `NamedAppRun`; answers `AppRun` | Each process waited for |
| `restart_app_run` (`POST /app_runs/restart`) | `NamedAppRun`; answers `AppRun`, 202 | Every process is a new instance |
| `save_app_run` (`POST /app_runs/save`) | `SaveAppRun`; answers `SavedAppRun`, 202 | Written into Kit's copy, then started |
| `app_run.changed` (event) | `AppRun`, whole | After each `server.*` fact of a server a run lists |

**A run is its servers and nothing more.** A process is a `ServerState` read through, so its output is `observe_server`'s and is addressed by the process's `server_id`.
No second socket carries a run's log, and `app_run.changed` carries no line of one.
A restart is a new instance, so a viewer reads `server_id` again from the next event.

**The run's field for how it opens is `opens_in`, not `kind`.** `Event` is tagged by `kind`, and `app_run.changed` carries the run flat beside it, as every event carries its row.
`opens_in` is `browser` where a server of the run listens on a leased port and offers a link, and `app` otherwise.

**The checkout is a tagged `AppRunCheckout`**: `main`, or `job` with `job_id`, `branch?` and `bay`.
An event names only a path, and Fleet finds the Job from its record of leases.
A path that is neither a repository's main nor a Job's worktree gets no event: a worktree somebody cut by hand is `start_server`'s, not a run's.
A Job's checkout is read from that record and never from a path in the request.

**`${port}` is the one spelling of the leased port a person sees.** The Manifest's own is `${port.NAME}`, and Fleet converts at the wire in both directions: a draft's `${port.web}` is sent as `${port}`, and a saved command holding `${port}` becomes a port named for its process.
A command without it listens on none.
`port_env` is the one piece of a draft that a person does not edit and the save must not lose, so it is handed back as it was read.

**`last_used` is what this Fleet has seen since it started.** It is the newest start of any process of the run on that checkout, and absent is not "never".

**The event stream.** Better or worse for the bounded-stream risk: slightly worse, and not in a way that grows with agents.
Each press adds one `app_run.changed` per run per server fact, which is two or three per process, and a person presses rarely.
It adds no per-client queue and no log line to `/events`.
Every event is whole, so one missed is replaced by the next, and a Bridge that fell behind reads `list_app_runs` on the resync that follows a `Missed`.
Inside Fleet it adds one unbounded in-process queue of server facts, drained by one task, in the shape the rail's and the timeline's taps already have.
The task reads after the server's own event is sent and never under the row's lock, so the run it publishes is at least as new as the fact that woke it.
