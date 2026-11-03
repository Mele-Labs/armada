// Work still open is never filed under a finished step. Over `spine-top-brief`, a Job at Walk whose second pull request is a draft with a failed check, whose third is
// waiting on its review, and which has a follow-up not yet settled: they show in the Open group after the last step, and in the current step instead where that group is hidden. Mock only; the
// `openWorkStaysVisible` walk and `?scenario=open-work-stays-visible` open it.

import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import { lanesOfJob } from "@armada/screens/src/session-lanes";

import type { Scenario } from "../moment";
import { s335SpineTopBrief as base } from "./spine-top-brief";

const BUSY = "one-view-3";

const reopened = (one: SessionAttachment): SessionAttachment => {
  if (one.kind !== "pull_request") return one;
  if (one.number === 3012)
    return {
      ...one,
      state: "draft",
      auto: false,
      checks: {
        state: "failed",
        failing: "ci",
      },
      closes: one.closes ?? [],
    };
  if (one.number === 3013)
    return {
      ...one,
      state: "open",
      auto: false,
    };
  return one;
};

/** The lanes of a Job on a workflow that holds no ticket or pull request yet, by the function a real Job's are drawn with. */
const laneless = (one: Session) =>
  lanesOfJob({
    held: {
      session_id: one.id,
      lifecycles: [],
      instances: [],
      moves: [],
    },
    working: true,
    ...(one.work?.workflow === undefined
      ? {}
      : {
          workflow: {
            name: one.work.workflow.name,
            steps: one.work.workflow.steps.map((label) => ({
              id: label.toLowerCase().replaceAll(" ", "-"),
              label,
            })),
            began: true,
            ...(one.work.workflow.at === undefined
              ? {}
              : {
                  at: one.work.workflow.at,
                }),
          },
        }),
  });

export const s336OpenWorkStaysVisible: Scenario = {
  ...base,
  name: "open-work-stays-visible",
  says: "A Job at Walk with a draft pull request and an open one from finished steps, and a follow-up to settle: they stand in the Open group, never under a done step.",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(
        BUSY,
        (one): Session => ({
          ...one,
          attachments: one.attachments.map(reopened),
          lanes: laneless(one),
          later: [
            ...(one.later ?? []),
            {
              id: "later-1",
              text: "Fold the saved-query migration into the next release",
              at: "10:15:00",
            },
          ],
        }),
      );
      return store;
    },
  },
};
