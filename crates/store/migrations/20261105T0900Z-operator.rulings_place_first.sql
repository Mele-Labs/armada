-- What a ruling said beyond its way. `place` is where a start goes in Waiting to start: `ahead:<id>`,
-- `behind:<id>` or `last`, null where none was ruled. `first_session` is the Job of two kept in touch
-- that builds first, the other held until it pushes. `refused` is each part of the ruling Fleet
-- refused while the rest stood, one a line.
ALTER TABLE operator_rulings ADD COLUMN place TEXT;
ALTER TABLE operator_rulings ADD COLUMN first_session TEXT;
ALTER TABLE operator_rulings ADD COLUMN refused TEXT NOT NULL DEFAULT '';
