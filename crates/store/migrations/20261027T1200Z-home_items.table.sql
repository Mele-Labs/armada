-- Each Kit item this Mac syncs with a profile's shared home on its own, whatever its kind's switch says. An item with no row is not switched on by itself. `repository` is a memory note's, and '' for every other kind. Additive.
CREATE TABLE home_items (
    login      TEXT NOT NULL,
    kind       TEXT NOT NULL,
    name       TEXT NOT NULL,
    repository TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (login, kind, name, repository)
) STRICT;
