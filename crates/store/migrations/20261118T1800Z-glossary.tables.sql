-- A project's glossary: the words its work uses and the words that mean the same and should not be. One row a term,
-- unique per project without regard to case (`term_key` is the lower-cased term). `not_words` is a JSON list. `job`
-- is the Job that wrote it, or `seed` where Fleet's seed did. `glossary_seeded` is one row a project once its seed has
-- been kept, so a seed is never kept twice whatever became of its terms. Additive.
CREATE TABLE glossary_terms (
    project   TEXT NOT NULL,
    term_key  TEXT NOT NULL,
    term      TEXT NOT NULL,
    meaning   TEXT NOT NULL,
    not_words TEXT NOT NULL DEFAULT '[]',
    job       TEXT NOT NULL DEFAULT '',
    at        TEXT NOT NULL,
    PRIMARY KEY (project, term_key)
) STRICT;

CREATE TABLE glossary_seeded (
    project TEXT NOT NULL PRIMARY KEY,
    at      TEXT NOT NULL
) STRICT;
