# Protocol 17.2: Kit reads the setup a person already has

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1491`. `get_kit_inventory` — one new query at `GET /kit/inventory`, and `KitInventory` with
`SetupKindRow`, `WhatWasRead`, `SetupItem` and `SetupUnreadable` beside it. Additive: a new route
and new DTOs, nothing renamed and nothing an older peer already reads changed.

**A Bridge that does not know the route draws Kit exactly as it draws it today** — the servers
table and the form — which is what that build can truthfully say about a read it cannot make.

**`harness` is the one vendor word on this wire**, and it crosses as data an adapter produced
rather than as a literal Bridge holds. A second harness is a second adapter and this DTO does not
move.

**Nothing on it can widen a Drone.** A row is a name, the item's own words for itself and where it
came from; a connected server carries the program's own file name or the host it is at, and never
the argument list, query string, userinfo or environment that follow either. What crosses could not
start the server it names, so one connected outside Armada is visible here and still reaches no
Drone. `add_kit_server` and `set_kit_server_reach` are the two separate acts they were.

**The field is the same shape either way.** `says` was optional and a string when this was written
and still is; what changed on the owner's word is what an adapter puts in it, which no peer parses
differently. 17.2 stands.
