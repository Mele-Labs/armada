-- The merged pulls of one repository in time order, for the build forecast of every Job row: the
-- filter is repo and kind, the order is at then seq, so neither a walk nor a sort. Additive.
CREATE INDEX timeline_events_by_repo_kind ON timeline_events (repo, kind, at, seq);
