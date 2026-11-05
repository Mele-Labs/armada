-- When the item was handed to the Operator, as against `at`, when its ruling came back. An Autopilot
-- run offers again what was handed before it began, which a call still out at its start would
-- otherwise hide behind its own later row. Rows kept before this column take `at`.
ALTER TABLE operator_rulings ADD COLUMN handed TEXT;
UPDATE operator_rulings SET handed = at;
