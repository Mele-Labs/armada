-- breaking
-- A project is his own and spans repositories. Every row now names its repository (`manifest_id`),
-- taken from the project it stood in, and a project loses its own. A task in no project has the
-- empty `project_id`, with a position among the loose rows. The rows of a repository's `own:<id>`
-- project were that repository's loose tasks: they become loose rows, and the `own:` projects go.
-- Projects keep their names and order, the repositories' orders laid in the order they were made.
-- The places are ranked into scratch tables first: SQLite lets an UPDATE read what it has already changed.
ALTER TABLE project_rows ADD COLUMN manifest_id TEXT NOT NULL DEFAULT '';
UPDATE project_rows SET manifest_id = COALESCE((SELECT manifest_id FROM projects WHERE projects.id = project_rows.project_id), '');
UPDATE project_rows SET project_id = '' WHERE project_id IN (SELECT id FROM projects WHERE id LIKE 'own:%');
DELETE FROM projects WHERE id LIKE 'own:%';
CREATE TEMP TABLE loose_places AS
    SELECT id, (
        SELECT COUNT(*) FROM project_rows other
        WHERE other.project_id = '' AND other.parent = project_rows.parent
          AND (other.manifest_id, other.position, other.id) < (project_rows.manifest_id, project_rows.position, project_rows.id)
    ) AS place FROM project_rows WHERE project_id = '';
UPDATE project_rows SET position = (SELECT place FROM loose_places WHERE loose_places.id = project_rows.id) WHERE project_id = '';
DROP TABLE loose_places;
CREATE TEMP TABLE project_places AS
    SELECT id, (SELECT COUNT(*) FROM projects other WHERE (other.created, other.id) < (projects.created, projects.id)) AS place FROM projects;
UPDATE projects SET position = (SELECT place FROM project_places WHERE project_places.id = projects.id);
DROP TABLE project_places;
DROP INDEX projects_by_repository;
ALTER TABLE projects DROP COLUMN manifest_id;
CREATE INDEX projects_by_position ON projects (position);
