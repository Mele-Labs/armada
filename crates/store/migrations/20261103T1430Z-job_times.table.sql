-- Where each Job's time went: a row a Job, a figure in milliseconds for each bucket, summing to
-- `ended_at` less `began_at` (docs/concepts/session.md). **Never pruned**: the timeline it is read
-- from keeps 30 days, and this is what outlasts it. Written once a Job has ended and again when
-- one reopened ends again, which is told by `ended_at` no longer matching the Job's own.
-- `tool_times_missing` and `model_starts_guessed` mark a Job whose turns lack the stamps the split
-- is read from, so its tools read as the model's and its model wait is a guess; `timeline_cut`
-- marks one that began before the timeline's oldest entry, whose waits on the owner are partial.
-- `repository` is the Manifest id, `workflow` the workflow id or empty. No foreign key. Additive.
CREATE TABLE job_times (
    session_id           TEXT NOT NULL PRIMARY KEY,
    repository           TEXT NOT NULL DEFAULT '',
    workflow             TEXT NOT NULL DEFAULT '',
    began_at             TEXT NOT NULL,
    ended_at             TEXT NOT NULL,
    model_ms             INTEGER NOT NULL,
    tools_ms             INTEGER NOT NULL,
    checks_ms            INTEGER NOT NULL,
    subagents_ms         INTEGER NOT NULL,
    slot_ms              INTEGER NOT NULL,
    you_ms               INTEGER NOT NULL,
    pull_request_ms      INTEGER NOT NULL,
    stopped_ms           INTEGER NOT NULL,
    idle_ms              INTEGER NOT NULL,
    tool_times_missing   INTEGER NOT NULL CHECK (tool_times_missing IN (0, 1)),
    model_starts_guessed INTEGER NOT NULL CHECK (model_starts_guessed IN (0, 1)),
    timeline_cut         INTEGER NOT NULL CHECK (timeline_cut IN (0, 1)),
    kept_at              TEXT NOT NULL
) STRICT, WITHOUT ROWID;
