-- breaking
-- A quick terminal is a Side Job since 13 Nov 2026, when the owner renamed it everywhere, the stored
-- column too. Each row keeps its value under the new name.
ALTER TABLE hosted_sessions RENAME COLUMN quick_terminal TO side_job;
