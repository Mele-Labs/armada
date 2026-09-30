// Which step-card design the Workflow tab draws, while the owner compares
// three (30 Sep 2026: *show me options*).
//
// **Provided by the mock alone.** This package holds no storage and reads no
// URL, so the mock reads `?cards=` and hands the answer in here; the app
// provides nothing and draws today's card. Goes when he has picked one.

import { createContext } from "react";
import type { WorkflowStepCardDesign } from "@armada/components";

export const StepCardDesign = createContext<WorkflowStepCardDesign | undefined>(undefined);
