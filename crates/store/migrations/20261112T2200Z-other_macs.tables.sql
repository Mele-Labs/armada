-- What his other Macs share with this one: their Job rows and bug reports, each whole as the wire carried it and
-- opaque here, kept so a Mac that is asleep still has its rows drawn after Fleet restarts. Additive.
-- docs/practices/store-migrations.md.
CREATE TABLE other_mac_sessions (
    mac    TEXT NOT NULL,
    id     TEXT NOT NULL,
    record TEXT NOT NULL,
    PRIMARY KEY (mac, id)
) STRICT;

CREATE TABLE other_mac_bug_reports (
    mac    TEXT NOT NULL,
    id     TEXT NOT NULL,
    report TEXT NOT NULL,
    PRIMARY KEY (mac, id)
) STRICT;
