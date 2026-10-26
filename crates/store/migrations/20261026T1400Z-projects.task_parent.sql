-- A subtask's parent: the id of a top-level task of the same project, empty for a row that is not
-- one. Positions count among rows of one parent. One level deep, which Fleet keeps. Additive.
ALTER TABLE project_rows ADD COLUMN parent TEXT NOT NULL DEFAULT '';
