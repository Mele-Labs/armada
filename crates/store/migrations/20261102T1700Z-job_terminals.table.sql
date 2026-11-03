-- A Job's terminals, one row each, kept after the shell ends so that `t1`, `t2` are never given
-- twice in a Job: the next is the highest number plus one. `name` is what the person renamed it
-- to, blank until then. `seen_run_id` and `seen_at` are the shell its agent last typed into or
-- read, and when; they replace `job_shell_seen`, which is left in place and no longer read, since
-- dropping it would be breaking and a preview of unlanded work could no longer be switched onto.
-- No foreign key, as `session_linked_told` has none. Additive.
-- crates/ipc/operations/type_in_terminal.toml.
CREATE TABLE job_terminals (
    session_id  TEXT    NOT NULL,
    number      INTEGER NOT NULL,
    name        TEXT    NOT NULL DEFAULT '',
    seen_run_id TEXT,
    seen_at     TEXT,
    PRIMARY KEY (session_id, number)
) STRICT;
