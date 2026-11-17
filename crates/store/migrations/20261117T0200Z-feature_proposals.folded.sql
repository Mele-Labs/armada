-- breaking
-- A feature an act names that the project does not hold is now a pending feature in `features`, as a commit trailer's is,
-- so the Operator's one card confirms both (docs/concepts/feature-catalog.md). Nothing read `feature_proposals`, and no
-- project held features before the catalog was seeded, so it holds no row to move.
DROP TABLE feature_proposals;
