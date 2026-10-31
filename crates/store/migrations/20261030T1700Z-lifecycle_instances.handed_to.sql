-- The Job an instance was handed to, empty for every other. A Job that starts another for a plan group
-- hands the group's ticket to it: the instance ends here at once (`ended_at`, so every reader that
-- skips an ended one skips it) and this says it did not end because the Job did, nor reach an end state
-- of its own. The Job it went to is the one that holds the ticket from then on. Additive: it defaults to
-- empty, and a row kept before this was never handed.
ALTER TABLE lifecycle_instances ADD COLUMN handed_to TEXT NOT NULL DEFAULT '';
