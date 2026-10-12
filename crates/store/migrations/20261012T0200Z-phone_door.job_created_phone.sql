-- Which Jobs a person dispatched from their phone. Creation is no move, so job_event_phone cannot
-- hold it; a row here is read as the Job's door being the phone, and no row as Bridge or no one.
CREATE TABLE job_created_phone (
    job_id TEXT PRIMARY KEY REFERENCES jobs(job_id)
) STRICT;
