-- A terminal watch: the text or pattern it waits for in its Job's terminal, in place of a command,
-- and whether it is a regular expression. Absent for a watch that runs a command. Additive.
ALTER TABLE watches ADD COLUMN in_terminal TEXT;
ALTER TABLE watches ADD COLUMN regex INTEGER NOT NULL DEFAULT 0;
