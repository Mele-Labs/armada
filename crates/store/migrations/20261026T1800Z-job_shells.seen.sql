-- The shell each Job's agent last typed into or read, by its run id, so that after a Fleet restart
-- ends that shell the agent is not let type into its replacement before reading it. One row a Job.
-- No foreign key, as `session_linked_told` has none. Additive.
-- crates/ipc/operations/type_in_terminal.toml.
CREATE TABLE job_shell_seen (
    session_id TEXT NOT NULL PRIMARY KEY,
    run_id     TEXT NOT NULL,
    seen_at    TEXT NOT NULL
) STRICT;
