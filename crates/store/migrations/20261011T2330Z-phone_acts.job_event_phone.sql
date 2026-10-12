-- Which moves of a Job were a person's press on their phone. Beside job_event_via, whose CHECK
-- cannot be widened without a rebuild; a seq here is read as via = 'phone'.
CREATE TABLE job_event_phone (
    seq    INTEGER PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(job_id)
) STRICT;
