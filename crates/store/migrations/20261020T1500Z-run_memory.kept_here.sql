-- The repositories a person marked Keep on this Mac (docs/concepts/kit.md, *Kept on this Mac*):
-- a row is the mark. While it stands, the repository's runs and every piece of the person's own
-- for it stay in Kit's folder on this Mac. Keyed by the repository's root. Additive.
CREATE TABLE run_memory_kept_here (
    repository TEXT NOT NULL PRIMARY KEY
) STRICT;
