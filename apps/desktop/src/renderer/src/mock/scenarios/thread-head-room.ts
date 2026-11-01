// `job-layouts` with the first Job named at length, its repository movable, its name one the wand can change, auto land on and a subagent running: the Thread head as it is
// when it is full. The walk `threadHeadRoom` plays it, in the head as it is and in the take that keeps one row (`?take=overflow`).

import type { Scenario } from "../moment";
import { sJobLayouts } from "./job-layouts";

/** The first Job, as `job-story` names it. */
const JOB = "story-1";

export const sThreadHeadRoom: Scenario = {
  ...sJobLayouts,
  name: "thread-head-room",
  says: "A Job whose long name and many acts fill its Thread head, in two takes of making room",
  draft: {
    ...sJobLayouts.draft,
    sessions: (board, more) => {
      const store = sJobLayouts.draft!.sessions!(board, more);
      store.edit(JOB, (one) => ({
        ...one,
        title: "Retry the slot lease when two Jobs ask for the same bay at once",
        autoLand: true,
        attachments: [...one.attachments, { kind: "subagent", id: "head-sa1", task: "Read the lease's retry path", state: "running", agentType: "explorer" }],
      }));
      return {
        ...store,
        move: async () => undefined,
        rename: (id, title) => store.edit(id, (one) => ({ ...one, title })),
        nameForMe: async () => undefined,
      };
    },
  },
};
