-- A Session's retros: what got in the agent's way since the last one, and the
-- Session's restarts, which nothing else kept. docs/concepts/retro.md.
--
-- A Session has several retros; each covers from the end of the one before
-- (`covers_from`, null for the first) to `covers_to`. Only a written retro is
-- kept: a failed one moves nothing, so the next try covers the same stretch.

CREATE TABLE session_retros (
    retro_id    INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT NOT NULL,
    covers_from TEXT,
    covers_to   TEXT NOT NULL,
    model       TEXT NOT NULL,
    -- What the agent said got in its way when Fleet asked it. Null where it
    -- was not asked or had nothing to say.
    note        TEXT,
    at          TEXT NOT NULL
) STRICT;

CREATE INDEX session_retros_by_session ON session_retros (session_id, retro_id);

CREATE TABLE session_retro_items (
    retro_id       INTEGER NOT NULL REFERENCES session_retros(retro_id),
    ordinal        INTEGER NOT NULL CHECK (ordinal >= 0),
    whose          TEXT NOT NULL CHECK (whose IN ('drone', 'agent', 'owner', 'fleet')),
    said           TEXT NOT NULL CHECK (trim(said) <> ''),
    evidence       TEXT NOT NULL,
    lands_in       TEXT CHECK (lands_in IS NULL OR lands_in IN ('armada', 'kit', 'manifest')),
    title          TEXT CHECK (title IS NULL OR trim(title) <> ''),
    what           TEXT CHECK (what IS NULL OR trim(what) <> ''),
    fix            TEXT CHECK (fix IS NULL OR trim(fix) <> ''),
    state          TEXT NOT NULL DEFAULT 'open'
        CHECK (state IN ('open', 'agreed', 'accepted', 'discarded')),
    job_proposed   TEXT,
    change_kind    TEXT CHECK (change_kind IS NULL OR change_kind IN ('allow_command')),
    change_command TEXT CHECK (change_command IS NULL OR trim(change_command) <> ''),
    applied        INTEGER NOT NULL DEFAULT 0 CHECK (applied IN (0, 1)),
    PRIMARY KEY (retro_id, ordinal)
) STRICT;

-- A hosted Session's process coming back: resumed, reattached after Fleet
-- restarted, or gone while a turn ran.
CREATE TABLE session_restarts (
    session_id TEXT NOT NULL,
    at         TEXT NOT NULL,
    kind       TEXT NOT NULL,
    said       TEXT
) STRICT;

CREATE INDEX session_restarts_by_session ON session_restarts (session_id, at);

-- A Session that ended and was owed a retro Fleet did not write: nothing new
-- in it, or the call failed. Not tried again. **Every Session already ended is
-- marked**, so the first turn after this migration spends no model call on the
-- ones the file has finished.
CREATE TABLE session_retro_skips (
    session_id TEXT PRIMARY KEY,
    at         TEXT NOT NULL
) STRICT;

INSERT INTO session_retro_skips (session_id, at)
SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now') FROM sessions WHERE state = 'ended';
