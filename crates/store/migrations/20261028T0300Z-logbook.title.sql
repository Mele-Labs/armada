-- A few words naming an entry, as its line in the Logbook reads (docs/concepts/logbook.md). NULL
-- for an entry written before titles were, until a Distil run is shown it and names it. Additive.
ALTER TABLE logbook_entries ADD COLUMN title TEXT;
