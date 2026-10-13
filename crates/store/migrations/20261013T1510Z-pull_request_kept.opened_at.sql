-- When Fleet opened a Job's pull request, kept past the merge as its title is. Null for a pull request
-- opened before this column, and for one a person opened. An open pull request's own `opened_at` is the
-- hub's; this is what outlives it. Additive.
ALTER TABLE jobs ADD COLUMN delivery_pr_opened_at TEXT;
