// A Job at its review gate with a pull request open, by what the forge's checks on it have come to:
// passed, still running, failed. Merge's face follows them, and the press is answered at the gate.

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { gateChecksFailed, gateChecksPassed, gateChecksRunning } from "@armada/jobs/fake";

const PASSED = gateChecksPassed();
const RUNNING = gateChecksRunning();
const FAILED = gateChecksFailed();

export const s220GateChecksPassed: Scenario = holding("gate/checks-passed", PASSED.name, [PASSED], { opens: PASSED.job.id });
export const s221GateChecksRunning: Scenario = holding("gate/checks-running", RUNNING.name, [RUNNING], { opens: RUNNING.job.id });
export const s222GateChecksFailed: Scenario = holding("gate/checks-failed", FAILED.name, [FAILED], { opens: FAILED.job.id });
