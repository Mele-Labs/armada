# A Job's Loadout

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 17 Oct 2026. What a Job's agent knows, was told, and used each turn, each item with where it came from.
**Additive only**: two facts, one optional field on `SessionRecord`, two operations and the types they carry.
No event kind or error is new.
Bridge's half is `packages/protocol/src/loadout.ts` and `sessions.ts`, written by hand.
The migration `20261017T1700Z-session_loadout.kept` is additive.
**Names, sources and counts only** on the facts, the record and `get_session_loadout`: never a skill's body, a prompt's text, a tool's input or a file's contents.
What one row says is `read_loadout_item`, read off disk where Fleet runs when a person opens that row.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionFact` `equipped` | The mod writes | `loadout`: `LoadoutItems` `{ items }`, the whole list, replacing the last. `LoadoutItem`: `kind`, `name`, `source`, `from?`, `state?`, `tokens?` |
| `SessionFact` `used` | The mod writes | `use`: `LoadoutUse`: `turn_started_at`, `kind` (`skill`, `tool` or `subagent`), `name`, `source`, `from?`, `model?`, `by?` |
| `SessionRecord.loadout_summary?` (`session.changed`, `get_session`, `list_sessions`) | Fleet writes, Bridge draws | `LoadoutSummary`: `knows`, `told`, `latest_turn?` (`started_at`, `used[]`). Absent where no `equipped` was ever reported |
| `get_session_loadout` (`GET /sessions/loadout?session_id=`) | Bridge reads | `SessionLoadout`: `items[]`, empty where none was reported, and `turns[]`, oldest first, at most fifty. 422 `fleet.session_unnamed` for a job Fleet does not know |
| `read_loadout_item` (`GET /sessions/loadout/item?session_id=&kind=&name=&source=&from=`) | Bridge reads | `LoadoutItemBody`, tagged on `what`: `read` (`path?`, `format` `markdown` or `text`, `body`, `truncated?`) or `not_read` (`why`). The query is `LoadoutItemAsked`; a use's kind is asked as the kind of that name. 422 `fleet.session_unnamed` for a job Fleet does not know |

**Every closed set is a plain enum Bridge matches on**: `LoadoutSource` (`kit`, `armada`, `repository`, `plugin`, `mcp`, `built_in`, `managed`), `LoadoutKind` (`skill`, `subagent`, `mcp_server`, `tool`, `memory_file` are known; `output_style`, `rules`, `injection` are told), `LoadoutState` and `LoadoutUseKind`.
A value outside them refuses the fact.

**A turn is the instant the mod says it began**, and turns are ordered by when Fleet first heard of each.
**A use repeated in its turn is kept once and counted, and publishes nothing by itself**, and an `equipped` equal to the one held publishes nothing, so a turn of a hundred MCP calls to one tool is one row and one `session.changed`.
`LoadoutUse.count` is **Fleet's field**: the times it heard that use in its turn, left out when 1, and a `count` a mod sends is ignored.
The store keeps it in `session_loadout_uses.count` (`20261018T1000Z-session_loadout.use_count`, additive).
A count that rose rides the next record Fleet publishes anyway: a measurement that moved a figure, or the turn's end, which always publishes.
There is no timer, so a Bridge shows the count as of the last of those and the final number at the turn's end.

**The event stream.** Each new list and each new distinct use publishes one `session.changed`, through the shared drop-oldest backlog.
It adds no event kind and no queue, and the dedupe keeps it to what changed, and a repeat's count adds no event, so it leaves the unmeasured sink risk where it was.
A Bridge that missed one reads the record again on the resync that follows a `Missed`.

**What a row reads as.** Only a row the job's Loadout lists, as an item or a use; any other reads `not_read`. Which file is `HarnessSetup::read_item`, so the harness's paths stay in `crates/adapters`:

| Kind | Read |
| --- | --- |
| `skill` | `SKILL.md` of `.claude/skills/<name>/` in the job's directory, `~/.claude/skills/<name>/` (or a folder up to three down whose name or front matter says it, where a synced set lands), or the plugin's `skills/<name>/`. A plugin's `<plugin>:<name>` is split on the colon, and `armada` is the mod's own plugin |
| `subagent` | `<name>.md` (or the file whose front matter names it) in `.claude/agents/`, `~/.claude/agents/` or the plugin's `agents/` |
| `output_style` | The same, in `output-styles/`. A built-in style is `not_read` |
| `memory_file` | The file at the path the mod reported: `~/.claude/…`, relative to the job's directory, or absolute under one of those two |
| `rules` | Armada's compiled job rules, `text`, no `path` |
| `injection` | The `armadaSession` note the mod adds, with the id as `<job id>`, `text`, no `path`. Kept by hand beside `idNote` in `plugins/armada/hooks/register.ts`, and a test fails when they differ |
| `mcp_server`, `tool`, and a `built_in`, `managed` or `mcp` source | `not_read` with why |

A plugin's folder is its `installPath` in the harness's record of installed plugins.
**A name that could climb is refused before anything opens**: `..`, `.`, a separator or a NUL in a skill's, an agent's, a style's or a plugin's name, `@` in a plugin's, and a memory path with a `..` part or absolute outside the job's directory and `~/.claude`.
**The file is resolved through every link and must still be under its folder or the person's home**, as Kit follows a dotfiles link, so a link to anywhere else reads nothing.
The body stops at `LOADOUT_ITEM_CAP`, 256 KiB, on a whole character, with `truncated`.
`path` is from the job's directory, `~/…` under the home, or from the plugin's folder.

**Skew.** A Bridge that predates this ignores a `loadout_summary` it does not read.
A Fleet that predates it refuses both facts, and the mod goes on.
`read_loadout_item` is a new route, so the protocol ID moves with it and a Bridge and a Fleet that differ on it never meet.
