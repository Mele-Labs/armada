-- The commit a red main was read at, kept while a newer commit's CI runs on top of it. Additive.
ALTER TABLE main_ci ADD COLUMN red_commit TEXT;
