-- The pull request that fixed a reported bug: the merge of one a Job started on the report held. `fixed_at`
-- is when it merged, and all four are empty while the report stands open. Set once, by the first merge.
-- docs/practices/protocol.md, *A reported bug is fixed by the Job started on it*. Additive.
ALTER TABLE bug_reports ADD COLUMN fixed_at           TEXT;
ALTER TABLE bug_reports ADD COLUMN fixed_pull_request INTEGER;
ALTER TABLE bug_reports ADD COLUMN fixed_title        TEXT;
ALTER TABLE bug_reports ADD COLUMN fixed_url          TEXT;
