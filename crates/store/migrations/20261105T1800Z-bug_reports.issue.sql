-- The open issue the Operator linked a report to, kept on this Mac only: nothing is written to the
-- issue. Null while the report stands on its own.
ALTER TABLE bug_reports ADD COLUMN issue TEXT;
