-- A question a Job's Commander has put to the owner, which outlives the turn that raised it. It is
-- modelled on a Drone's question and never on a held tool call: it waits until he answers, however
-- long, and Fleet restarting does not lose it. `options` is the labels in the order asked, as a JSON
-- array; an option's id is its one-based place. `chose` is the id he picked, and a row with an
-- `answered_at` is history. The index is the rule that a Job has one card out at a time.
CREATE TABLE commander_cards (
    card_id     TEXT PRIMARY KEY,
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    raised_by   TEXT NOT NULL,
    raised_at   TEXT NOT NULL,
    text        TEXT NOT NULL,
    reach       TEXT NOT NULL,
    question    TEXT NOT NULL,
    options     TEXT NOT NULL,
    chose       TEXT,
    answered_at TEXT
) STRICT;
CREATE UNIQUE INDEX commander_cards_one_out ON commander_cards (job_id) WHERE answered_at IS NULL;
