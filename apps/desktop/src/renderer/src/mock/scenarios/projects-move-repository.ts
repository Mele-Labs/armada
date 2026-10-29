// Projects on All, with a task filed in the wrong repository: Move the docs search box to the header is armada's, in site's Docs
// refresh, with two subtasks, armada's issue #2822 and armada's pull request #2878. The newest task Fleet lists is armada's, so the
// palette offers armada first. Mock only. The walk `projects-move-repository` plays it.

import type { Projects, ProjectRow } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s331Projects as projects } from "./projects";

const MISFILED: ProjectRow[] = [
  {
    kind: "task",
    id: "task-search-box",
    manifest_id: "armada",
    title: "Move the docs search box to the header",
    notes: "",
    state: "open",
    issues: [{ issue: "2822", manifest_id: "armada", title: "Docs search is hard to find", state: "open" }],
    pulls: [{ number: 2878, manifest_id: "armada", title: "Add a search box to the docs header", state: "open" }],
  },
  { kind: "task", id: "task-search-draw", manifest_id: "armada", title: "Draw the search box in the header", notes: "", state: "open", parent: "task-search-box" },
  { kind: "task", id: "task-search-drop", manifest_id: "armada", title: "Drop the old search page", notes: "", state: "open", parent: "task-search-box" },
];

const seed = projects.draft!.projectsTab!;
const PROJECTS: Projects = {
  ...seed,
  projects: seed.projects.map((one) => (one.id === "docs-refresh" ? { ...one, rows: [...one.rows, ...MISFILED] } : one)),
};

export const projectsMoveRepository: Scenario = {
  ...projects,
  name: "projects-move-repository",
  says: "Projects on All, with a task, its two subtasks, an issue and a pull request filed in armada that belong in site",
  draft: { ...projects.draft, projectsTab: PROJECTS },
};
