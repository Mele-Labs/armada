-- Where the sorter placed an entry: git, slots, testing, tools or mac (docs/concepts/logbook.md). NULL
-- until it has, and shown as Other. Additive.
ALTER TABLE logbook_entries ADD COLUMN topic TEXT;
