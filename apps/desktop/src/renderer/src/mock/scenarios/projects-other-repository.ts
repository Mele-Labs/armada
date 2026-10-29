// Projects, with a task dispatched to another repository than its own, and a start Fleet refuses. Fleet's copy has lost Raise a
// call on all done since the list was read, so a start that names it is refused with Fleet's words, raised as the window's toast
// as the wired store raises it. Mock only. The walk `projects-other-repository` plays it.

import { raiseOutcome } from "../../tell";
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
        start: (tag, root, request, tuning, after, held, projectRows, toasted) => {
          if (projectRows?.some((one) => one.row === GONE) !== true) return store.start(tag, root, request, tuning, after, held, projectRows, toasted);
          if (toasted === true)
            raiseOutcome({ ok: false, why: "refused", error: { code: "fleet.no_such_project_row", message: REFUSED, run_id: "mock", fields: { act: "start_session", root: root ?? "", project_rows: JSON.stringify(projectRows) }, chain: [] } });
          return Promise.resolve(undefined);
        },
      };
    },
  },
};
