-- A repair Fleet hosts beside a broken Job: an agent process with no session of its own, one turn in
-- the Job's checkout. `id` is the repair's own, `broke` the kind of break (`quiet`, `hung`, `died`,
-- `checkout`, `red_pull` or `stranded`), `outcome` how it ended (`repaired`, `not_repaired`,
-- `unmarked`, `timed_out`, `stopped`, `failed` or `abandoned`) and `said` the sentence its first line
-- carried, or why it did not. `stop_nudges.repair_job` now holds a repair's id. `stop_nudges.repairs`,
-- which said a Job was a repair, is no longer written or read, and stays so an older build opens this
-- store. Additive.
CREATE TABLE job_repairs (
    id         TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    broke      TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at   TEXT,
    outcome    TEXT,
    said       TEXT
) STRICT;

CREATE INDEX job_repairs_by_session ON job_repairs (session_id);
