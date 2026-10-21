-- The commands a Job's shell reported missing on this Mac (docs/concepts/kit.md, *What a repository
-- runs*). One row per name, for the machine and not for a repository, so a Job is told what this Mac
-- lacks wherever it starts. `last_seen` is milliseconds since the epoch. Additive.
CREATE TABLE missing_commands (
    name      TEXT NOT NULL PRIMARY KEY,
    last_seen INTEGER NOT NULL
) STRICT;
