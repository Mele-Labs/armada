-- The VAPID key a subscription was made with, where it is another Mac's: the phone
-- holds one subscription, made with its first Mac's key, and every Mac it pairs
-- with pushes to it signed with that key. Base64url of the raw 32-byte P-256
-- scalar. Null where the subscription was made with this Mac's own key, and for
-- every row before this column. Additive.
ALTER TABLE pocket_push_subscriptions ADD COLUMN vapid_private TEXT;
