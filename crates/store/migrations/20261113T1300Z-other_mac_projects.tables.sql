-- The projects another Mac shares with this one, each whole as the wire carried it and opaque here, so a Mac that is
-- asleep still has its projects drawn after Fleet restarts; and the task acts made on such a copy while its Mac
-- could not be reached, `act` a `ProjectAct` as text and `at` when the person made it in this Mac's milliseconds,
-- kept until that Mac answers them. Additive. docs/practices/store-migrations.md.
CREATE TABLE other_mac_projects (
    mac     TEXT NOT NULL,
    id      TEXT NOT NULL,
    project TEXT NOT NULL,
    PRIMARY KEY (mac, id)
) STRICT;

CREATE TABLE other_mac_project_acts (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    mac TEXT NOT NULL,
    at  INTEGER NOT NULL,
    act TEXT NOT NULL
) STRICT;
CREATE INDEX other_mac_project_acts_by_mac ON other_mac_project_acts (mac, seq);
