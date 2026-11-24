-- What the model found when the owner pressed Check on a feature's docs, and the calls those presses spent.
-- One row a (feature id, doc) in `feature_doc_checks`: `checked_at` is when the last check of it ran; `doc_quote`,
-- `code_at` and `code_quote` are the disagreement it found, all empty where the doc agrees with the code; `stamp` is git's
-- ids of the doc's text and the feature's paths as that check read them; `dismissed` is the stamp the owner dismissed the
-- disagreement at, which keeps it quiet while a later check reads the same stamp. One row a press in
-- `librarian_check_calls`, which the night's share counts beside the runs' calls. Additive.
-- docs/concepts/feature-catalog.md, *Librarian*.
CREATE TABLE feature_doc_checks (
    feature_id TEXT NOT NULL,
    target     TEXT NOT NULL,
    checked_at TEXT NOT NULL,
    doc_quote  TEXT NOT NULL DEFAULT '',
    code_at    TEXT NOT NULL DEFAULT '',
    code_quote TEXT NOT NULL DEFAULT '',
    stamp      TEXT NOT NULL,
    dismissed  TEXT,
    PRIMARY KEY (feature_id, target)
) STRICT;

CREATE TABLE librarian_check_calls (
    project TEXT NOT NULL,
    night   TEXT NOT NULL,
    at      TEXT NOT NULL,
    calls   INTEGER NOT NULL
) STRICT;
