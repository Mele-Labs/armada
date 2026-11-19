-- What the owner settled in a Job: his answer to a question, his pick on takes, his Approve or Ask for changes, his Looks
-- good. One row a decision, keyed by the Job and by `key`, which names the act (a call and its question, an artifact and
-- the revision he answered), so Fleet hearing the same answer twice keeps one row. `artifact` is the target of the
-- sketch, page, draft or walk he answered, empty where none: the features it names are read from `feature_links`, so a
-- rename or a merge of a feature moves its decisions with its links. Additive.
CREATE TABLE feature_decisions (
    job      TEXT NOT NULL,
    key      TEXT NOT NULL,
    artifact TEXT NOT NULL DEFAULT '',
    question TEXT NOT NULL,
    choice   TEXT NOT NULL,
    note     TEXT NOT NULL DEFAULT '',
    at       TEXT NOT NULL,
    PRIMARY KEY (job, key)
) STRICT;
CREATE INDEX feature_decisions_by_at ON feature_decisions (at);
