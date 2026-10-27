// The Projects page: the left column (views, his projects, the repositories) beside the pane in the Cockpit's grid. A view or a
// repository lists its tasks grouped by project; a project lists its own, across repositories, in his order. Dispatching cards
// opens the one dispatch sheet on a repository, tasks as lines and issues as tags; a subtask sent without its parent carries the
// parent as context. The sheet's own act starts the Job, with the rows, which Fleet holds for it. Cards picked in several repositories open one request for each, the next once the one before
// has started or been put away.

import { useEffect, useRef, useState } from "react";
import { DockHandle } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import { useColumns } from "../cockpit/deck/columns";
import { useDispatching } from "../dispatch-panel/dispatching";
import { issueHit } from "../numbered";
import { Column } from "./column";
import { ProjectsKeys } from "./keys";
import { bandsIn } from "./list";
import type { ProjectRow } from "./model";
import { Pane } from "./pane";
import { allRows, parentOf, useProjects, useProjectsStore } from "./store";
import { VIEW_MARKS, groupOf, groupsOf, type View } from "./views";
import "../cockpit/cockpit.css";
import "../cockpit/deck/deck.css";
import "./projects-tab.css";

type Props = {
  picked: RepositorySummary | null;
  repositories: readonly RepositorySummary[];
  onOpenSession: (id: string) => void;
  onOpenLink: (address: string) => void;
};

/** Rows split by the repository they are in, in the order each is first met. */
const byRepository = (rows: readonly ProjectRow[]): ProjectRow[][] => {
  const out = new Map<string, ProjectRow[]>();
  for (const row of rows) out.set(row.manifestId, [...(out.get(row.manifestId) ?? []), row]);
  return [...out.values()];
};

export function ProjectsSurface({ picked, repositories, onOpenSession, onOpenLink }: Props) {
  const store = useProjectsStore()!;
  const snap = useProjects();
  const dispatching = useDispatching();
  const grid = useRef<HTMLDivElement>(null);
  const { columns, resize, reset } = useColumns(grid, "projects");
  const bands = bandsIn(picked, repositories);
  const project = snap.projects.find((one) => one.id === snap.open);
  const [view, setView] = useState<View>("all");
  const [repo, setRepo] = useState<string | undefined>(undefined);
  // A view or a repository takes the stage back from a project open.
  const chooseView = (next: View) => {
    store.open(undefined);
    setRepo(undefined);
    setView(next);
  };
  const chooseRepo = (manifestId: string) => {
    store.open(undefined);
    setView("all");
    setRepo(manifestId);
  };

  // Tasks ride as lines, each with its subtasks listed under it, and each issue as a tag, as a `#` chosen in the request
  // would: its reference and title in the words, and the tag with them. An issue a task carries rides as a tag too. A subtask
  // sent without its parent (`parentOf`) rides as its own title, then `Part of:` and the parent's title and notes, and the
  // parent's issues as tags: the parent is context, and the Job holds the subtask only. The rows sent stand in one
  // repository, which the sheet opens on.
  const send = (rows: readonly ProjectRow[]) => {
    const first = rows[0];
    if (dispatching === undefined || first === undefined) return;
    const root = bands.find((one) => one.manifestId === first.manifestId)?.root;
    const issues = rows.flatMap((row) => (row.kind === "issue" ? [row] : (parentOf(snap, row.id)?.issues ?? row.issues ?? [])));
    const tags = [...new Map(issues.map((one) => [one.reference, issueHit({ reference: one.reference, title: one.title, url: one.url }).tag] as const)).values()];
    const lines = rows.flatMap((row) => {
      if (row.kind !== "task") return [];
      const parent = parentOf(snap, row.id);
      if (parent === undefined) return [row.title, ...(row.subtasks ?? []).map((one) => `  - ${one.title}`)];
      const notes = parent.notes?.trim() ?? "";
      return [row.title, `Part of: ${parent.title}`, ...(notes === "" ? [] : [notes]), "Only the subtask is to be done."];
    });
    const words = [...lines, ...tags.map((tag) => tag.title)].join("\n");
    dispatching.choose("session");
    dispatching.openSheet(
      `${words} `,
      root,
      tags,
      rows.flatMap((row) => store.refOf(row.id) ?? []),
    );
  };
  // One request for each repository: the first opens now, and each next opens when the one before starts or is put away.
  const later = useRef<ProjectRow[][]>([]);
  const dispatch = (rows: readonly ProjectRow[]) => {
    const [first, ...rest] = byRepository(rows);
    later.current = rest;
    if (first !== undefined) send(first);
  };
  const sheet = dispatching?.sheet === true;
  const was = useRef(false);
  useEffect(() => {
    if (was.current && !sheet) {
      const next = later.current.shift();
      if (next !== undefined) send(next);
    }
    was.current = sheet;
  });
  const dispatchRow = (rowId: string) => {
    const row = allRows({ rows: [...snap.loose, ...snap.projects.flatMap((one) => one.rows)] }).find((one) => one.id === rowId);
    if (row !== undefined) dispatch([row]);
  };

  const hosts = { onOpenSession, onOpenLink, onDispatch: dispatch };
  const repository = bands.find((one) => one.manifestId === repo);
  // A task added on Needs you or In a Job would not show there, so those panes have no field.
  const pane =
    project !== undefined ? (
      <Pane key={`project:${project.id}`} {...hosts} label={`Project ${project.name}`} title={project.name} groups={groupOf(project, snap.showDone)} bands={bands} scope={{ key: project.id, project: project.id }} />
    ) : repository !== undefined ? (
      <Pane key={`repo:${repository.manifestId}`} {...hosts} label={`Repository ${repository.label}`} title={repository.label} groups={groupsOf(snap, "all", repository.manifestId)} bands={bands} scope={{ key: `repo:${repository.manifestId}`, repo: repository.manifestId }} />
    ) : (
      <Pane key={`view:${view}`} {...hosts} label={`View ${VIEW_MARKS[view].said}`} title={VIEW_MARKS[view].said} groups={groupsOf(snap, view, undefined)} bands={bands} scope={view === "all" ? { key: "all" } : undefined} />
    );
  const widths = columns === undefined ? undefined : ({ "--deck-left": `${columns.left}px`, "--deck-right": `${columns.right}px` } as React.CSSProperties);

  return (
    <div className="armada-cockpit deck projects-surface">
      <div ref={grid} className="deck-grid" data-projects style={widths}>
        <div className="deck-left">
          <Column bands={bands} view={view} repo={repo} onView={chooseView} onRepo={chooseRepo} />
          {columns === undefined ? null : <DockHandle quiet className="deck-handle" grows="right" label="the left column" width={columns.left} {...columns.range.left} onResize={(width) => resize("left", width)} onReset={() => reset("left")} />}
        </div>
        <div className="deck-centre">
          <div className="deck-stage">{pane}</div>
        </div>
      </div>
      <ProjectsKeys onDispatchRow={dispatchRow} onView={chooseView} />
    </div>
  );
}
