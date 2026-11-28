# Areas and the catalog's rules: `change_area`, `reject_in_catalog`, `list_catalog_rules` and `drop_catalog_rule`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 28 Nov 2026. `docs/concepts/feature-catalog.md`, *Areas* and *His rules*.
**Additive only**: four operations and routes, one optional field on each of `ChangeFeature` and `FeatureChanged`, and new types; no event kind.
There is no number to bump: the protocol ID moves with `crates/ipc/src`, the operation files and the route table.
Only the native Bridge reads it.

| Where | Side | Carries |
| --- | --- | --- |
| `change_area`, `POST /features/areas/change` | Bridge sends `ChangeArea`, Fleet answers `AreaChanged` | `project`, `area`, `change` and `why`. `change` is an `AreaChange` tagged by `act`: `make` `{title}`, `rename` `{slug?, title?}`, `split` `{slug, title, features}`, `merge` `{into}`. The answer: `project`, `area` as it is now, and `rule`, the id of the rule `why` became |
| `reject_in_catalog`, `POST /features/reject` | Bridge sends `RejectInCatalog`, Fleet answers `CatalogRuleKept` | `project`, `reject` (`not_this_feature` or `not_together`), `features`, `areas`, `paths` and `why`. The answer: `project` and the `CatalogRule` kept |
| `list_catalog_rules`, `GET /features/rules?project=` | Fleet answers `CatalogRules` | `project` and `rules`, newest first, each a `CatalogRule`: `id`, `text`, `act` (a `CatalogRuleAct`), `features`, `areas`, `paths` and `at` |
| `drop_catalog_rule`, `POST /features/rules/drop` | Bridge sends `DropCatalogRule`, Fleet answers `CatalogRules` | `project` and `rule`, its id. The answer is the rules left |
| `ChangeFeature.why`, `FeatureChanged.rule` | Bridge sends, Fleet answers | His words for a merge, rename or move, and the id of the rule they became. Absent, no rule is kept |

**A split is one call**: Fleet keeps the new area right after the one split and moves the named features in one transaction, where Bridge sent `upsert_features` with every area and then a `change_feature` for each feature.
The native Features screen still sends those two until its gallery is built.
**`act` is a closed set**: `make_area`, `rename_area`, `split_area`, `merge_areas`, `merge_features`, `rename_feature`, `move_feature`, `not_this_feature` and `not_together`.
A row whose act a Fleet does not know is left out of the list.

**The acts are the person's**, as `change_feature` is: the Operator through the door, Bridge on the port, and a 403 `api.the_persons_own` from inside a Job.
The read is Bridge's alone: the librarian and the Operator are handed the rules in what Fleet gives them.
Refusals are in the operation files: `fleet.area_change`, `fleet.area_slug_taken`, `fleet.catalog_reject` and `fleet.catalog_rule_unknown` are new.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** No new event: Bridge reads the library or the rules again after a press.
