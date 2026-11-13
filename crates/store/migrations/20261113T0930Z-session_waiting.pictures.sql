-- The pictures an agent's item offers its options as, kept with the options they belong to: the JSON of an
-- array as long as `options`, each entry null or one picture, and empty where no option has one. Kept on the
-- open item and on the closed one, so a restart and a reopen keep them. Additive: a row kept before this
-- reads as no pictures. docs/practices/protocol/how-he-works-with-jobs-and-a-question-offered-as-pictures.md.
ALTER TABLE session_waiting ADD COLUMN pictures TEXT NOT NULL DEFAULT '';
ALTER TABLE session_waiting_dismissed ADD COLUMN pictures TEXT NOT NULL DEFAULT '';
