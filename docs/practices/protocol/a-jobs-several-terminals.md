# A Job's several terminals

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 2 Nov 2026. `docs/concepts/session/shared-terminal.md`; Protocol 23.93 was a Job's one shell.
A Job holds several terminals, each named `t1`, `t2` and so on, and the person can hand the agent an ID.
**Additive except three required fields**: two operations, one route each, and optional fields on the rest; `TerminalTyped`, `TerminalScreen` and `JobTerminal` gain a required `terminal`.
There is no number to bump: the protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`.
Bridge's half is `packages/protocol/src/sessions.ts`, `events.ts`, `watches.ts` and `hosted-sessions.ts`, written by hand, field for field.

IDs are a plain `String` on the wire, as `run_id` is.

| Where | Side | Carries |
| --- | --- | --- |
| `OpenTerminal` (`open_terminal`) | Bridge sends | Gains `terminal?` and `new?` (absent is `false`). `new` mints the next ID and starts a shell. `terminal` names one: its running shell, or a new shell under the same ID where it has ended. Neither is the lowest-numbered open terminal, else the lowest-numbered the Job has had reopened, else `t1`. 422 for a `terminal` the Job never had, and for `new` with `terminal` |
| `SessionRunUnderway` (open's answer) | Fleet answers | Gains `terminal?`: set for a Job's shell, absent for any other run |
| `TypeInTerminal` (`type_in_terminal`) | agent sends | Gains `terminal?` and `new?`. Named: that terminal, 422 where it is not open. `new`: opens a new terminal, then types. Neither: the agent's last used where it is open; where it has ended, opens a new one and refuses, naming both; where it never used one, opens a new one and types. **Never the person's terminal by default.** `new` with `terminal` is 422 |
| `TerminalTyped`, `TerminalScreen` | Fleet answers | Gain `terminal`, **required**: which one answered |
| `ReadTerminal` (`read_terminal`) | agent sends | Gains `terminal?`. Named: that terminal. Absent: the agent's last used where it is open, else the lowest-numbered open, else 422 |
| `list_terminals` (`GET /sessions/terminal/list`, `agent_access = "Yes"`) | agent reads | `ListTerminals` `{ session_id? }`; answers `JobTerminals` `{ terminals: [{ terminal, name, run_id, running }] }`. Open terminals only, lowest number first. `running` means a program other than the shell has the foreground |
| `rename_terminal` (`POST /sessions/terminal/rename`, `agent_access = "No"`) | Bridge sends, Fleet answers 200 | `RenameTerminal` `{ session_id, terminal, name }`; answers `ListedTerminal`, the terminal as it now stands. A blank `name` restores the default, `Terminal N`. 422 for a terminal the Job has not open |
| `JobTerminal` (`terminal.opened`, `terminal.exited`) | Fleet publishes | Gains `terminal`, **required** |
| `start_watch` with `in_terminal` | agent sends | Gains `terminal?`; unnamed is the agent's last used where it is open, else the lowest open, fixed when the watch starts. `terminal` without `in_terminal` is refused as the body is read, 400. `WatchSummary` gains `terminal?` |
| `TagKind` | Bridge sends | Gains `Terminal`: the tag's `id` is `t2` and its `title` the terminal's name. Fleet's line to the agent tells it to pass the ID as `terminal`. The TypeScript union already held `"terminal"` |

**The three required fields** are the only ones an old peer cannot ignore: a Bridge that reads a `JobTerminal` or an answer without `terminal` has nothing to key a pane on, and a Fleet that builds one without it does not decode.
They are required because every answer and event of a terminal names which one, and an absent one would read as `t1` by habit.
The pair is refused whenever the IDs differ, so neither is built against the other.

**The agent door.** `list_terminals` is a door tool and is served with no Manifest, as `type_in_terminal` and `read_terminal` are.
Their descriptions say a Job has several terminals, that the person can hand over an ID, that `list_terminals` names them, that an unnamed call means the agent's last used, and that `new` opens its own.
`rename_terminal` is the person's act and stays off the door.

**What Fleet does with them.** A Job's shells are in memory under their number; the store's `job_terminals` keeps a row a terminal for as long as the Job lives, with its name and the shell its agent last saw, so a number is never given twice and a name outlives the shell.
`running` reads the terminal's foreground process group against the shell's own.
A terminal watch fixes its terminal when it starts and keeps it.
The agent's unnamed typing takes its last used terminal while that is open, and otherwise opens one of its own.
Where that last used has ended, it opens a new one and refuses, naming both.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the two routes answers 404.

**The event stream.** The same two events, `terminal.opened` and `terminal.exited`, now one pair for each terminal, so a Job with three shells publishes up to three of each.
A rename publishes nothing: Bridge reads the name from the answer it is given and the agent reads it from `list_terminals`.
The open risk below is slightly worse and not measured: a Job can now run several shells that print at once, though their output goes by `observe_session_run` sockets of their own and not by `/events`.
