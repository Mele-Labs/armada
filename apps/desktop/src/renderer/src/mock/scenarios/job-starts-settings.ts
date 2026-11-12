// Settings › Job starts: per project, whether a Job that another Job or the Operator starts runs at once or waits on its start card.
// Cockpit trust is set to Ask me; the rest set nothing, so they read Start at once. Over `projects`. The walk `job-starts-settings` plays it.

import type { ValuesSet } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s331Projects } from "./projects";

const held = s331Projects.draft!.projectsTab!;

export const s342JobStartsSettings: Scenario = {
  ...s331Projects,
  name: "job-starts-settings",
  says: "Settings › Job starts lists each project with Start at once or Ask me for the Jobs that Jobs and the Operator start",
  draft: {
    ...s331Projects.draft,
    projectsTab: {
      ...held,
      projects: held.projects.map((one) =>
        one.name === "Cockpit trust"
          ? {
              ...one,
              values: {
                ...one.values,
                agent_starts: "ask_me",
              } as ValuesSet,
            }
          : one,
      ),
    },
  },
};
