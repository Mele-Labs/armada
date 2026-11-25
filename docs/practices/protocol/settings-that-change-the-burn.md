# Settings that change the burn

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 19 Nov 2026. The settings the Usage page changes in place, what a change to each would save over the last day, week or fortnight, and the changes already made with the waste before and since, for the page and for an agent.
**Additive only**: one operation, one route, one setting, one table and the types they carry; no event, no error and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `classic-final:packages/protocol/src`, and there is no number to take.
Bridge's half is `classic-final:packages/protocol/src/usage-adjust.ts`, written by hand.
The new setting, `tokens.subagentCacheTtl`, is one more key of `get_settings` and `save_settings`, which carry it as any other.
**Names and figures only.**
What each row is, how a saving is priced and how a change is read are `docs/concepts/session/where-the-tokens-went.md`, *Settings that change it*.

| Where | Side | Carries |
| --- | --- | --- |
| `get_usage_adjustments` (`GET /usage/adjustments?window=`, `agent_access = "Yes"`) | Bridge reads; an agent reads | `UsageAdjustments`. `window` is `day`, `week` or `fortnight` and defaults to `week`. 422 `fleet.session_unnamed` for any other window |

| Field | Is |
| --- | --- |
| `settings` | A row for each setting the page changes, in a fixed order: `tokens.subagentCacheTtl`, `tokens.compactionWindow`, `timeouts.sessionQuietSeconds`. A later setting is a new row, so a client draws any `key` from its `label` and `choices` |
| `key` | The settings.json path, which `save_settings` takes as the key of a change |
| `label` | The setting's name for a person |
| `now` | What the file holds for the key, or what ships. A word or a number, as `save_settings` takes it |
| `choices` | `{ value, label }`: the values the page offers, `now` among them. A `value` is what `save_settings` takes, and a `label` is what a person reads: `5 min`, `1 hour`, `250k`, `55 min` |
| `suggested` | One of the `choices`' values. Absent where changing would save nothing, or where Fleet cannot say |
| `cuts` | The id of the waste kind the setting cuts, as `get_usage_waste` names it: `cold_cache`, `main_context` or `fresh_starts` |
| `saves_share` | 0 to 1: the share of the window's whole burn a change to `suggested` would save, priced from the window's requests. Absent with `suggested`, and always absent for `timeouts.sessionQuietSeconds` |
| `changes` | The changes to those keys that came into force in the last thirty days, newest first, no more than six: `{ key, label, from, to, at, cuts, before_share, since_share?, since_days }` |
| `from`, `to` | The values either side, as a person reads them. A change of `5m` to `1h` is `5 min` and `1 hour` |
| `before_share` | The `cuts` kind's share of the burn over the seven days before `at`, 0 to 1 |
| `since_share` | The same share from `at` to now. Absent until a day has passed |
| `since_days` | The days from `at` to now, to a tenth |

**Changing a row is `save_settings`.** The page posts `POST /settings/save` with `{ "changes": { "<key>": <value> } }`, one key or several, and Fleet answers with the whole settings list.
`timeouts.sessionQuietSeconds` applies at a restart: `get_settings` answers it with `pending_restart` until Fleet starts again.
`tokens.subagentCacheTtl` and `tokens.compactionWindow` are live, and reach the Jobs Fleet starts after the save.

**Fresh for five minutes, and at once after a change.** Fleet keeps each window's answer in memory and serves it for 5 minutes from when it was made, so `saves_share` and the shares of `changes` move every fifth minute and no faster.
**A change to any of the three keys drops it**, by a save or by a hand edit of settings.json, so the page reads its own change on the next ask: `now` is the new value and `changes` holds it.
A save of a value the key already held changes nothing and drops nothing.
The sums are made on a connection of their own that only reads, so a read made after the five minutes does not hold up any other request.
A refusal is not kept.

**Skew.** A Fleet that predates this answers 404, and the page offers no settings panel; a Bridge that predates it never asks.
A Fleet that predates `tokens.subagentCacheTtl` refuses a save of it as a key it does not have, 422 `fleet.unacceptable_settings`.
