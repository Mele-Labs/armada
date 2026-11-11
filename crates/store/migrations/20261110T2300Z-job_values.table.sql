-- The values a Job froze when it started: one row for each key of its project's and its repository's
-- resolved values, `layer` the layer the value came from (`default`, `machine`, `kit`, `repository`,
-- `project` or `job`). `key` is `pr_mode`, `auto_merge`, `review_model`, `layout`, `theme`, `density`
-- or `autonomy.<kind of act>`, and `value` is NULL for a key no layer set. A Job already running keeps
-- its rows when a later edit moves a layer. Additive.
CREATE TABLE job_values (
    session_id TEXT NOT NULL,
    key        TEXT NOT NULL,
    value      TEXT,
    layer      TEXT NOT NULL,
    PRIMARY KEY (session_id, key)
) STRICT;
