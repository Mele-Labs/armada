// A Job near the end of its window asks to carry on in a new one, over four Jobs on the Cockpit. The walk `jobHandoff` plays it. Built in `../job-handoff.ts`.

import { jobHandoff } from "../job-handoff";
import type { Scenario } from "../moment";

export const s295JobHandoff: Scenario = jobHandoff("job-handoff", "A Job asks to carry on in a new one, and he says yes");
