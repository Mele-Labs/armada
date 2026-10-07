-- The newest finished commit main's state was decided from, and how many newer commits are still running. Additive.
ALTER TABLE main_ci ADD COLUMN decided_commit TEXT;
ALTER TABLE main_ci ADD COLUMN newer_running INTEGER NOT NULL DEFAULT 0;
