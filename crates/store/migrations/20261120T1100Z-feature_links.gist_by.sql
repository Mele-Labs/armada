-- Who wrote a read's gist: empty where it is the result's first sentence, `job` where the Job that read it wrote a
-- better one at its end. A Job's gist is not overwritten by a later first sentence. Kept on the Job's read too, so a link
-- made after it carries the same gist. Additive.
ALTER TABLE feature_links ADD COLUMN gist_by TEXT NOT NULL DEFAULT '';
ALTER TABLE feature_job_reads ADD COLUMN gist_by TEXT NOT NULL DEFAULT '';
