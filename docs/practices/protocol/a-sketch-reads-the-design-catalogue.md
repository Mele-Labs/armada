# A sketch reads the design catalogue: `get_design_catalogue`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 13 Nov 2026. [Drawing a sketch](../../concepts/session/drawing-a-sketch.md), *The design catalogue*.
**Additive only**: one operation, one route and the types it answers; no event kind and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
The mirror is `packages/protocol/src/design-catalogue.ts`, written by hand; Bridge does not call the route.

| Where | Side | Carries |
| --- | --- | --- |
| `get_design_catalogue` (`GET /sessions/design_catalogue?area=&match=&repository=&session_id=`, `agent_access = "Yes"`) | Job reads | `DesignCatalogue` `{ area, themes[]?, sheets[]?, contents?, tokens[]?, components[]?, icons[]?, patterns[]?, cut? }`, only the asked area filled |
| `area` | Job names | One of `contents` (the default), `tokens`, `components`, `icons`, `patterns`; another word is a 422 `fleet.catalogue_area_unknown` |
| `tokens[]` | Fleet answers | `TokenGroup` `{ role, family, tokens[] }`, `role` one of `colour`, `space`, `type`, `radius`, `shadow`, `motion`, `other`; each `CatalogueToken` `{ name, value, alias?, themes? }` |
| `components[]` | Fleet answers | `CatalogueComponent` `{ name, file, sheet?, classes[], sample? }` |
| `patterns[]` | Fleet answers | `CataloguePattern` `{ name, note?, html }`, from `.armada/catalogue/patterns.html` |
| `contents` | Fleet answers | `CatalogueContents` `{ tokens[] of { role, family, tokens }, components[], patterns[], icons }` |

**The answer stays under 24 KB**, so it fits beside a sketch's own 32 KiB, and `cut` says what was left out and how to ask for it.
**Nothing rides on a record or an event.** Fleet keeps each checkout's catalogue in memory, read again only when the fingerprint of its inputs moves.
No `session_id` and no placed caller, with no `repository`, is a 422 `fleet.catalogue_unplaced`; a `repository` Fleet does not serve is a 404.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream is neither better nor worse.**
