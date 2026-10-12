-- Sleep mode: a decision the owner has kept or changed is reviewed, and leaves the review.
ALTER TABLE sleep_rows ADD COLUMN reviewed INTEGER NOT NULL DEFAULT 0;
