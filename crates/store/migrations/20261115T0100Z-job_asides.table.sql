-- What the owner asked a Job on the side (`ask_aside`) and what a read-only copy of its conversation answered, kept
-- on this Mac with the Job's record and **never in its thread**, so the Job's agent does not see it unless the owner
-- tells it (`told_at`). `state` is `asking`, `answered` or `failed`, and `failed` carries Fleet's sentence for why.
-- `quote` is the text the owner selected in the thread, where he did. `answer` is what has arrived, so it grows while
-- the state is `asking`. No foreign key: an aside outlives its Job. Additive.
CREATE TABLE job_asides (
    aside_id    TEXT NOT NULL PRIMARY KEY,
    session_id  TEXT NOT NULL,
    question    TEXT NOT NULL,
    quote       TEXT,
    answer      TEXT NOT NULL,
    state       TEXT NOT NULL,
    failed      TEXT,
    asked_at    TEXT NOT NULL,
    answered_at TEXT,
    told_at     TEXT
) STRICT;
CREATE INDEX job_asides_by_session ON job_asides (session_id, asked_at);
