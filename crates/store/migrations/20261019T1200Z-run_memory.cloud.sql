-- Where `cloud` keeps a repository's runs (docs/concepts/kit.md, *What a repository runs*): the
-- word `store` for Armada's central store, else a bucket address such as `s3://bucket/prefix`.
-- Empty for every other place. Additive.
ALTER TABLE run_memory_places ADD COLUMN cloud TEXT;
