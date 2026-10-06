// What every scenario is made of: the state a connected Fleet publishes, and a
// scenario built from rows a test picks. Apart from `scenario.ts`, so a
// scenario's own Fleet (`setup-fleet.ts`) can build on it and still be listed
// there without the two importing each other. The generic parts live in
// `@armada/bridge-api`; this file fixes them to desktop's `BridgeState` and `BridgeApi`.

import { connected as connectedOver, onBoard as onBoardOver, unanswered } from "@armada/bridge-api";
import type { FleetHandle as FleetHandleOf, Scenario as ScenarioOf } from "@armada/bridge-api";
import type { ArcDraft } from "@armada/jobs/fixtures/build/arc";
import type { JobSummary, RepositorySummary, WorkflowSummary } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
import { NOTHING_YET } from "../../../shared/bridge";
import type { BridgeState } from "../../../shared/bridge";

export { unanswered };

/** One moment: what is published before anything is opened, and the reads behind each Job. */
export type Scenario = ScenarioOf<BridgeState, BridgeApi, ArcDraft>;

/** What a scenario's `behaves` reaches: the state as published, and the one way to change it. */
export type FleetHandle = FleetHandleOf<BridgeState>;

/** The state a connected Fleet publishes, holding these Jobs. */
export function connected(
  jobs: JobSummary[],
  workflows: WorkflowSummary[],
  repositories: RepositorySummary[],
): BridgeState {
  return connectedOver(NOTHING_YET, jobs, workflows, repositories);
}

/** A connected Fleet holding exactly these rows, for a test that needs a Board no named scenario draws. */
export function onBoard(
  jobs: JobSummary[],
  options: { workflows?: WorkflowSummary[]; repositories?: RepositorySummary[]; picked?: string | null } = {},
): Scenario {
  return onBoardOver<BridgeState, BridgeApi>(NOTHING_YET, jobs, options);
}
