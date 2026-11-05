-- The agent's word that a plan group changes what the person sees, kept per group: whether the
-- Operator may answer a question from a Job working on it. A plan marked seen as a whole, before
-- this, reads as every group seen.
ALTER TABLE session_plan_groups ADD COLUMN seen INTEGER NOT NULL DEFAULT 0;
