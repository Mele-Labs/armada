-- What a run's model-written part kept and spent, beside the run. `paragraph` is the one paragraph the since_you_looked
-- part wrote for the top of Since you looked, '' where it wrote none. `night` is the machine's local date the run fell
-- on, as 2026-11-18, and `model_calls` the calls to a model the run made, so a night's share is a sum over one project
-- and one night. `skipped` names the parts a run left out because the share was spent, joined by commas. Additive: a
-- row kept before this reads as no paragraph, no calls and nothing skipped. docs/concepts/feature-catalog.md, *Librarian*.
ALTER TABLE librarian_runs ADD COLUMN paragraph   TEXT NOT NULL DEFAULT '';
ALTER TABLE librarian_runs ADD COLUMN night       TEXT NOT NULL DEFAULT '';
ALTER TABLE librarian_runs ADD COLUMN model_calls INTEGER NOT NULL DEFAULT 0;
ALTER TABLE librarian_runs ADD COLUMN skipped     TEXT NOT NULL DEFAULT '';
