-- The session ledger: every agent session a person runs, and what each holds.
-- docs/concepts/session.md.
--
-- `sessions` is a row per session whatever started it. `ledger_attachments` is
-- what a holder has taken or done, and its holder is not a foreign key: a Job
-- and a need are holders too, later, and none of them points at `sessions`.

CREATE TABLE sessions (
    id           TEXT PRIMARY KEY,
    harness      TEXT NOT NULL,
    origin       TEXT NOT NULL,
    manifest_id  TEXT,
    cwd          TEXT NOT NULL,
    title        TEXT,
    state        TEXT NOT NULL CHECK (state IN ('live', 'ended')),
    started_at   TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    last_turn_at TEXT,
    ended_at     TEXT,
    end_reason   TEXT,
    -- A JSON object of the figures `session.measure` reported, shown and
    -- never queried.
    usage        TEXT
) STRICT;

CREATE INDEX sessions_by_manifest ON sessions (manifest_id, state);

-- `kind` is open text: a new kind is a new word here and no migration. `target`
-- is what the kind is of (a slot number, a branch, a pull request number, a
-- Job id, a path) and `manifest_id` says whose slot or branch it is, empty
-- where it belongs to no repository Fleet serves. `detail` is a JSON object of
-- text values, shown and never queried.
CREATE TABLE ledger_attachments (
    holder_kind TEXT NOT NULL,
    holder_id   TEXT NOT NULL,
    kind        TEXT NOT NULL,
    manifest_id TEXT NOT NULL DEFAULT '',
    target      TEXT NOT NULL,
    state       TEXT NOT NULL CHECK (state IN ('standing', 'spent', 'given_back')),
    detail      TEXT NOT NULL DEFAULT '{}',
    since       TEXT NOT NULL,
    changed_at  TEXT NOT NULL,
    PRIMARY KEY (holder_kind, holder_id, kind, manifest_id, target)
) STRICT;

CREATE INDEX ledger_attachments_by_target ON ledger_attachments (kind, target);
