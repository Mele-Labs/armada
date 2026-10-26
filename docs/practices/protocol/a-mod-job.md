# A Mod Job: `start_mod_session`, `mod` and `made_by`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 23 Oct 2026. `docs/concepts/mods.md`, *A Mod Job*.
**Additive**: one operation, one route, the two types they carry, and four optional fields on existing types; no event kind.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/mods.ts` and `sessions.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `start_mod_session` (`POST /mods/session`, `agent_access = "No"`) | Bridge sends, Fleet answers 200 | `StartModSession` `{ title?, model?, effort?, note?, said? }`, `note`, a `SessionNote` as `send_session_note` takes it, or `said`, his words where Mod was pressed on nothing, **not both**, or neither for an empty Mod Job; answered with `ModSessionStarted` `{ session_id, title, send_failed? }` |
| `SessionRecord.mod` | Fleet answers | `true` on a Mod Job, left out on every other |
| `ScaffoldMod.session_id` | an agent sends | The Job calling. Left out records nothing |
| `ModSummary.made_by` | Fleet answers | The Job whose `scaffold_mod` made the mod, from its `mod.toml`. Left out where none was recorded |

**Where this Fleet does not serve the Manifest named `armada`** the Job starts in the mods folder, with no Manifest, slot or branch, and `SessionRecord.manifest_id` is left out.
The agent door a Job with no Manifest stands at offers `list_mods`, `scaffold_mod`, `validate_mod` and `try_tracker_mod` beside the Job's own acts, so that Job can make one.

**Create mod** (24 Oct 2026) makes `note` optional and adds `said`, both on `StartModSession` and both Bridge-sent.
With `said` the first message is the Mod brief and then his words, by the road a message takes, and the title defaults to the first line of the words.
With neither (25 Oct 2026) the Job opens empty, titled `Mod`, and sends nothing; no field is added.
The first message the person then sends reaches the agent with the brief ahead and retitles the Job `Mod <first line of his words>`, cut at a word within 40 characters, and the thread keeps his words alone.
Fleet derives "first" from the thread, so nothing is kept for it.

**An aim** (25 Oct 2026) adds `aim?: ModAim` to the body of `send_session_message`, Bridge-sent: `SendSessionMessageBody` is `SendSessionMessage` with `aim` beside its fields, so the JSON is flat and a message built anywhere else carries no aim.
`ModAim` sits beside `StartModSession`: `theme`, `layout`, `words`, `fleet`, `tracker` or `source`.
It is what he picked in an empty Mod Job, and Fleet reads it with that Job's first message only: one line after the brief and before his words, naming the aim in plain words, and for `source` in the mods folder saying that a source change cannot be made there and to tell him so if a mod cannot do it.
On any other message, and on a Job that is not a Mod Job, it is ignored and not an error.
The thread keeps his words alone.

**Refusals**: a 422 `fleet.unacceptable_mod_start` for both `note` and `said`, and for a `said` with nothing in it, and nothing is started; a 500 `fleet.mod_not_written` where the mods folder cannot be made, and nothing is started; a 422 `fleet.unacceptable_mod` for a `session_id` that names no Job Fleet knows, and nothing is made.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the route answers 404, an older Bridge ignores `mod` and `made_by`, and an older Fleet ignores `session_id` and records nothing.

**The event stream.** Untouched: the start publishes `session.changed` as any start and first message do, and a scaffold publishes `mods.changed` as before, now carrying `made_by`.
The open risk below is neither better nor worse.
