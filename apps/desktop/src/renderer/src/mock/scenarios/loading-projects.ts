// The Projects tab before Fleet has answered its first read: the read is held, so the column and the pane draw their skeleton rows and nothing lands.
// The same Fleet as `projects`, which is the one with data to land on.

import type { Scenario } from "../moment";
import { s331Projects } from "./projects";

export const loadingProjects: Scenario = {
  ...s331Projects,
  name: "loading/projects",
  says: "Projects while Fleet's first read is still out: skeleton entries in the column and skeleton rows in the pane",
  behaves: (...args) => ({
    ...s331Projects.behaves?.(...args),
    listProjects: () => new Promise(() => {}),
  }),
};
