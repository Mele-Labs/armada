-- What a dismissed item was, kept whole when the person closes it: its words, where it came from, its act,
-- the options and steps and when it began (so an agent's own item can be put back), and when it was
-- closed. A row dismissed any other way (a withdrawn walk, a merged pull request) leaves these null and is
-- not a closed item the person can see again. Additive: a row kept before this reads as not closed.
-- docs/concepts/session/waiting-on-the-person.md.
ALTER TABLE session_waiting_dismissed ADD COLUMN text TEXT;
ALTER TABLE session_waiting_dismissed ADD COLUMN source TEXT;
ALTER TABLE session_waiting_dismissed ADD COLUMN act_kind TEXT;
ALTER TABLE session_waiting_dismissed ADD COLUMN act_target TEXT;
ALTER TABLE session_waiting_dismissed ADD COLUMN options TEXT NOT NULL DEFAULT '[]';
ALTER TABLE session_waiting_dismissed ADD COLUMN steps TEXT NOT NULL DEFAULT '[]';
ALTER TABLE session_waiting_dismissed ADD COLUMN since TEXT;
ALTER TABLE session_waiting_dismissed ADD COLUMN closed_at TEXT;
