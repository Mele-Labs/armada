// The Dashboard with nothing waiting on the owner: no Session and no merge line asking anything.
// Command Central is the quick dispatch box.

import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const base = holding("dashboard-quick-dispatch", "The Dashboard with nothing needing the owner: the quick dispatch box");

// One repository picked, so the composer opens straight on its field rather than asking which.
export const s205DashboardQuickDispatch: Scenario = { ...base, state: { ...base.state, repository: repository().root } };
