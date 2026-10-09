-- The phones the owner paired with the Phone Gateway, and the Web Push
-- subscription each one holds. `public_key` is the SPKI DER the phone exported;
-- times are Unix seconds. Additive. docs/practices/store-migrations.md.
CREATE TABLE pocket_devices (
    device_id    TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    public_key   BLOB NOT NULL,
    created_at   INTEGER NOT NULL,
    last_seen_at INTEGER
) STRICT;

CREATE TABLE pocket_push_subscriptions (
    device_id TEXT NOT NULL,
    endpoint  TEXT NOT NULL,
    p256dh    TEXT NOT NULL,
    auth      TEXT NOT NULL,
    PRIMARY KEY (device_id, endpoint)
) STRICT;
