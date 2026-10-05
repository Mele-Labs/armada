// A running Job and every Drone it has had, as `list_job_drones` serves them:
// one killed, two finished with their cost, and the one running now.

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { everyDroneHad } from "@armada/screens/src/fixtures/build/drones-had";

export const s050EveryDroneHad: Scenario = holding("drones/every-drone-had", everyDroneHad().name, [everyDroneHad()], { opens: everyDroneHad().job.id });
