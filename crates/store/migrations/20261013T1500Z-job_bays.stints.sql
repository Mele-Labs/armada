-- Which bay (pool slot) a Job held and when: one row per stint, `until` null while it still holds it.
-- Append-only history beside the session ledger's slot row, which keeps one row per slot and so loses
-- the first stint when a paused Job comes back to the same slot. Written where Fleet takes and gives
-- back a Job's slot; read for `JobSummary.bays`. Additive.
CREATE TABLE job_bays (
    id     INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL REFERENCES jobs(job_id),
    slot   INTEGER NOT NULL,
    since  TEXT NOT NULL,
    until  TEXT
) STRICT;
CREATE INDEX job_bays_by_job ON job_bays (job_id, id);

-- What the ledger already knew: each Job's slot rows, a given-back one closed at the instant it changed.
INSERT INTO job_bays (job_id, slot, since, until)
SELECT a.holder_id, CAST(a.target AS INTEGER), a.since,
       CASE WHEN a.state = 'standing' THEN NULL ELSE a.changed_at END
FROM ledger_attachments a
JOIN jobs j ON j.job_id = a.holder_id
WHERE a.holder_kind = 'job' AND a.kind = 'slot'
ORDER BY a.since;
