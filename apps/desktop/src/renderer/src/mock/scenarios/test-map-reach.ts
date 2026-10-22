// A Session's pull request whose Files list carries the tests that reach each changed file: measured, a static graph, a model's
// reading, stale, and none. The walk `test-map-reach` plays it. Mock only: no Fleet serves the test map to Bridge yet.

import type { Scenario } from "../moment";
import { testsReaching } from "../test-map-reach";
import { s236SessionPrPanel } from "./session-pr-panel";

export const testMapReach: Scenario = {
  ...s236SessionPrPanel,
  name: "test-map-reach",
  says: "A Session's pull request, its changed files marked with the tests that reach them",
  draft: { ...s236SessionPrPanel.draft, testsReaching },
};
