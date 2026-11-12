-- A quick terminal (`start_terminal_job`) opens onto its terminal and its agent waits for the person,
-- and the record keeps that, so a restarted Fleet still lists it as one. `terminal_dir` is the folder
-- under its repository's root its terminals open in, where it was started in one.
-- Additive: 0 and NULL on every other row.
ALTER TABLE hosted_sessions ADD COLUMN quick_terminal INTEGER NOT NULL DEFAULT 0;
ALTER TABLE hosted_sessions ADD COLUMN terminal_dir TEXT;
