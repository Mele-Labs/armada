-- Kit over the Mac link: what this Mac crosses, and what it last agreed with each linked Mac. Additive.
-- docs/concepts/kit.md, *Over the Mac link*.
--
-- One switch a kind, kept on this Mac alone; a kind with no row is off.
CREATE TABLE kit_link_kinds (
    kind    TEXT NOT NULL PRIMARY KEY,
    crosses INTEGER NOT NULL
) STRICT;

-- An item switched on by itself while its kind is off. `repository` is a memory note's, '' for every other kind.
CREATE TABLE kit_link_items (
    kind       TEXT NOT NULL,
    name       TEXT NOT NULL,
    repository TEXT NOT NULL,
    PRIMARY KEY (kind, name, repository)
) STRICT;

-- The last-agreed marker: the hash of an item's text both this Mac and Mac `mac` held when they last agreed on it.
CREATE TABLE kit_link_agreed (
    mac        TEXT NOT NULL,
    kind       TEXT NOT NULL,
    name       TEXT NOT NULL,
    repository TEXT NOT NULL,
    hash       TEXT NOT NULL,
    PRIMARY KEY (mac, kind, name, repository)
) STRICT;

-- Each item's text on this Mac by its hash, and when this Mac first read that version: the edit time Kit shows.
CREATE TABLE kit_link_seen (
    kind       TEXT NOT NULL,
    name       TEXT NOT NULL,
    repository TEXT NOT NULL,
    hash       TEXT NOT NULL,
    at         TEXT NOT NULL,
    PRIMARY KEY (kind, name, repository)
) STRICT;

-- What Mac `mac` last offered, whole as the wire carried it and opaque here, and when it crossed. Times are RFC3339.
CREATE TABLE kit_link_held (
    mac   TEXT NOT NULL PRIMARY KEY,
    at    TEXT NOT NULL,
    offer TEXT NOT NULL
) STRICT;
