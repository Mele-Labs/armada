// The Workspace the Workspaces page and the pieces read draw, made of a project. Fleet has no Workspace of its own now: a project is the set of
// repositories a lifecycle or a rule is scoped to (`docs/practices/protocol/projects.md`), and this is the shape the page has always drawn it in.
// A project's file is Kit's, so it has no home repository, and one whose file did not read has no members to draw.

import type { Project, Projects } from "@armada/protocol";

/** A set of repositories that build one product, as the page draws it. */
export type WorkspaceOfRepositories = {
  /** The id a lifecycle or a rule is scoped to it by. */
  id: string;
  name: string;
  /** Manifest ids, in the order the file lists them. */
  repositories: string[];
  /** The repository whose `.armada/workspaces/` holds the file. Absent for one that is Kit's, which every project is. */
  home?: string;
  /** The place it was kept: `kit` or `repository`. */
  source: string;
  path: string;
  /** The place of the one that runs instead of this. Absent on the one that runs. */
  overridden_by?: string;
  /** The place whose file this one replaced because it says something else. */
  hides?: string;
};

/** One project as the Workspace it is: each repository once, in the order its members list them, folders ignored. */
export function workspaceOfProject(one: Project): WorkspaceOfRepositories {
  return {
    id: one.id,
    name: one.name,
    repositories: [
      ...new Set(one.members.map((member) => member.manifest_id)),
    ],
    source: "kit",
    path: "",
  };
}

/** The Workspaces among projects: those whose file read, since one that did not has no members to say. */
export function workspacesOfProjects(projects: Projects): WorkspaceOfRepositories[] {
  return projects.projects.filter((one) => one.unread === undefined).map(workspaceOfProject);
}
