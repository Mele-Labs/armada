-- The owner's own layout choices, as the text of a layout.json: one row or none, in a table of its
-- own for theme_preference's reason. No row is no choices made.

CREATE TABLE layout_preference (
    id    INTEGER PRIMARY KEY CHECK (id = 1),
    value TEXT NOT NULL CHECK (length(value) BETWEEN 1 AND 4096)
) STRICT;
