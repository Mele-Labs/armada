-- Every build Fleet has run: a row for each commit Fleet saw `restart-commit` name, in the order it
-- began. `started_at` is when the restart wrote the commit, and a build runs until the next one
-- begins. `source` is `main` or `preview`; `main_commit` the main commit the build stands on (its
-- own, for a main build); `branches` the names of the branches a preview holds, as JSON. **Unknown
-- stays NULL**, and `looked` says git was asked and had the commit. Never pruned.
-- The first rows are the builds Jobs were told of (`build_notices`), when the first was told: that
-- history is partial, as a build told to no Job left no row. Additive.
CREATE TABLE fleet_builds (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    commit_id   TEXT NOT NULL,
    started_at  TEXT NOT NULL,
    source      TEXT CHECK (source IN ('main', 'preview')),
    main_commit TEXT,
    branches    TEXT NOT NULL DEFAULT '[]',
    looked      INTEGER NOT NULL DEFAULT 0 CHECK (looked IN (0, 1))
) STRICT;
CREATE UNIQUE INDEX fleet_builds_by_start ON fleet_builds (started_at, commit_id);

INSERT INTO fleet_builds (commit_id, started_at)
    SELECT commit_id, MIN(noted_at) FROM build_notices GROUP BY commit_id ORDER BY MIN(noted_at);
