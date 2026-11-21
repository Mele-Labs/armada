-- The one Job the person prioritised, so its builds run while other Jobs' wait on a loaded machine.
-- One row; NULL for none. Kept when that Job ends: the reader leaves an ended Job unprioritised.
CREATE TABLE session_priority (
    only_row   INTEGER PRIMARY KEY CHECK (only_row = 1),
    session_id TEXT
) STRICT;
INSERT INTO session_priority (only_row) VALUES (1);
