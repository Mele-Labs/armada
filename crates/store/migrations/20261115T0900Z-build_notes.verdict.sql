-- What the owner made of a note when he looked at it: `good` or `needs_work`, absent for a note not looked at or
-- marked before he could say. A later mark changes it; `looked_at` keeps the first time. Additive.
ALTER TABLE build_notes ADD COLUMN verdict TEXT;
