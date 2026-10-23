# The shared home: Kit's rows against it, three acts, and what syncs

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 23 Oct 2026. `docs/concepts/kit.md`, *The shared home*, and `docs/concepts/account.md`, *What syncs*.
**Additive**: seven operations, seven routes, ten error codes and the types they carry; no event kind.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/shared-home.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `get_shared_home` (`GET /shared_home?login=`, `agent_access = "No"`) | Bridge asks, Fleet answers 200 | `SharedHomeRead` `{ home?, rows, unreadable }`: `home` `{ account, repository }`, left out with no copy here; each row `HomeRow` `{ kind, name, state, says?, source?, repository?, key_here?, here?, there? }` |
| `sync_shared_home` (`POST /shared_home/sync`, `No`) | Bridge sends, Fleet answers 200 | `HomeProfile` `{ login }`; makes the home where it does not exist, clones or pulls it, and answers `SharedHomeRead` |
| `add_to_shared_home` (`POST /shared_home/add`, `No`) | Bridge sends, Fleet answers 200 | `HomeRowAt` `{ login, kind, name, repository? }`; answers `SharedHomeRead` |
| `bring_from_shared_home` (`POST /shared_home/bring`, `No`) | Bridge sends, Fleet answers 200 | `HomeRowAt`; answers `SharedHomeRead` |
| `choose_shared_home_side` (`POST /shared_home/choose`, `No`) | Bridge sends, Fleet answers 200 | `ChooseHomeSide` `{ row, side }`, `side` `here` or `there`; answers `SharedHomeRead` |
| `get_home_kinds` (`GET /shared_home/kinds?login=`, `No`) | Bridge asks, Fleet answers 200 | `HomeKinds` `{ login, kinds }`, one `{ kind, on }` a kind, every one off until turned on |
| `set_home_kind` (`POST /shared_home/kinds/set`, `No`) | Bridge sends, Fleet answers 200 | `SetHomeKind` `{ login, kind, on }`; answers `HomeKinds` |

**Mapping onto Kit.** A `HomeRow` is Bridge's `SharedRow` with `key_here` in place of `keyHere` and inverted: Kit's `keyHere: true` draws the key mark, and the wire's `key_here: false` is a server whose key is not here.
`syncing` is Bridge's own while `sync_shared_home` is in flight.
A memory row's act carries its `repository`, which Kit's row does not, so Bridge keeps each drawn row's wire row beside it.
Bridge calls the seven through main, `apps/desktop/src/main/accounts.ts`, and draws them from `renderer/src/fleet-shared-home.ts` and `fleet-account.ts`.

**A row that differs is never merged.** Add and Bring refuse it with a 409 `fleet.home_row_differs`; only `choose_shared_home_side` replaces a side, and it takes away the files the kept side has not.

**Nothing secret leaves.** A server is carried as its name and host.
Add and choosing `here` refuse a piece whose text holds an address with a user and password or a credential in its query, with a 422 `fleet.home_secret_refused` that names the host and never the value: refused, not scrubbed, so a scrubbed copy never reads as differing for good.

**A home is a profile's.** A Mac can hold a personal profile and a work one, each with its own `<login>/armada-home`, so every operation names the profile's `login`, and the forge is asked as that login, never as the active account.
What syncs is kept a profile at a time.

**Refusals**: a 404 `fleet.no_such_profile` for a login that is not a profile here, a 422 `fleet.home_login_not_held` where the forge here does not hold the login, naming it and how to add it, a 404 `fleet.no_such_home_row`, a 409 `fleet.no_home_yet` for an act with no copy here, a 422 `fleet.home_refused` for a server brought (its key is not in the home), a memory note whose repository is not served here, a mod that is not valid, or a forge that refused, a 500 `fleet.home_unreachable`, a 500 `fleet.home_not_kept`, and a 409 `fleet.home_not_served` for a Fleet started without the home.

**`get_accounts`.** A sign-in's `setup` flow reads the home's row `done`, and names the repository, once this machine holds a copy of that login's own home, and waits while the forge here does not hold the login.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves. Were a pair to meet anyway, a Fleet without the routes answers 404.

**The event stream.** Untouched: none of the seven publishes an event, so the open risk below is no better and no worse.
