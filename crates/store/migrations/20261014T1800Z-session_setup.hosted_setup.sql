-- A hosted session Setup started (`start_session`'s `root`), so after its Manifest is served it
-- still writes armada.yml, .gitignore and .git/info/exclude in the checkout. 0 for every other
-- session; one started before this column is marked when it binds to its Manifest. Additive.
ALTER TABLE hosted_sessions ADD COLUMN setup INTEGER NOT NULL DEFAULT 0;
