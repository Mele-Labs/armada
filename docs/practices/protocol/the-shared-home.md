# The shared home: Kit's rows against it, three acts, and what syncs

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 23 Oct 2026. `docs/concepts/kit.md`, *The shared home*, and `docs/concepts/account.md`, *What syncs*.
**Additive**: eight operations, eight routes, twelve error codes and the types they carry; no event kind.
Since 25 Oct 2026 `set_home_repository`, `planned` and `Profile`'s `orgs` and `home` are added, and the protocol ID moved with them.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/shared-home.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `get_shared_home` (`GET /shared_home?login=`, `agent_access = "No"`) | Bridge asks, Fleet answers 200 | `SharedHomeRead` `{ home?, planned?, rows, unreadable }`: `home` `{ account, repository }`, left out with no copy here; `planned` `HomePlanned` `{ account, repository, exists }`, where it is or will be, served whenever `home` is left out; each row `HomeRow` `{ kind, name, state, says?, source?, repository?, key_here?, mod_kind?, here?, there? }` |
| `sync_shared_home` (`POST /shared_home/sync`, `No`) | Bridge sends, Fleet answers 200 | `HomeProfile` `{ login }`; makes the home where it does not exist, clones or pulls it, adds every row only on this machine of each kind switched on, and each item switched on on its own, in one commit and one push, and answers `HomeSynced`: `SharedHomeRead` flattened, and `left`, each `HomeLeft` `{ kind, name, repository?, why }` a row only here that stayed, `why` Fleet's sentence naming it |
| `add_to_shared_home` (`POST /shared_home/add`, `No`) | Bridge sends, Fleet answers 200 | `HomeRowAt` `{ login, kind, name, repository? }`; answers `SharedHomeRead` |
| `bring_from_shared_home` (`POST /shared_home/bring`, `No`) | Bridge sends, Fleet answers 200 | `HomeRowAt`; answers `SharedHomeRead` |
| `choose_shared_home_side` (`POST /shared_home/choose`, `No`) | Bridge sends, Fleet answers 200 | `ChooseHomeSide` `{ row, side }`, `side` `here` or `there`; answers `SharedHomeRead` |
| `get_home_kinds` (`GET /shared_home/kinds?login=`, `No`) | Bridge asks, Fleet answers 200 | `HomeKinds` `{ login, kinds, items }`, one `{ kind, on }` a kind, every one off until turned on, and `items`, each `HomeItem` `{ kind, name, repository? }` switched on on its own |
| `set_home_kind` (`POST /shared_home/kinds/set`, `No`) | Bridge sends, Fleet answers 200 | `SetHomeKind` `{ login, kind, on }`; answers `HomeKinds` |
| `set_home_item` (`POST /shared_home/items/set`, `No`) | Bridge sends, Fleet answers 200 | `SetHomeItem` `{ login, kind, name, repository?, on }`; an item on has its row and is added by Sync while its kind is off; answers `HomeKinds` |
| `set_home_repository` (`POST /shared_home/repository`, `No`) | Bridge sends, Fleet answers 200 | `SetHomeRepository` `{ login, owner, name }`; keeps where the first Sync makes the home and answers `SharedHomeRead` |

**Mapping onto Kit.** A `HomeRow` is Bridge's `SharedRow` with `key_here` in place of `keyHere` and inverted: Kit's `keyHere: true` draws the key mark, and the wire's `key_here: false` is a server whose key is not here.
`mod_kind` is Kit's `modKind`, the kind a mod's `mod.toml` names, this machine's where it has the mod and else the home's, so a mod only the home has is listed under its own kind.
`syncing` is Bridge's own while `sync_shared_home` is in flight.
A memory row's act carries its `repository`, which Kit's row does not, so Bridge keeps each drawn row's wire row beside it.
Bridge calls the nine through main, `apps/desktop/src/main/accounts.ts`, and draws them from `renderer/src/fleet-shared-home.ts` and `fleet-account.ts`.

**A row that differs is never merged.** Add and Bring refuse it with a 409 `fleet.home_row_differs`; only `choose_shared_home_side` replaces a side, and it takes away the files the kept side has not.
Sync passes it by, and Bring stays the person's act.

**Sync adds as Add does, and refuses nothing for one row.** Each row only on this machine is written as `add_to_shared_home` writes one, and a row Add would refuse, for a credential, a mod that is not valid here or a home that cannot take it, is named in `left` while the rest go.
Bridge tells each `left` row's `why` as a failure toast of its own, and never counts them.

**Nothing secret leaves.** A server is carried as its name and host.
Add and choosing `here` refuse a piece whose text holds an address with a user and password or a credential in its query, with a 422 `fleet.home_secret_refused` that names the host and never the value: refused, not scrubbed, so a scrubbed copy never reads as differing for good.

**A home is a profile's.** A Mac can hold a personal profile and a work one, each with its own home, so every operation names the profile's `login`, and the forge is asked as that login, never as the active account.

**Where it is made is chosen first.** `set_home_repository` keeps an owner, the login or one of its organisations, and a name, a profile at a time in this machine's store; Sync makes the home there, `<login>/armada-home` until one is chosen, and marks it with the `armada-home` topic.
With no copy here and nothing chosen, Fleet asks the forge once a run for a repository with that topic under the login or its organisations, and one found is served as `planned` with `exists` set and kept as the choice.
The copy here is taken as the profile's where its origin's owner is the login or one of its organisations.
What syncs is kept a profile at a time.

**Refusals**: a 404 `fleet.no_such_profile` for a login that is not a profile here, a 422 `fleet.home_choice_refused` for an owner that is neither the login nor one of its organisations or a name the forge would not take, a 409 `fleet.home_already_made` for a choice once the home is here or was found, a 422 `fleet.home_login_not_held` where the forge here does not hold the login, naming it and how to add it, a 404 `fleet.no_such_home_row`, a 409 `fleet.no_home_yet` for an act with no copy here, a 422 `fleet.home_refused` for a server brought (its key is not in the home), a memory note whose repository is not served here, a mod that is not valid, or a forge that refused, including an organisation that will not let the login make the home and a home the login cannot push to, a 500 `fleet.home_unreachable`, a 500 `fleet.home_not_kept`, and a 409 `fleet.home_not_served` for a Fleet started without the home.

**`get_accounts`.** Each `Profile` carries `orgs`, read at sign-in, and `home` `ProfileHome` `{ account, repository, made }`: the copy here, else where it will be made.
It never asks the forge, so a home found on another Mac shows there once `get_shared_home` has run.
A sign-in's `setup` flow reads the home's row `done`, and names the repository, once this machine holds a copy of that login's home, and waits while the forge here does not hold the login.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves. Were a pair to meet anyway, a Fleet without the routes answers 404.

**The event stream.** Untouched: none of the nine publishes an event, so the open risk below is no better and no worse.
