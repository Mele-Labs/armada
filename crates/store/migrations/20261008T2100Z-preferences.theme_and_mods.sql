-- The theme Bridge draws with, and this machine's switch for each mod: tables of
-- their own, one row or none and a row per mod. `preferences` names its set in a
-- CHECK, which SQLite cannot widen without a rebuild, and a rebuild is breaking.

CREATE TABLE theme_preference (
    id    INTEGER PRIMARY KEY CHECK (id = 1),
    value TEXT NOT NULL CHECK (length(value) BETWEEN 1 AND 64)
) STRICT;

CREATE TABLE mod_switches (
    name    TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1))
) STRICT;
