-- The one queue for what waits on a Mac that cannot be reached now holds acts on its Jobs beside the edits of its
-- project copies, in one order: `kind` is `project` (every row before this) or `job`, `id` is the sender's id for a
-- job act, so a send that went twice is one send, and `refused` is what that Mac said when it refused the act on its
-- turn, which keeps the act from being sent again until the person takes it back. And the acts this Mac ran for
-- another, keyed by that Mac and the id it sent, with the answer, so a repeat is answered and not run again; `via`
-- is the door the act ran as and `what` its kind. Additive. docs/practices/store-migrations.md.
ALTER TABLE other_mac_project_acts ADD COLUMN kind TEXT NOT NULL DEFAULT 'project';
ALTER TABLE other_mac_project_acts ADD COLUMN id TEXT NOT NULL DEFAULT '';
ALTER TABLE other_mac_project_acts ADD COLUMN refused TEXT;

CREATE TABLE other_mac_acts_run (
    mac    TEXT NOT NULL,
    id     TEXT NOT NULL,
    at     INTEGER NOT NULL,
    via    TEXT NOT NULL,
    what   TEXT NOT NULL,
    status INTEGER NOT NULL,
    body   TEXT NOT NULL,
    PRIMARY KEY (mac, id)
) STRICT;
