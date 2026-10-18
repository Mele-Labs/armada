# Log Envelope

**What it is:** The field contract every log line carries across Fleet and Bridge, so that one Job's activity can be assembled from more than one emitter. Fleet is the sole ID authority.

---

**Kind:** Policy.

Armada emits log lines from independent emitters into sinks that already exist: machine-level `audit.jsonl` and the WebSocket event stream, beside Fleet's own log. Nothing else guarantees a line from Bridge can be joined to a line from Fleet. This document is that guarantee.

**This document owns field names, their types, and which emitter is allowed to mint them.** It does not own sink paths, retention or redaction. Those live in `../contracts/system-architecture.md` sections 7 and 8.

**It is a Policy rather than an Entity: it has no ID, no state and no lifecycle.** It is a rule applied at every emit site, and code branches on it only at the sink.

## The envelope

Identical key names across all emitters. A key is either present with a value or absent. Never present and null.

| Field | Type | Present | Notes |
| --- | --- | --- | --- |
| `ts` | RFC3339 UTC, ms precision | Always |  |
| `level` | `trace` `debug` `info` `warn` `error` | Always | Lowercase |
| `component` | `fleet` `bridge-main` `bridge-ui` | Always | Emitter identity. Closed set |
| `run_id` | ULID | Always | The **emitting process's** instance, minted by that process at start |
| `msg` | String | Always | Never carries an interpolated ID |
| `session_id` | ULID | Once a Job exists | The correlation spine |
| `workspace` | String | When the line concerns exactly one workspace | Single-valued. See the rule below |
| `target`, `span` | String | `fleet` only | Supplied by `tracing` |
| `fields` | Object | Optional | All structured data. Nothing structured belongs at the top level |

## Rules

**Fleet is the sole authority for `session_id`.** Bridge never mints one; it echoes what it was handed, which makes the join reliable rather than best-effort. Why: an id invented by something that does not own the record joins to nothing.

### `run_id` names the emitter

**`run_id` is the exception, because it names the emitter rather than a record.** Each process mints its own at start: Fleet's changes on every restart, which is what makes a restart visible in the log, and Bridge's identifies its own.

Bridge runs before it has reached Fleet at all, so a single Fleet-owned `run_id` could not describe it. Reading it is per emitter: a Fleet restart is Fleet's `run_id` changing, not any `run_id` changing.

### Each emitter is handed the spine

**Bridge attaches the spine at the protocol layer, not at the call site.** Every Fleet-to-Bridge message envelope carries the IDs; a Bridge log line inherits them from the message being handled. Why: threading them by hand through TypeScript call sites is how they go missing.

**Fleet uses spans, not arguments.** A `session` span means `tracing` propagates the field automatically, so no function signature grows an `id` parameter for logging's sake.

### Lexicographic sort is chronological

**ULIDs, not UUIDv4.** Why: lexicographic sort is chronological, so merging two emitters' lines into one ordered view costs a string sort.

### Nothing greps `msg`

**IDs are fields only.** Any query or `jq` filter targets a field; nothing greps `msg` text. This is the same discipline as `store` being the only crate that deserializes: the structured path is the only path.

## A line spanning workspaces omits `workspace`

**`workspace` is single-valued and omitted when the line is not scoped to one workspace.** A line from a Job writing in several of them carries `session_id` and no `workspace`. Why: an array would tax every query on a hot-path field forever, and most lines genuinely concern one workspace or none.

The absence of `workspace` on a line is itself the signal that the line is Session-scoped rather than workspace-scoped.

## Relationship to the existing sinks

The envelope is the line shape. The sinks and their retention are already decided elsewhere and are not restated here.

| Sink | Carries the envelope | Owned by |
| --- | --- | --- |
| `~/Library/Application Support/Armada/audit.jsonl` | Yes, plus `actor` | `../contracts/system-architecture.md` section 7 |
| WebSocket event stream | The ID fields only. Events are typed, not log lines | `../contracts/system-architecture.md` section 6 |

**`Redactor` runs after the envelope is assembled, not before.** It already scrubs every sink. Nothing in the envelope is exempt, and `fields` is where a leaked credential would most plausibly land.

## Open questions

- **[log-envelope-name]** Is "Log Envelope" admitted into the sanctioned lexicon as a proper noun, or is this document renamed to something the lexicon already allows? "Log Envelope" is descriptive and currently outside it.
- **[log-envelope-bridge-ui-emitter]** Does `bridge-ui` emit log lines at all, or does it forward to `bridge-main`? A renderer writing to disk directly would be a second writer for one process class.
