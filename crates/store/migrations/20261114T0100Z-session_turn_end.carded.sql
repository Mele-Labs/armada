-- Whether the turn that ended put a question card to the person. The card was that turn's ask, so its reply,
-- the prose before the card or the words after his answer, raises no ask of its own and no notice to ask
-- again. 0 for every turn end kept before this. Additive.
-- docs/concepts/session/what-a-job-is-doing.md.
ALTER TABLE session_turn_end ADD COLUMN carded INTEGER NOT NULL DEFAULT 0;
