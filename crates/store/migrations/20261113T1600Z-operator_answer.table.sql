-- The person's own answers to a Job's question, kept so the Operator can follow one that already covers a new
-- question in the same project. One row for each answer he gave: from the Job's list, from the Operator's page, or
-- in a Job's thread (`via` is `job`, `operator_page` or `thread`). `question` is the item's text, `options` the
-- labels it offered as a JSON array, `choice` the label he pressed and `words` what he wrote. An answer the
-- Operator gave in his place is never a row. Additive.
CREATE TABLE operator_answer (
    n INTEGER PRIMARY KEY AUTOINCREMENT,
    session TEXT NOT NULL,
    manifest_id TEXT NOT NULL,
    job TEXT,
    item_id TEXT NOT NULL,
    question TEXT NOT NULL,
    options TEXT NOT NULL,
    choice TEXT,
    words TEXT,
    answered_at TEXT NOT NULL,
    via TEXT NOT NULL
);
CREATE INDEX operator_answer_manifest ON operator_answer (manifest_id, n);
