# Kit over the Mac link: each kind's switch, each row against every linked Mac, and a clash settled

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 19 Nov 2026. `docs/concepts/kit.md`, *Over the Mac link*.
**Additive**: four operations Bridge calls, three the Gateways call between Macs, the `kit.changed` event, two error codes and the types they carry.
The shared home's routes stay and answer for Classic, [The shared home](the-shared-home.md).
Bridge's half is `classic-final:packages/protocol/src/kit-link.ts`, written by hand; what one Mac offers another crosses between Fleets and Gateways only and is not mirrored.
Native Bridge draws it from `crates/ipc/src/kit_link.rs` directly.

| Where | Side | Carries |
| --- | --- | --- |
| `get_kit_link` (`GET /kit/link`, `agent_access = "No"`) | Bridge asks, Fleet answers 200 | `KitLink { kinds, items, macs, rows, unreadable }`. `kinds` one `{ kind, on }` per kind, off until switched on. `macs` each `KitLinkedMac { mac, name, crossed_at?, kinds, items }`. Each row `KitLinkRow { kind, name, repository?, says?, source?, key_here?, mod_kind?, refused?, against }`, `against` one `KitAgainst { mac, mark, here?, there? }` per linked Mac that crosses it; `mark` is `same`, `differs`, `new_here`, `only_there` or `clash`, and a row that differs or clashes carries both sides as `KitSide { mac_name, text, changed_at? }` |
| `set_kit_link_kind` (`POST /kit/link/kinds/set`, `No`) | Bridge sends, Fleet answers 200 | `SetKitLinkKind { kind, on }`; answers `KitLink` |
| `set_kit_link_item` (`POST /kit/link/items/set`, `No`) | Bridge sends, Fleet answers 200 | `SetKitLinkItem { kind, name, repository?, on }`; answers `KitLink` |
| `settle_kit_clash` (`POST /kit/link/settle`, `No`) | Bridge sends, Fleet answers 200 | `SettleKitClash { mac, kind, name, repository?, keep }`, `keep` `here` or `there`; answers `KitLink`. 409 `fleet.kit_no_clash` for a row that is not a clash with that Mac, 404 `fleet.other_mac_not_linked` for a Mac this one holds no Kit from |
| `share_kit` | the Gateway calls | `GET /other_mac/kit`, answered `KitOffer { name, kinds, items, pieces }`, each `KitPiece { kind, name, repository?, files, says?, source?, hash, changed_at?, agreed? }`, cut by `crate::share`: no piece that carries a credential. `agreed` is the offering Mac's marker with the Mac named in `x-armada-mac`, which the Gateway sends on this read |
| `hold_other_mac_kit` | the Gateway calls | `PUT /other_macs/kit`, body `OtherMacKit { mac, kit }`, kept in place of what that Mac last offered, then taken. Answers `MacNamed` |
| `kit_settle_from_mac` | the other Mac's Gateway calls | `POST /other_mac/kit_settle`, body `KitSettled { kind, name, repository?, agreed }`, header `x-armada-mac`; answers 204 |
| `kit.changed` | Fleet sends | Carries nothing. Bridge reads `get_kit_link` again; it crosses to the linked Macs, whose Gateways read `share_kit` again and hand it in, and is not handed to their Fleets as an event |

**The exchange.** Each Gateway reads the other Mac's `share_kit` when the link opens, on every resync, and on each `kit.changed` it carries, and hands it to its own Fleet as `hold_other_mac_kit`.
**Each Mac writes only to itself**: the Fleet that holds an offer takes from it what changed on the other Mac only, and the other Mac takes the rest from this one's offer on its own exchange.
A change made in the harness's home outside Armada is read on the Mac's next Kit read, `get_kit_link` or an exchange, and where this Mac's offer moved, `kit.changed` tells the linked Macs.

**The last-agreed marker.** Fleet keeps, per linked Mac and item, the hash of the item's canonical text both held when they last agreed (`kit_link_agreed` in the store).
Each offered piece carries the offering Mac's own marker with the asker, so a Mac that took an edit and changed the item again before the first Mac read the take still reads as the only one that moved.
Against the two a change on one side is told from a change on both: the table is `crates/fleet/src/kit_link.rs`'s module doc.
`changed_at` is when this Mac first read the version (`kit_link_seen`), not the file's own time.
Settling a clash records the hash of the side not kept as the marker on both Macs, so the kept side reads as the only change and crosses.
A settle the other Mac cannot be reached for waits in the queue project acts use, `kind` `kit`, and is sent oldest first when its link opens.

**What is not done.** A deletion does not cross, and is not brought back either: the marker says the item was agreed, so an item only on the other Mac is left there and marked.
A mod is checked for validity on the sending Mac only.
A server is never written: it crosses as its name and host, and its key is added on each Mac in Kit.
