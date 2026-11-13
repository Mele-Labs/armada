-- When each field of a task was last edited, in the editing Mac's milliseconds, so an edit another Mac made while this
-- one slept is applied only to a field it is newer than: the title, the notes, the state, and its place (project,
-- position and parent). Null is never edited since the task was written, older than any edit. Additive.
ALTER TABLE project_rows ADD COLUMN title_at INTEGER;
ALTER TABLE project_rows ADD COLUMN notes_at INTEGER;
ALTER TABLE project_rows ADD COLUMN state_at INTEGER;
ALTER TABLE project_rows ADD COLUMN place_at INTEGER;
