-- The question Fleet put to a person about where a taught pattern opens in one repository, one row
-- each for good so it is asked once: `mod_name` is the Words mod, `repository` the Manifest id,
-- `session_id` the Session whose Needs you list holds it. Whether it is still open is read from the
-- mod: an address for the repository closes it. Additive.
CREATE TABLE word_asks (
    mod_name   TEXT NOT NULL,
    repository TEXT NOT NULL,
    session_id TEXT NOT NULL,
    asked_at   TEXT NOT NULL,
    PRIMARY KEY (mod_name, repository)
) STRICT;
CREATE INDEX word_asks_by_session ON word_asks (session_id);
