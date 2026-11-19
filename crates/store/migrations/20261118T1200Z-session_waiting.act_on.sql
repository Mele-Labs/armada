-- The Mac a `run` item's command is meant for, kept with the item: a Mac's name, and empty where the item names
-- none. Kept on the open item and on the closed one, so a restart and a reopen keep it. Additive: a row kept
-- before this reads as naming none. docs/concepts/session/waiting-on-the-person.md.
ALTER TABLE session_waiting ADD COLUMN act_on TEXT NOT NULL DEFAULT '';
ALTER TABLE session_waiting_dismissed ADD COLUMN act_on TEXT NOT NULL DEFAULT '';
