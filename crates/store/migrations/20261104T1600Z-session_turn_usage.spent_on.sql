-- What each turn was spent on (docs/concepts/session/where-the-tokens-went.md, *What the burn was
-- spent on*). `cache_creation_1h_tokens` is the part of `cache_creation_tokens` written to be held an
-- hour. `spent_on` is how the turn's input is shared among the parts, as JSON: `read` for its cache
-- read and `write` for its input and writes, each a part's name to its share. A turn kept before
-- this, or heard only from the mod, has neither. Additive.
ALTER TABLE session_turn_usage ADD COLUMN cache_creation_1h_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_turn_usage ADD COLUMN spent_on TEXT;

-- What a transcript's read hands to the next: the estimate of the thread's context by part, and the
-- calls still waiting on a result. Opaque to the store.
ALTER TABLE transcript_offsets ADD COLUMN carried TEXT;

-- A compaction's own request, which the transcript does not hold: what it read from the cache and
-- the summary it wrote. `id` is named after the compaction's boundary line.
CREATE TABLE session_background_usage (
    session_id        TEXT NOT NULL,
    id                TEXT NOT NULL,
    at                TEXT NOT NULL,
    model             TEXT NOT NULL,
    cache_read_tokens INTEGER NOT NULL,
    output_tokens     INTEGER NOT NULL,
    PRIMARY KEY (session_id, id)
) STRICT, WITHOUT ROWID;

CREATE INDEX session_background_usage_by_at ON session_background_usage (at);

-- Every request that burned, for the Usage page's sums: the turns, and the compactions as turns that
-- called nothing.
CREATE VIEW usage_turns AS
    SELECT session_id, at, model, input_tokens, cache_creation_tokens, cache_creation_1h_tokens,
           cache_read_tokens, output_tokens, tools, subagent, bash_use, spent_on, 0 AS background
      FROM session_turn_usage
    UNION ALL
    SELECT session_id, at, model, 0, 0, 0, cache_read_tokens, output_tokens, '[]', 0, NULL, NULL, 1
      FROM session_background_usage;

-- Every transcript is read again from the top, so the last week's turns gain their hour's writes and
-- their shares. The rows are merged, never doubled.
DELETE FROM transcript_offsets;
