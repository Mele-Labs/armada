-- What the owner asked of a Job, sorted from his messages by a small model and kept apart from the thread.
-- `story_messages` holds the kind each message of his was sorted as and who sorted it (`fleet` or `owner`); a
-- correction of his is the row where `sorted_by` is `owner`. `ask_id` is the ask a `new_ask` opened or a
-- `revision` or `answer` joined, and absent for `talk`. `story_asks` holds one ask: the Job it was said in
-- (`session_id`), the Job building it (`building`), its `state`, and the repository and project that Job was in.
-- `story_ask_features` carries feature slugs as plain strings, `story_ask_merges` the pull requests that built an
-- ask, `story_ask_handoffs` each hand from one Job to another. No foreign keys: an ask outlives its Job. Additive.
CREATE TABLE story_asks (
    ask_id      TEXT NOT NULL PRIMARY KEY,
    session_id  TEXT NOT NULL,
    manifest_id TEXT,
    project     TEXT,
    kind        TEXT NOT NULL,
    title       TEXT NOT NULL,
    state       TEXT NOT NULL,
    building    TEXT,
    opened_at   TEXT NOT NULL,
    state_at    TEXT NOT NULL
) STRICT;
CREATE INDEX story_asks_by_session ON story_asks (session_id, opened_at);
CREATE INDEX story_asks_by_building ON story_asks (building);
CREATE INDEX story_asks_by_project ON story_asks (project);

CREATE TABLE story_ask_features (
    ask_id TEXT NOT NULL,
    slug   TEXT NOT NULL,
    PRIMARY KEY (ask_id, slug)
) STRICT;
CREATE INDEX story_ask_features_by_slug ON story_ask_features (slug);

CREATE TABLE story_ask_merges (
    ask_id       TEXT NOT NULL,
    pull_request TEXT NOT NULL,
    merged_at    TEXT NOT NULL,
    PRIMARY KEY (ask_id, pull_request)
) STRICT;

CREATE TABLE story_ask_handoffs (
    ask_id       TEXT NOT NULL,
    from_session TEXT NOT NULL,
    to_session   TEXT NOT NULL,
    at           TEXT NOT NULL,
    PRIMARY KEY (ask_id, to_session)
) STRICT;
CREATE INDEX story_ask_handoffs_by_from ON story_ask_handoffs (from_session);
CREATE INDEX story_ask_handoffs_by_to ON story_ask_handoffs (to_session);

CREATE TABLE story_messages (
    session_id TEXT NOT NULL,
    row_id     TEXT NOT NULL,
    kind       TEXT NOT NULL,
    ask_id     TEXT,
    words      TEXT NOT NULL,
    at         TEXT NOT NULL,
    sorted_by  TEXT NOT NULL,
    sorted_at  TEXT NOT NULL,
    PRIMARY KEY (session_id, row_id)
) STRICT;
CREATE INDEX story_messages_by_ask ON story_messages (ask_id);
CREATE INDEX story_messages_by_sorter ON story_messages (sorted_by, sorted_at);
