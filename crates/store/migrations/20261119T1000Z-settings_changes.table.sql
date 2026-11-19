-- Each change to a setting the Usage page offers, kept when Fleet puts it in force, so what the change did
-- to the waste it cuts can be read after (docs/concepts/session/where-the-tokens-went.md, *Settings that
-- change it*). `key` is the settings.json path, `at` the stamp it came into force, and `from_value` and
-- `to_value` what the key held either side, as text: a number as its digits and a word as itself.
-- Only the keys Fleet names are kept. Additive.
CREATE TABLE settings_changes (
    key        TEXT NOT NULL,
    at         TEXT NOT NULL,
    from_value TEXT NOT NULL,
    to_value   TEXT NOT NULL,
    PRIMARY KEY (key, at)
) STRICT, WITHOUT ROWID;

CREATE INDEX settings_changes_by_at ON settings_changes (at);
