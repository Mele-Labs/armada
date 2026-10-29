// Projects, with a task dispatched to another repository than its own, and a start Fleet refuses. Fleet's copy has lost Raise a
// call on all done since the list was read, so a start that names it is refused with Fleet's words. Mock only. The walk
// `projects-other-repository` plays it.

import type { Scenario } from "../moment";
import { s331Projects as projects } from "./projects";

const GONE = "task-all-done";
const REFUSED = "no row is named task-all-done";

export const projectsOtherRepository: Scenario = {
  ...projects,
  name: "projects-other-repository",
  says: "Projects on All, with a task dispatched to another repository than its own, and a start Fleet refuses for a task gone",
  draft: {
    ...projects.draft,
    sessions: (control, more) => {
      const store = projects.draft!.sessions!(control, more);
      return {
        ...store,
        start: (tag, root, request, tuning, after, held, projectRows, refused) => {
          if (projectRows?.some((one) => one.row === GONE) !== true) return store.start(tag, root, request, tuning, after, held, projectRows);
          refused?.(REFUSED);
          return Promise.resolve(undefined);
        },
      };
    },
  },
};
