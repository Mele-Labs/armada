-- What the librarian's upkeep step found of a feature, and of a linked doc, against the code at HEAD. One row a
-- (project, feature slug) in `feature_upkeep`: `kind` is `planned` (its paths match no file and never did), `retire_gone`
-- (they matched some once and match none now) or `retire_unused` (files, but nothing touched them and no Job was on it
-- for months); `evidence` is the one line a retire proposal rests on; `checked_at` is when the check ran; `kept_at` is
-- when the owner answered Keep, which holds while a later check finds the same kind and evidence. A check that finds
-- nothing clears the row. One row a (project, doc) in `feature_doc_gone`: a linked doc that is a file of the repository
-- and is not at HEAD. Additive. docs/concepts/feature-catalog.md, *Librarian*.
CREATE TABLE feature_upkeep (
    project    TEXT NOT NULL,
    slug       TEXT NOT NULL,
    kind       TEXT NOT NULL,
    evidence   TEXT NOT NULL DEFAULT '',
    checked_at TEXT NOT NULL,
    kept_at    TEXT,
    PRIMARY KEY (project, slug)
) STRICT;

CREATE TABLE feature_doc_gone (
    project    TEXT NOT NULL,
    target     TEXT NOT NULL,
    checked_at TEXT NOT NULL,
    PRIMARY KEY (project, target)
) STRICT;
