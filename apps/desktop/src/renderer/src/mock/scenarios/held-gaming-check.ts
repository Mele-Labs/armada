// A Job the gaming check holds with its Drone still on the step: a weakened
// assertion and three refused commands, answered under the lead (#1672).
// The Job `held/gaming-check` opens on is served with the Drone still there.

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { heldByTheGamingCheck } from "@armada/jobs/fake";

const HELD_BY_A_FLAG = heldByTheGamingCheck(["override_verdict", "redirect_drone", "redispatch_job"]);

export const s080HeldByTheGamingCheck: Scenario = holding("held/gaming-check", HELD_BY_A_FLAG.name, [HELD_BY_A_FLAG], { opens: HELD_BY_A_FLAG.job.id });
