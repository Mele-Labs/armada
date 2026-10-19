-- How a repository runs, as Fleet remembers it from what its Jobs did (docs/concepts/kit.md,
-- *What a repository runs*). `run_memory_places` is where a repository's facts are written out
-- (`this_mac`, `repository`, `synced`; absent is `this_mac`), `synced_folder` only for `synced`.
-- `run_facts` is the working record, one row per workspace + kind + command, merged newest run first;
-- `kept_in` is the place that has taken it and `why` what it waits on when none has.
-- `run_facts_dropped` is the deny list: a row there is never merged or written again.
-- Keyed by the repository's root. No foreign key. Additive.
CREATE TABLE run_memory_places (
    repository    TEXT NOT NULL PRIMARY KEY,
    place         TEXT NOT NULL,
    synced_folder TEXT
) STRICT;
CREATE TABLE run_facts (
    repository TEXT NOT NULL,
    workspace  TEXT NOT NULL,
    kind       TEXT NOT NULL,
    command    TEXT NOT NULL,
    port       INTEGER,
    succeeded  INTEGER NOT NULL,
    last_ran   INTEGER NOT NULL,
    source     TEXT NOT NULL,
    kept_in    TEXT,
    why        TEXT,
    PRIMARY KEY (repository, workspace, kind, command)
) STRICT;
CREATE TABLE run_facts_dropped (
    repository TEXT NOT NULL,
    workspace  TEXT NOT NULL,
    kind       TEXT NOT NULL,
    command    TEXT NOT NULL,
    dropped_at TEXT NOT NULL,
    PRIMARY KEY (repository, workspace, kind, command)
) STRICT;
