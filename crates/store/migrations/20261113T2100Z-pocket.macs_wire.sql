-- What another Mac's build speaks and when it was built, as its hello last said, so this Mac can
-- tell a link whose builds cannot talk (frozen until one updates) and which of the two is older.
-- `source` is `main` or `preview`. Additive. docs/concepts/pocket.md, *Builds that differ*.
ALTER TABLE pocket_macs ADD COLUMN wire TEXT;
ALTER TABLE pocket_macs ADD COLUMN built_at TEXT;
ALTER TABLE pocket_macs ADD COLUMN source TEXT;
