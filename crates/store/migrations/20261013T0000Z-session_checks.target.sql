-- What a Session's Check run was narrowed to: the test filter it was given, or `--changed` and the
-- packages the change reached. Null where it ran whole.
ALTER TABLE session_check_runs ADD COLUMN target TEXT;
