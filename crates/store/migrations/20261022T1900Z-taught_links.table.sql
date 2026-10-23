-- The link rules a person taught Fleet: words matching `pattern` become a link to `address`, in
-- every Session. `pattern` is a regular expression in the subset Bridge and Fleet both run;
-- `address` has `{match}` where the matched words go; `example` is the words it was taught from.
-- `repository` is the repository's key, or NULL for every repository. Time is milliseconds since
-- the epoch. No foreign key. Additive.
CREATE TABLE taught_links (
    id         INTEGER PRIMARY KEY,
    pattern    TEXT NOT NULL,
    address    TEXT NOT NULL,
    example    TEXT NOT NULL,
    repository TEXT,
    made_at    INTEGER NOT NULL
) STRICT;
CREATE INDEX taught_links_by_repository ON taught_links (repository);
