-- The first turn a Job on its start card will read, kept off its thread until the person presses
-- Start. `body` is Fleet's, opaque here: the row the thread will keep and the words the agent reads.
-- One per Job; taken when it begins. Additive.
CREATE TABLE held_first_turns (
    session_id TEXT NOT NULL PRIMARY KEY,
    body       TEXT NOT NULL
) STRICT;
