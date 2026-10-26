// The Projects page: his projects, a panel for each repository, beside Issues in the Cockpit's grid. The list is the left
// column and the project open the stage. Dispatching rows opens the one dispatch sheet on the project's repository, tasks
// as lines and issues as tags; the sheet's own act starts the Job.

import { useRef } from "react";
import { DockHandle } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import { useColumns } from "../cockpit/deck/columns";
import { useDispatching } from "../dispatch-panel/dispatching";
import { issueHit } from "../numbered";
import { ProjectsKeys } from "./keys";
import { ProjectsList, bandsIn } from "./list";
import type { Project, ProjectRow } from "./model";
import { ProjectPane } from "./project";
import { allRows, useProjects } from "./store";
import "../cockpit/cockpit.css";
import "../cockpit/deck/deck.css";
import "./projects-tab.css";

type Props = {
  picked: RepositorySummary | null;
  repositories: readonly RepositorySummary[];
  onOpenSession: (id: string) => void;
  onOpenLink: (address: string) => void;
};

export function ProjectsSurface({ picked, repositories, onOpenSession, onOpenLink }: Props) {
  const snap = useProjects();
  const dispatching = useDispatching();
  const grid = useRef<HTMLDivElement>(null);
  const { columns, resize, reset } = useColumns(grid, "projects");
  const bands = bandsIn(picked, repositories);
  const project = snap.projects.find((one) => one.id === snap.open && bands.some((band) => band.manifestId === one.manifestId));
  const band = bands.find((one) => one.manifestId === project?.manifestId);

  // Tasks ride as lines, each with its subtasks listed under it, and each issue as a tag, as a `#` chosen in the request
  // would: its reference and title in the words, and the tag with them. An issue a task carries rides as a tag too.
  const dispatch = (from: Project, rows: readonly ProjectRow[]) => {
    const root = bands.find((one) => one.manifestId === from.manifestId)?.root;
    if (dispatching === undefined || rows.length === 0) return;
    const issues = rows.flatMap((row) => (row.kind === "issue" ? [row] : (row.issues ?? [])));
    const tags = issues.map((one) => issueHit({ reference: one.reference, title: one.title, url: one.url }).tag);
    const lines = rows.flatMap((row) => (row.kind === "task" ? [row.title, ...(row.subtasks ?? []).map((one) => `  - ${one.title}`)] : []));
    const words = [...lines, ...tags.map((tag) => tag.title)].join("\n");
    dispatching.choose("session");
    dispatching.openSheet(`${words} `, root, tags);
  };
  const dispatchRow = (rowId: string) => {
    const row = project === undefined ? undefined : allRows(project).find((one) => one.id === rowId);
    if (project !== undefined && row !== undefined) dispatch(project, [row]);
  };

  return (
    <div className="armada-cockpit deck projects-surface">
      <div ref={grid} className="deck-grid" data-projects style={columns === undefined ? undefined : ({ "--deck-left": `${columns.left}px`, "--deck-right": `${columns.right}px` } as React.CSSProperties)}>
        <div className="deck-left">
          <ProjectsList bands={bands} />
          {columns === undefined ? null : <DockHandle quiet className="deck-handle" grows="right" label="the left column" width={columns.left} {...columns.range.left} onResize={(width) => resize("left", width)} onReset={() => reset("left")} />}
        </div>
        <div className="deck-centre">
          <div className="deck-stage">
            <ProjectPane band={band} project={project} onOpenSession={onOpenSession} onOpenLink={onOpenLink} onDispatch={dispatch} />
          </div>
        </div>
      </div>
      <ProjectsKeys onDispatchRow={dispatchRow} />
    </div>
  );
}
