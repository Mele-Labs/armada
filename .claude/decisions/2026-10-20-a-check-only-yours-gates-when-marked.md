# Does a Check that only your own armada.yml declares gate your Jobs?

**Decided 2026-10-20.** Where the repository has an `armada.yml` and his copy in Kit names a Check or Command the repository does not, that name used to be left out as `not_read`.

**Chosen: it is added and available, and a Check gates only where he marks it `gate: true`.** A Command it adds is available likewise.

| Asked | He chose | The cost he took |
|---|---|---|
| A Check or Command only his copy names | Added to the Manifest his Jobs run against, no longer `not_read` | His copy now changes what his Jobs can run, beyond the names it replaces |
| Whether such a Check gates | Not unless he marks it `gate: true` | A Check of his own does not protect his Jobs until he marks it |
| A Check his copy replaces by name | Gates as the repository's did | None |
| CI | Unchanged: CI never sees Kit | A Check he marked gates his Jobs on this Mac and not on CI |

**The key.** No existing key kept a Check out of a gate, and `runs_at` says where a Check runs rather than whether it gates, so one optional key, `gate`, was added on a Check. It is read only on a Check that only Kit's copy declares, and refused on one the repository declares.
