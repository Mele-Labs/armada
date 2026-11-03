// The tab of one walk. Its state is kept in `store.ts`, so leaving the Job and coming back, or moving the tab to another pane, finds the page, take and step as they were. Approving
// settles the walk, and `sync.tsx` takes the tab of a settled walk away.

import { useEffect } from "react";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Tab } from "../job-panes/tree";
import { markSeen } from "./store";
import { WalkPane } from "./WalkPane";

export function WalkBody({ session, tab }: { session: Session; tab: Tab }) {
  const jobId = session.id;
  // Drawn in front of the person: the tab stops pulsing.
  useEffect(() => {
    if (tab.page !== undefined) markSeen(jobId, tab.page);
  }, [
    jobId,
    tab.page,
  ]);
  return tab.page === undefined ? null : <WalkPane jobId={jobId} walkId={tab.page} />;
}
