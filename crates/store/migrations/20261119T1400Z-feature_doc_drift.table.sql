-- The names a document linked to a feature gives that the code had when the document was last changed and no longer has:
-- one row a (project, document), kept by the librarian's drift part and gone once a check finds nothing. `names` is a JSON
-- list, `doc_commit` the commit the document was last changed in, `checked_at` when the check ran. Additive.
-- docs/concepts/feature-catalog.md, *Librarian*.
CREATE TABLE feature_doc_drift (
    project    TEXT NOT NULL,
    target     TEXT NOT NULL,
    names      TEXT NOT NULL,
    doc_commit TEXT NOT NULL,
    checked_at TEXT NOT NULL,
    PRIMARY KEY (project, target)
) STRICT;
