-- The tip of a repository's main that the test map was last fed up to, so a move made while Fleet
-- was down is marked stale from here on the first sight after it starts. One row a repository.
-- Additive. `docs/concepts/test-map.md`.
CREATE TABLE test_map_fed_tips (
    repository TEXT NOT NULL PRIMARY KEY,
    tip        TEXT NOT NULL
) STRICT;
