-- breaking
-- A Logbook entry the person holds is locked, not pinned (docs/concepts/logbook.md). The column
-- keeps its values; only the name changes.
ALTER TABLE logbook_entries RENAME COLUMN pinned TO locked;
