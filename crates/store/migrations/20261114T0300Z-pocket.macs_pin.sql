-- The SHA-256 of the other Mac's Wi-Fi door certificate, as hex, kept at pairing. A call to it on
-- the same network is refused unless its certificate matches. Additive.
-- docs/concepts/pocket.md, *Macs nearby*.
ALTER TABLE pocket_macs ADD COLUMN pin TEXT;
