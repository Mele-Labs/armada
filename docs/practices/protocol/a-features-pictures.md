# A feature's pictures: `add_feature_picture`, `remove_feature_picture`, `get_feature_picture`, `pictures` and `cover`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 28 Nov 2026. `docs/concepts/feature-catalog.md`, *Pictures*.
**Additive only**: three operations and routes, two optional fields on existing types and one librarian part; no event kind.
There is no number to bump: the protocol ID moves with `crates/ipc/src`, the operation files and the route table.
Only the native Bridge reads it, and it draws none of it yet; `classic-final:packages/protocol` mirrors no feature type.

| Where | Side | Carries |
| --- | --- | --- |
| `add_feature_picture`, `POST /features/pictures/add` | A Job sends `AddFeaturePicture`, Fleet answers `FeaturePictureAdded` | `feature`, `path` (a PNG on this Mac), `url` or `screen`, and `project` and `session_id` as `write_feature_sections` has them. The answer: `feature` and `picture` |
| `remove_feature_picture`, `POST /features/pictures/remove` | Bridge sends `RemoveFeaturePicture`, Fleet answers `FeaturePictureRemoved` | `picture` and an optional `why`. The answer: `feature`, `picture` and `cover`, absent where none is left |
| `get_feature_picture`, `GET /features/picture?id=` | Bridge asks, Fleet answers the bytes | `image/png`, with `nosniff`. A removed picture is served too |
| `FeaturePage.pictures` (`get_feature_page`) | Fleet writes, Bridge reads | The standing pictures, newest first, each a `FeaturePicture`: `id`, `url` or `screen`, `commit`, `taken_at`, `width`, `height`, `bytes`, `by` (`job` or `librarian`) and `job`. Absent where none, and on a read by `section` |
| `FeatureRow.cover` (`list_features`, and the page's `feature`) | Fleet writes, Bridge reads | The newest standing picture's id. Absent where none |
| `LibrarianPart::Pictures`, step `take_pictures` | Fleet writes, Bridge reads | `pictures` in `features.librarianParts` and in a run's `ran`; its step on `list_features` and `librarian.step` |

**Who calls what.** `add_feature_picture` is a Job's, placed by its connection, and the door offers it to a Job and not the Operator.
`remove_feature_picture` is the person's, as `answer_retire` is: the Operator through the door, Bridge on the port, and a 403 `api.the_persons_own` from inside a Job.
Its refusals are 404 `fleet.picture_unknown`, and for the add 422 `fleet.feature_unknown`, `fleet.picture_source` and `fleet.picture_unreadable`.

**`why` is a seam.** Fleet keeps it with the removal and reads nothing from it yet; the catalog's rules are where it is meant to be read.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
An older Fleet's page has no `pictures` and its rows no `cover`, which a newer Bridge reads as a feature with no picture.
A Bridge that reads `features.librarianParts` and does not know `pictures` leaves it out.
**The event stream.** No new event: the page read after a run, a Job's add or a removal carries the pictures.
