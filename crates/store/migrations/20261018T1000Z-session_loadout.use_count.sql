-- How many times a use was heard in its turn (docs/practices/protocol.md, *A Session's Loadout*).
-- A repeat of a kept use raises this instead of being dropped. A column with a default, so additive;
-- every use kept before it was heard once.
ALTER TABLE session_loadout_uses ADD COLUMN count INTEGER NOT NULL DEFAULT 1;
