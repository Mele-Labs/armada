# Store migrations

A migration has a **name**, not a number, and is **one file**:
`crates/store/migrations/<UTC yyyymmddThhmmZ>-<module.slug>.sql`, for example
`20261007T0130Z-main_ci.state.sql`. The file holds the SQL. A `-- breaking`
line among its leading comments declares it breaking; without one it is
additive. `build.rs` reads the directory at compile time and lists the files in
file-name order, after the first 113, which are the old numbered list under
`module.vN` names in `src/legacy_migrations.rs`, frozen and never edited.

## Adding one

Add a file. **No `armada need`, no number to take, and no shared file to
touch**, so two branches that each add one merge cleanly in git and on GitHub,
which ignores `merge=union`. A name used twice fails the build. Pick the
timestamp when you write it and `module.slug` unique across the list.

**Order between independent branches does not matter.** The store applies by
missing name, never by position, so a branch that lands with an earlier
timestamp than one already applied is still applied. **A migration must not
depend on one from another branch that has not landed**: it may use only
tables that are on `main`.

## What the store records

A table `armada_migrations (name, additive)`, one row per applied name, made by
the store itself and not by a migration. At open the store applies, in the
list's order, every name not in the table. Each is its own transaction and
writes its row with it.

**The count is gone.** `armada_meta.schema_version` is read once, to convert a
file written before names (a file recording `n` is given the first `n` names
without re-running them), and is written only for the first 113 entries so a
build that predates names still reads a fresh file. A numbered file past 113 is
refused: the conversion cannot say which entries it has.

## Additive, and what it buys

**A migration is additive unless it says `Migration::breaking`.** Additive means
it creates tables or indexes, adds columns with defaults, or inserts rows. It
never drops, renames, updates or deletes. `tests::named_migrations` refuses an
additive entry whose SQL does, so the flag is read off the SQL and not off a
promise. 26 of the first 113 are breaking by that test.

**A file that has applied names this build does not list is opened when every
such name was additive**, and Fleet prints them at startup. It is refused, as
before, when one was breaking. That is what makes going back safe: a preview of
unlanded work may apply an additive migration to the real database, and `main`
still opens it. The older build ignores the extra tables and columns.

A breaking migration is a deliberate act: declare it `breaking`, and it lands
only after the migration it follows is on every build that must still open the
file.

## The one file at count 114

A preview applied step-baseline's V115 before names existed, so one database
records 114, which the conversion refuses. `scripts/convert-db-once <db>`, run
once with Fleet stopped, records it under names: it copies the file to
`<db>.before-names`, then in one transaction creates `armada_migrations`, records
the 113 entries and `step_baseline.survives_restart`, and sets `schema_version`
to 113. The step-baseline branch then adds
`20261006T2300Z-step_baseline.survives_restart.sql` in the migrations
directory instead of a list entry. Delete the script once that file is converted.

## Where it is checked

`scripts/restart --from` (the preview) refuses to switch Fleet onto a build whose
list holds names `origin/main` lacks unless all are additive.
`docs/practices/running-locally.md`, *A preview of unlanded work*.
