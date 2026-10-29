// A walk the agent shows again on the same page is one walk in Now, the newest showing. Over the Job of `job-walk-on-the-spine`, at Present, where its walk has been shown
// twice at one address under two titles.

import type { Session } from "@armada/screens/src/draft/sessions";

import { BUSY, auditLedger, shown, waitingOn, walkPage } from "../audit-job";
import type { Scenario } from "../moment";
import { s332JobWalkOnTheSpine as base } from "./job-walk-on-the-spine";

const FLOW = ["Draft", "Build", "Present", "Verify", "Pull request"];
const URL = walkPage("walk-shown-twice", "audit-files");
const FIRST = shown("b-4", "14:01:00", "Audit files in System", URL, 3);
const AGAIN = shown("b-5", "14:03:00", "Audit files in System, take 2", URL, 3);

export const s339WalkShownTwice: Scenario = {
  ...base,
  name: "walk-shown-twice",
  says: "A Job at Present whose walk was shown twice at one address: Now draws it once, as its newest showing",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, (one: Session) => {
        const ledger = auditLedger(one, { steps: FLOW, at: 2 }, [FIRST, AGAIN], waitingOn(AGAIN));
        return { ...ledger, rows: [...ledger.rows.filter((row) => row.kind === "message" && row.from.kind === "you"), FIRST, AGAIN] };
      });
      return store;
    },
  },
};
