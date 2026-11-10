-- Finds the suggestions kept before for the same kind of arrival and way in a repository, settled or
-- not, so a repeat of one is not raised again. Additive.
CREATE INDEX operator_suggestions_by_kind ON operator_suggestions (manifest_id, arrived, way);
