// A Job on the feature workflow at Refine whose Open zone holds what has not landed: a draft pull request and an open ticket, beside the
// closed ticket and merged pull request already done. For the walk `stepped-open-unlanded`: on a stepped card, Open lists the unlanded work
// rather than leaving it empty. Over the same Job as `agent-row-refine`.

import type { SessionAttachment } from "@armada/screens/src/draft/sessions";

import { AUDIT_FILES, BUSY, ROTATION, approved, auditLedger } from "../audit-job";
import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const FLOW = [
  "Propose",
  "Discuss",
  "Approve plan",
  "Walk",
  "Refine",
  "Deliver",
];

const GIT = "https://git.example/armada/";

const DRAFT: SessionAttachment = {
  kind: "pull_request",
  number: 2940,
  title: "Keep the rotation window in the config",
  branch: "armada/audit-rotation-window",
  address: GIT + "pull/2940",
  checks: {
    state: "pending",
  },
  state: "draft",
  auto: false,
  closes: [],
};

export const s346SteppedOpenUnlanded: Scenario = {
  ...base,
  name: "stepped-open-unlanded",
  says: "A stepped Job whose Open zone lists a draft pull request and an open ticket",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, (one) => {
        const ledger = auditLedger(
          one,
          {
            steps: FLOW,
            at: 4,
          },
          [
            approved(ROTATION, "2026-10-26T13:58:00.000Z"),
            approved(AUDIT_FILES, "2026-10-26T14:06:00.000Z"),
          ],
        );
        return {
          ...ledger,
          attachments: [
            ...ledger.attachments,
            DRAFT,
          ],
        };
      });
      return store;
    },
  },
};
