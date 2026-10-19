// The waiting item Fleet serves while it puts a pending plan to him: `plan:<version>`, source `plan`. A mock that serves a pending
// plan serves this beside it; a plan Fleet holds back, or one he has approved, has none.

import type { WaitingItem } from "@armada/protocol";

/** `version` is the plan's own; the mock plans that carry none are version 1. */
export const planCall = (version: number | undefined, since: string): WaitingItem => ({ id: `plan:${version ?? 1}`, text: "Plan to approve", since, source: "plan" });

/** What he is owed besides the plan's call: approving settles it, and so does recording the plan again before it is asked. */
export const withoutPlanCall = (waiting: readonly WaitingItem[] | undefined): WaitingItem[] => (waiting ?? []).filter((one) => one.source !== "plan");
