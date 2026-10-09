// A Job mid-Implement that is open, beside Job 2 with a fix held on an alert. The open Job's lead
// lists that alert and opens Job 2 at the Trigger; the open Job takes added steps whose switches
// change until they fire, and a Drone step kept for every Job takes the next free name.

import { featureRunning, job2FixAlert } from "@armada/jobs/fake";

import { holding } from "../holding";
import { manifesting } from "../manifest-fake";
import type { Scenario } from "../moment";

const open = featureRunning();
const fixed = job2FixAlert();
const held = holding("real/job-2-edits-and-alerts", "A running Job open, and Job 2 with a held fix on an alert", [open, fixed], { opens: open.job.id });

export const s026EditsAndAlerts: Scenario = {
  ...held,
  state: { ...held.state, journalled: { ...held.state.journalled, ...fixed.journalled } },
  behaves: (fleet) => ({ readManifestFile: manifesting().behaves?.(fleet).readManifestFile! }),
};
