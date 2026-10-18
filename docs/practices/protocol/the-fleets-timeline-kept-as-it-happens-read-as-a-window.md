# The Fleet's timeline: kept as it happens, read as a window

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`get_fleet_timeline` (`GET /timeline?since=&until=&limit=`) and `timeline.appended` serve the Cockpit's
timeline: what happened to Jobs, Sessions, pull requests, slots and main, each with its instant. Before
them nothing answered it. `get_events_since` counts kinds, `get_job_events` is one Job's status moves,
and a Session or a merge line moves as a row replaced whole, with no instant to cut a window by.
Since 23.82.

| Source | How it is entered | Why not an event of its own |
|---|---|---|
| A Job's moves, a Session row | A tap on the stream (`Broadcaster::tap`) turns each published event into an entry | They are events already; they only lacked a record |
| Pull requests, main's colour | `merge_lines::keep_reading` compares each read of the hub with the one before | `merge_lines.changed` carries the list whole, so what happened is the difference |
| A person's answer | The three Job answers and the Session ones enter it after they succeed | The answer is a command, and the row it clears is replaced |
| A slot leased or given back | The ledger calls that take and give back a Job's or Session's slot, when the row changed | The pool is read whole and never published |

**A tap and not a subscription.** A subscription drops its oldest when it falls behind, and a timeline
that silently lacks a Job's failure is worse than none. The tap runs inside `publish`, so it only
queues; one writer takes the store lock and numbers entries in the order queued. `timeline.appended` is
published after the write, so its `id` is the number the store gave it.

**Entries are dated by whoever saw the thing**: a Job's event carries its instant, a Session's row states
its start and each item's `since`, the forge dates a merge, and the rest are stamped by the read that saw
them. The window is cut by `at` and never by `id`, which only says what a client holds already.

**Kept thirty days**, pruned on the next write, as `check_runs` is. The row is the fact as it read then,
so a forgotten Job keeps its history.

**A Session's turn is entered once, where it ends**, and the fact is its length (`session.turn`, "Turn took
2m 14s"), from the start Fleet heard to the end it heard. Never on the rows between, and not on the end of a
turn held for a slot, which resumes in it as one turn. A turn reattached after a restart has no start heard,
so it enters nothing.

**Reads, edits and commands are entered one to a burst**, because an agent makes hundreds of tool calls an hour and a row for each would bury the turns.
`session.read`, `session.edited`, `session.ran`, `job.read` and `job.ran` count what one Session or Job did of one act: `Read 14 files`, `Edited 3 files`, `Ran 5 commands`, with the latest path or command as `detail`.
A burst ends when its subject has done none of it for fifteen seconds, when the Session's turn or life ends, or two minutes after it began, so a long run still shows as it goes (`QUIET` and `MOST` in `timeline/tools.rs`).
**The entry is written when the burst closes and dated by its first call**, so a client orders a window by `at` and not by `id`.
Only a path is kept of a file, and a command is cut to the line and has the value of a `NAME=`, a `--flag` or a query pair whose name says secret, what follows `Authorization:` or `Bearer`, a password in an address and any long token-shaped word hidden: it errs toward hiding.

**`items` is what the burst named**, so a row can open to its files or commands: newest first, a path or command appearing once however often it was named, **at most 25** (`MOST_TIMELINE_ITEMS`), each cut and hidden exactly as `detail` is, and the first is `detail`. 25 bounds an entry at about 3 KB and a list at about a screen; the count in `says` is still the whole burst's.
The store keeps them in `timeline_events.items`, one to a line (a line is all an item can be), and an entry written before the column has none: Bridge then has nothing to open.
Absent from every other kind.
Additive.

| Source | Where it is heard |
|---|---|
| A terminal Session | The mod's `touched`, `read` and `ran` facts (`sessioning.rs`), counted when the Session's origin is a terminal |
| A Session Bridge hosts | Its own process's calls (`session_host::hearing`), through the harness's `edited` and `used`, so the mod's facts for it are not counted twice |
| A Job's Drone | A tap on its stream (`JobToolsSeen`). A Job's edits stay `job.files` |

