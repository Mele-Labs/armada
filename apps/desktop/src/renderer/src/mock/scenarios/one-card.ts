// The four lists that wear the one card: the Cockpit's Jobs with Time on, the Issues tab, the Reviews tab and the Logbook, on one Fleet. The walk `one-card` plays it.

import { MOCK_SETTINGS, savedInto } from "@armada/settings/fake";

import type { Scenario } from "../moment";
import { reviewSeed } from "../pr-review-pages";
import { s321IssuesTab as issues } from "./issues-tab";
import { logbookPage } from "./logbook";

export const oneCard: Scenario = {
  ...issues,
  name: "one-card",
  says: "Jobs, Issues, Pull requests and the Logbook, each list on the one card",
  draft: {
    ...issues.draft,
    prReview: reviewSeed(),
  },
  logbook: logbookPage.logbook,
  state: {
    ...issues.state,
    settings: savedInto(MOCK_SETTINGS, {
      "bridge.rowView": {
        pr: false,
        issue: false,
        now: false,
        time: true,
        conflicts: true,
        status: true,
        workflow: false,
      },
    }),
  },
};
