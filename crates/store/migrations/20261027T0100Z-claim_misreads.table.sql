-- The sentences of a reply that set off a walk or done reminder a job disputed and the model agreed
-- asked nothing, one row each for good: the reminder holds near ones off in every Session. Additive.
CREATE TABLE claim_misreads (
    sentence   TEXT NOT NULL,
    claim      TEXT NOT NULL,
    session_id TEXT NOT NULL,
    marked_at  TEXT NOT NULL,
    PRIMARY KEY (sentence, claim)
) STRICT;
