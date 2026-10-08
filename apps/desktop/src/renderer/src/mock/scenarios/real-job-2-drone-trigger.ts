// Job 2 at its review gate with a saved Drone Trigger fired when the pull request opened, its
// Drone at work. The mock Fleet moves it as Fleet does (`repair-fleet.ts`): the Drone commits, the fix
// is held for the owner, and the choice places it with no Command to run again. The repository is picked
// and the Workflows page served, so the Trigger can be made in the editor first.

import { job2DroneTrigger } from "@armada/jobs/fake";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { manifesting } from "../manifest-fake";
import { workflowing } from "../workflows-fleet";

const fixture = job2DroneTrigger();
const held = holding("real/job-2-drone-trigger", "Job 2 at its review gate, a saved Drone Trigger running", [fixture], { opens: fixture.job.id });
const editing = workflowing();

export const s025JobTwoDroneTrigger: Scenario = {
  ...held,
  state: { ...held.state, journalled: fixture.journalled, repository: held.state.holds.repositories?.[0]?.root ?? null, health: editing.state.health },
  behaves: (fleet) => ({ readManifestFile: manifesting().behaves?.(fleet).readManifestFile! }),
};
