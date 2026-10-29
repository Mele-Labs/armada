// Projects, with a task dispatched to another repository than its own, and a start Fleet refuses. Fleet's copy has Raise a call on
// all done in site, moved there after the list was read, so a start of it in armada is refused with Fleet's words. Mock only. The
// walk `projects-other-repository` plays it.

import type { Scenario } from "../moment";
import { s331Projects as projects } from "./projects";

const MOVED = "task-all-done";
const REFUSED = "the task Raise a call on all done is in site, and this Job starts in armada";

export const projectsOtherRepository: Scenario = {
  ...projects,
  name: "projects-other-repository",
  says: "Projects on All, with a task dispatched to another repository than its own, and a start Fleet refuses",
  draft: {
    ...projects.draft,
    sessions: (control, more) => {
      const store = projects.draft!.sessions!(control, more);
      return {
        ...store,
        start: (tag, root, request, tuning, after, held, projectRows, refused) => {
          if (root?.endsWith("/armada") !== true || projectRows?.some((one) => one.row === MOVED) !== true) return store.start(tag, root, request, tuning, after, held, projectRows);
          refused?.(REFUSED);
          return Promise.resolve(undefined);
        },
      };
    },
  },
};
