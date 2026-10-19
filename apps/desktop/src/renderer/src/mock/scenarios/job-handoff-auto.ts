// A Job under the Context rule's line, and the Settings row that says what happens when it passes: set to carry on automatically, the Job hands off with no card. The walk `jobHandoffAuto` plays it. Built in `../job-handoff.ts`.

import { jobHandoff } from "../job-handoff";
import type { Scenario } from "../moment";

export const s298JobHandoffAuto: Scenario = jobHandoff("job-handoff-auto", "Context set to carry on automatically, and a Job passes the line", false, true);
