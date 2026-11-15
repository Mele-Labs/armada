-- What a Ready to try card shows and how he answered it, kept on the build note of the (branch, tip) it is. The
-- artifact he approved is kept as it read at the offer (`approved_*`); `pictures` a JSON array of paths; `words` his
-- words that asked for the work. `later_until` is when a card he asked to be reminded of comes back, and `back_at`
-- when it last did, which is where the thread draws it. `withdrawn_at` is when the Job took the offer back. Additive.
ALTER TABLE build_notes ADD COLUMN approved_artifact TEXT;
ALTER TABLE build_notes ADD COLUMN approved_title TEXT;
ALTER TABLE build_notes ADD COLUMN approved_form TEXT;
ALTER TABLE build_notes ADD COLUMN approved_revision INTEGER;
ALTER TABLE build_notes ADD COLUMN approved_at TEXT;
ALTER TABLE build_notes ADD COLUMN pictures TEXT NOT NULL DEFAULT '[]';
ALTER TABLE build_notes ADD COLUMN words TEXT;
ALTER TABLE build_notes ADD COLUMN later_until TEXT;
ALTER TABLE build_notes ADD COLUMN back_at TEXT;
ALTER TABLE build_notes ADD COLUMN withdrawn_at TEXT;
