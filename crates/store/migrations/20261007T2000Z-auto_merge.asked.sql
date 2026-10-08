-- A person pressed Merge on a Job's pull request while its checks ran, and the
-- forge was asked to merge it when they pass. Null is never asked. Set once, by
-- the press; the sweep reads it to complete a Job the forge merged.
ALTER TABLE jobs ADD COLUMN delivery_auto_merge_asked TEXT;
