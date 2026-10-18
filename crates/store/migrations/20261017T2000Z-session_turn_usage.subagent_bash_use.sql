-- Two more facts about a turn (docs/concepts/session.md, *Where the tokens went*): whether a subagent
-- answered it, and, where its first call was a shell command, what the command was for. `bash_use` is a
-- label (read_search, scripts, git, build_test, loops_waits, github, other), never the command. Additive.
ALTER TABLE session_turn_usage ADD COLUMN subagent INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_turn_usage ADD COLUMN bash_use TEXT;

-- How far a transcript file has been read, by path, so a restart reads only what was appended.
-- `read_to` is the end of the last whole line taken in. Additive.
CREATE TABLE transcript_offsets (
    path    TEXT NOT NULL PRIMARY KEY,
    read_to INTEGER NOT NULL
) STRICT, WITHOUT ROWID;
