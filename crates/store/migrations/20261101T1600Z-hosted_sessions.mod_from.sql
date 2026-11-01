-- A Mod Job started from a Custom tab keeps the Job whose panes hold the tab and the tab's id,
-- so Bridge gives it no row of its own. Additive: both columns are null on every other row.
ALTER TABLE hosted_sessions ADD COLUMN from_job TEXT;
ALTER TABLE hosted_sessions ADD COLUMN from_tab TEXT;
