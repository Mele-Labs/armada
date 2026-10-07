# Store migrations

A migration has a **name**, not a number. `crates/store/src/migration_list.rs`
lists them: `Migration::additive("main_ci.state", crate::main_ci::V115),`. The
first 113 are the old numbered list under `module.vN` names, in their order, and
are never edited or reordered.

## Adding one

Append a line at the end of the list. **No `armada need`, no number to take.** The
list is a list file (`docs/practices/list-files.md`), so two branches that each
append one both keep it when `git merge` joins them, in either order, and a
name used twice fails `tests::named_migrations`. Pick `module.slug`, unique
across the list. The migration's own SQL stays beside the table it creates.

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

## Where it is checked

`scripts/restart --from` (the preview) refuses to switch Fleet onto a build whose
list holds names `origin/main` lacks unless all are additive.
`docs/practices/running-locally.md`, *A preview of unlanded work*.
