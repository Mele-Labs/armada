# A Job shows a live graph: `show_graph`, `graph_points`, `withdraw_graph`, `list_graphs` and `graphs.changed`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**Additive only**: four operations, one event kind, one thread item and the types they carry, in `crates/ipc/src/graphs.rs`.
The protocol ID moves with `crates/ipc/src`, the operation files and the route table, and there is no number to take.
Fleet's half is `crates/fleet/src/graphs.rs`, the store's `crates/store/src/job_graphs.rs`, and the recipe runs through `adapters::run_graph_recipe`.
Bridge links `ipc`, so it reads the same types; the native card is `crates/bridge-native/src/job_view/graph_card.rs`.

| Where | Carries | Who |
| --- | --- | --- |
| `show_graph` (`POST /sessions/graph`) | `ShowGraph`; answers `GraphShown` | The agent |
| `graph_points` (`POST /sessions/graph/points`) | `GraphPoints`; answers `GraphPointsTaken` | The agent |
| `withdraw_graph` (`POST /sessions/graph/withdraw`) | `WithdrawGraph`; answers `GraphShown` | The agent |
| `list_graphs` (`GET /sessions/graphs?session_id=`) | `GraphList` | Bridge |
| `graphs.changed` (event) | `GraphsChanged` | Bridge |
| The `graph` thread item | `ThreadGraph`: `id`, `at`, `graph`, `again` | Bridge |

## The act

`ShowGraph` is `id`, `title`, one of `spec` and `strip`, an optional `headline`, `source` and `session_id`.

- **`spec`** is Vega-Lite, read by `ipc::graph::read_spec`.
  A key Bridge does not draw is refused by name with the words that place takes, so the agent can write it again.
  Its `data.name` names a feed of the Job; `data.values` draws rows given once.
- **`headline`** is `{field, unit?, since?: {label, at}, better?: up|down}`, the latest value of the spec's feed over the chart.
- **`strip`** is up to 8 `{label, feed, field, unit?, shown?, since?, better?}`, in place of a spec.
- **`source`** is `{recipe: {call, map, every_secs, credential?}}`.
  `call` is a project metric's `MetricCall` (`crates/ipc/src/metrics.rs`), held to `adapters::problem_with`'s rules.
  `every_secs` is 10 to 86400.
  `credential` is a metric recipe's `MetricCredential`, `{name, how, url?}`: its token is read from this Mac's keychain for each run and fills `{credential}`, and every why is scrubbed of it.

**The map prints `{"<feed>": [rows]}`**, or a list of rows where the graph reads one feed.
One call can feed several feeds that way, which a dashboard query answering several series needs.
A row equal to one the feed holds is not added again, so a map may print the whole window a query answers each tick.

**The same `id` again replaces it in place**: its place among the Job's graphs, its feeds' rows and its last read stay, and its recipe is the new call's.
A Job holds at most 24 graphs.
Every refusal is a 422: `fleet.graph_refused` (each problem said at once), `fleet.graph_too_many`, `fleet.graph_not_shown`, `fleet.graph_unplaced` and `fleet.no_such_session`.

## What Bridge is sent

`GraphShown` is the card: `id`, `title`, `form` (`chart` with `spec` and `headline?`, or `strip` with `stats`), `fed_by` (`recipe` with `every_secs`, or `points`), `live`, `shown_at`, `read_at?`, `broken?` and `no_token?`.
Bridge words `fed_by` under the title.
**`live` is false once the graph is withdrawn or its Job has ended**, and the card stays.
`broken` is why the recipe's last read gave nothing, and the rows it held are kept.
**`no_token` is the credential the keychain does not hold**, and the recipe is not run until it does; `broken` says which token is missing too, so a card read after a restart says it before the next pass.
Bridge draws it as a project metric's block draws its missing token: what to make, and where.

**The change on a headline or a stat is Fleet's**: `delta` is `{said, since?, tone}`, as `+41%`, `since deploy`, `worse`.
It runs from the value at `since.at`, the latest row at or before it, to the latest row; with no `since`, from the feed's first row.
A chart reads a row's time from its first temporal `x` field, a strip from `at`.
A unit of `%` is said in points, anything else as a percentage, and a start of 0 gives no `delta`.
`better` sets the tone; with none it is `steady`.

`GraphFeed` is `name`, `rows` and `added`, every row the feed was ever given.
In `list_graphs`, `rows` is every row the feed keeps, at most `ipc::GRAPH_FEED_KEPT` (2000), oldest first.

## `graphs.changed`: rows appended, not whole

**The event carries the Job's cards whole and only the rows added since the last event.**
Whole feeds would put up to 2000 rows a feed on the socket every tick of every recipe; appended, a 15 second recipe sends the rows it read.
The cards are few and small, and a headline's change moves with every row, so they go whole.

**A client knows when it missed rows.** It holds each feed's `added`; an event whose `added - rows.len()` is not that count means an event went by unseen, and the client reads `list_graphs` again rather than drawing a feed with a hole in it.
That is the "you missed N events, resync" [the unmeasured risk](the-unmeasured-risk-the-websocket-sink-has-no-back-pressure.md) asks for, for this event only.
It does not bound the sink: it adds a few small events a Job each tick to the stream, and the sink is still unbounded.

**One event a Job a tick.** The recipes due in a pass of the watch loop (`fleet::watches::EVERY`, 5 seconds) run together, off the async thread, and each Job is sent one event once they have all finished.
A `graph_points` call that added rows is one event; a show, a replace and a withdraw are one each, with no rows.

## Kept

`job_graphs` holds a row a (Job, id): the checked body as JSON, when it was shown, its recipe's interval, `live`, and its recipe's last run, last read and why it broke.
`job_graph_rows` holds each feed's newest 2000 rows, numbered by `added`.
A restart keeps both, and each recipe runs on from when it last ran.
**A recipe stops when its Job ends**: the loop finds the Job ended, marks its graphs not live and sends one event.

## In the thread

The agent's `show_graph` call is the thread's `graph` item, placed where the call is.
The row's text is `show_graph <id>`, the transcript reading the id off the call as it reads `show_file`'s path.
**Bridge draws the card of the graph `graph` names**, from what `list_graphs` and `graphs.changed` carry, and the call's line where it holds no such graph, as for a call Fleet refused.
A later call with the same id is `again`: the card stands at the first.
A call made as `armada job show_graph` from Bash is a Bash row and places no card, as for `show_sketch`.
