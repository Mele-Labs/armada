-- The Session that held the pull request that turned main red, by id and no foreign key, as merge_job is. Additive.
ALTER TABLE main_ci ADD COLUMN merge_session TEXT;
