-- A hosted session that is a repository's Helm, so there is at most one live per Manifest and the
-- dock finds it. 0 for every session that was started before this column and for every ordinary
-- one. docs/concepts/helm.md. Additive.
ALTER TABLE hosted_sessions ADD COLUMN helm INTEGER NOT NULL DEFAULT 0;