**A call outside the repository is `session.called`, one entry for each call** (17 Oct 2026).
A Session's `gh` command, a page it fetches, a web search, and a tool of any MCP server that leaves the machine.
The entry's `says` is `Called {name}` (`Called GitHub`, `Called the issue tracker`, `Called the web`, else the server's name), `detail` is the one line that says what was asked, and `call` is `{ service, name, line }`: the service, what it is called (the `Reach.name` the adapter already has, the same words `says` uses) and the same line.
Bridge labels a service's outpost with the newest call's `name`, the leading `the` dropped and the first letter capitalised, so it holds no service's name of its own.
**The line names the act and never what it was given**: a `gh` command is `gh` and its subcommand words up to two (`gh pr view`, never a number, flag or body); a fetch is the page's host and no scheme, user, port, path or query; a search is `search`, never the query; an MCP call is the tool's name.
`gh issue …` is the `tracker` service, every other `gh` command and the forge's own server the `forge` service, and so is a Linear, Jira, Atlassian, Asana, ClickUp or Monday server; any other server is its own lowercase name with a connector or plugin prefix dropped (`claude_ai_Figma` is `figma`).
Armada's own servers and the code-graph helper never leave the machine and enter nothing.
A `gh` command is this and not a `session.ran`, so it is in no burst.
**`service` is an open string**: Bridge draws a plug for one it has no word for, so a new server moves nothing in Bridge.
The set is read in `crates/adapters/src/outside.rs`, because the tool names are the harness's words and the commands the forge's; Fleet only carries the `Reach` it is handed (`ToolUse::Reached`).
The store keeps the service in `timeline_events.service`, its name in `service_name` and the line in `detail`; a row written before the column has none.
A terminal Session's `gh` command is told by the mod's `ran` fact like any command.
**Its MCP and web calls are not entered yet**: the mod does not tell them, and the `used` fact it does tell also fires for a tool merely loaded, so it cannot stand in for a call.
Additive: a new kind, one optional field left out when absent, two nullable columns.
**It adds entries to the event stream**, one per call rather than one per burst, so a busy Session writes more than it did; that makes the unbounded sink's risk a little worse and not better, and the drop-oldest bound with its resync message stays owed.

**The agent's own list is `session.waiting` and `session.resolved`**: an item put on the `waiting_for` list is a wait, dated by its `since`, and one gone from it while the Session lives is resolved, dated by the row that dropped it.
What Fleet raised (a question or permission card, a question in a walk window, a reply to read) stays `session.asked`, and a person's answer stays `session.answered`.
A Session ending clears its list, which resolves nothing.
`session.subagent` is a subagent attached to the row (`Subagent started`), standing again after it settled (`Subagent resumed`, dated by its new `since`) or spent (`Subagent finished`), with `Type: description` as the detail; a Session already holding one when Fleet first saw it enters nothing for it.
`session.spawned` is entered under the Session whose agent started another, or forked itself (`start_another_session`, `fork_this_session`), with the new Session's title as the detail.

**A Session's walk, commit and push are entered as `session.walked`, `session.committed` and `session.pushed`.** The walk is written when the page is shown (`show_window`), with its title as the detail, and apart from the `walk` item on the Session's list, which is not entered.
A commit and a push come from the mod's `committed` and `pushed` facts, which it tells only where git printed a commit made or a ref updated; the detail is the branch.
They are the road a Session's `build` reads its stops from (`docs/concepts/session.md`, *The build record*), so the Session's row is published again once one is written.
The store holds an index on `timeline_events (subject_kind, subject_id, kind)` for that read.

**A pull request is entered whether or not anyone is shown it**, but Bridge draws it only on the track of the Job or Session that owns it, and leaves out one it cannot attribute (`fromWire` in `cockpit/fleet-events.tsx`).

**Not entered, deliberately.** A Check's start (its result is entered), a slot an agent leases with `armada worktree lease` (Fleet never hears of it; *A bay's history* below reads it from the pool), and
anything before this shipped. A Session already live when Fleet starts is entered as started the first time
its row changes.

**The event wraps the entry under `event`.** The stream tags its events with `kind`, and an entry has a
`kind` of its own; spread into one message they produced a duplicate key, which a test caught.
