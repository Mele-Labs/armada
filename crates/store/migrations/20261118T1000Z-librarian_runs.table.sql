-- One row for each run of a project's librarian, which keeps the feature catalog current. `ran_at` is when it ran;
-- `by` is what started it: `press` for a person's press, `after_merges` or `nightly` for Fleet under the setting of
-- that name. `through_pr` and `through_at` are the newest merged pull request the run read through and when it merged,
-- 0 and '' where the project had none. `at_commit` is the commit its checkout stood at, '' where git would not say.
-- `parts` are the names of the parts that ran, joined by commas, '' where none did. Additive.
CREATE TABLE librarian_runs (
    project     TEXT NOT NULL,
    ran_at      TEXT NOT NULL,
    by          TEXT NOT NULL,
    through_pr  INTEGER NOT NULL DEFAULT 0,
    through_at  TEXT NOT NULL DEFAULT '',
    at_commit   TEXT NOT NULL DEFAULT '',
    parts       TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (project, ran_at)
) STRICT;
