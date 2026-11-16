-- What the read-only copy looked at while it answered, one `<Tool> <target>` a line, in the order it read. Empty for
-- an aside that read nothing, and for every aside kept before this column. Additive.
ALTER TABLE job_asides ADD COLUMN reads TEXT NOT NULL DEFAULT '';
