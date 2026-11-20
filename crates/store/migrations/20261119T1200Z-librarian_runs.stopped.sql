-- Whether Stop was pressed on a run before it ended, 1 where it was: the steps after the one running then were skipped.
-- Additive: a row kept before this reads as a run that was not stopped. docs/concepts/feature-catalog.md, *Librarian*.
ALTER TABLE librarian_runs ADD COLUMN stopped INTEGER NOT NULL DEFAULT 0;
