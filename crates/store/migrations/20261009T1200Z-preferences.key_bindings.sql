-- The owner's own key bindings, as the text Settings → Keyboard wrote: one row or none, in a table of
-- its own for theme_preference's reason. No row is the registry's bindings, unchanged.

CREATE TABLE key_bindings_preference (
    id    INTEGER PRIMARY KEY CHECK (id = 1),
    value TEXT NOT NULL CHECK (length(value) BETWEEN 1 AND 16384)
) STRICT;
