-- breaking
-- The Helm Job is gone and a Job started on your words takes the one flag it held. The flag is
-- kept under its new name and cleared on every row, so the old Helm threads read as plain ended
-- Jobs and none is deleted. A Helm Job still live and holding no slot is ended here; one holding
-- a slot is left to Fleet, whose release of a slot is more than a row.
ALTER TABLE hosted_sessions RENAME COLUMN helm TO on_your_words;

UPDATE sessions
   SET state = 'ended',
       ended_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
       end_reason = 'closed'
 WHERE state = 'live'
   AND id IN (SELECT session_id FROM hosted_sessions WHERE on_your_words = 1 AND lease_slot IS NULL);

UPDATE hosted_sessions SET on_your_words = 0;

-- A plan change by a Job the person was talking to.
ALTER TABLE job_work_plan_changes RENAME COLUMN by_helm TO by_job;

-- What the Helm Job resumed and what it ran on, per repository.
DROP TABLE helm_tuning;
DROP TABLE helm_sessions;

-- Who put something on a Studio is `job` where it was `helm`. The sets in the CHECKs cannot be
-- widened in place, so the three tables are rebuilt the way V91 rebuilt two of them. A node may
-- sit inside a frame that is copied after it, so the keys are checked at the end.
PRAGMA defer_foreign_keys = ON;

CREATE TABLE studios_parked AS SELECT * FROM studios;
CREATE TABLE studio_nodes_parked AS SELECT * FROM studio_nodes;
CREATE TABLE studio_edges_parked AS SELECT * FROM studio_edges;

DROP TABLE studio_edges;
DROP TABLE studio_nodes;
DROP TABLE studios;

CREATE TABLE studios (
    id          TEXT PRIMARY KEY,
    manifest_id TEXT NOT NULL,
    name        TEXT,
    created_at  TEXT NOT NULL,
    touched_at  TEXT NOT NULL,
    named_by    TEXT CHECK (named_by IS NULL OR named_by IN ('person', 'job'))
) STRICT;

CREATE INDEX studios_by_manifest ON studios (manifest_id, touched_at);

CREATE TABLE studio_nodes (
    id         TEXT PRIMARY KEY,
    studio_id  TEXT NOT NULL REFERENCES studios (id) ON DELETE CASCADE,
    kind       TEXT NOT NULL CHECK (kind IN ('run', 'note', 'cluster', 'zone', 'finding',
               'contradiction', 'sketch', 'link', 'file', 'picture', 'issue', 'pull_request',
               'epic', 'deferral', 'outline', 'issue_draft', 'job')),
    state      TEXT CHECK (state IS NULL OR state IN ('proposed', 'gathering', 'reported',
               'issue_draft', 'deferral', 'not_a_problem', 'resolved_here', 'open', 'answered',
               'draft')),
    content    TEXT NOT NULL,
    within     TEXT,
    x          INTEGER NOT NULL,
    y          INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    added_by   TEXT CHECK (added_by IS NULL OR added_by IN ('person', 'job')),
    UNIQUE (studio_id, id),
    CHECK (within IS NULL OR within <> id),
    FOREIGN KEY (studio_id, within) REFERENCES studio_nodes (studio_id, id)
) STRICT;

CREATE TABLE studio_edges (
    id         TEXT PRIMARY KEY,
    studio_id  TEXT NOT NULL REFERENCES studios (id) ON DELETE CASCADE,
    from_node  TEXT NOT NULL,
    to_node    TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('produced', 'same_as', 'blocks', 'answers')),
    standing   TEXT NOT NULL CHECK (standing IN ('proposed', 'accepted')),
    created_at TEXT NOT NULL,
    added_by   TEXT CHECK (added_by IS NULL OR added_by IN ('person', 'job')),
    FOREIGN KEY (studio_id, from_node) REFERENCES studio_nodes (studio_id, id) ON DELETE CASCADE,
    FOREIGN KEY (studio_id, to_node) REFERENCES studio_nodes (studio_id, id) ON DELETE CASCADE,
    UNIQUE (from_node, to_node, kind),
    CHECK (from_node <> to_node),
    CHECK (kind <> 'produced' OR standing = 'accepted')
) STRICT;

INSERT INTO studios (id, manifest_id, name, created_at, touched_at, named_by)
SELECT id, manifest_id, name, created_at, touched_at,
       CASE named_by WHEN 'helm' THEN 'job' ELSE named_by END
FROM studios_parked;

INSERT INTO studio_nodes (id, studio_id, kind, state, content, within, x, y, created_at, added_by)
SELECT id, studio_id, kind, state, content, within, x, y, created_at,
       CASE added_by WHEN 'helm' THEN 'job' ELSE added_by END
FROM studio_nodes_parked;

INSERT INTO studio_edges (id, studio_id, from_node, to_node, kind, standing, created_at, added_by)
SELECT id, studio_id, from_node, to_node, kind, standing, created_at,
       CASE added_by WHEN 'helm' THEN 'job' ELSE added_by END
FROM studio_edges_parked;

DROP TABLE studios_parked;
DROP TABLE studio_nodes_parked;
DROP TABLE studio_edges_parked;
