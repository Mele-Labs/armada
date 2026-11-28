-- A note on a screen Fleet sent to every live Job on the screen's features, and the Job that took it. Additive.
-- docs/concepts/feature-catalog.md, *A note on a screen*.
--
-- One row a note. `offered` and `paths` are JSON arrays of strings, `offered` in the order the Jobs were sent it.
-- `claimed_by` is empty until a Job claims the note or Fleet gives it one; `given` is 1 where Fleet gave it.
-- `project`, `words` and `note` (the note as sent, JSON) start a Job for it where every Job it went to has ended.
CREATE TABLE note_claims (
    note_id     TEXT PRIMARY KEY,
    manifest_id TEXT NOT NULL,
    paths       TEXT NOT NULL,
    offered     TEXT NOT NULL,
    sent_at     TEXT NOT NULL,
    claimed_by  TEXT NOT NULL DEFAULT '',
    claimed_at  TEXT NOT NULL DEFAULT '',
    given       INTEGER NOT NULL DEFAULT 0,
    project     TEXT NOT NULL DEFAULT '',
    words       TEXT NOT NULL DEFAULT '',
    note        TEXT NOT NULL DEFAULT ''
) STRICT;
