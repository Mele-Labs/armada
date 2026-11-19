-- What a doc or ticket link says, for the page: the read's first sentence. Empty for every other kind. A link whose
-- address has gone is marked `gone` in `state`, which a later check sets and nothing here does. Additive.
ALTER TABLE feature_links ADD COLUMN gist TEXT NOT NULL DEFAULT '';
