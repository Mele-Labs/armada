// The Projects page: his projects, a panel for each repository, beside Issues in the Cockpit's grid. The list is the left
// column and the stage holds the project open, or a repository's own tasks. Dispatching rows opens the one dispatch sheet on the project's repository, tasks
// as lines and issues as tags; the sheet's own act starts the Job, with the rows, which Fleet holds for it.
// The mock also draws two other layouts of the same tab, take A and take B (`layout.ts`); Bridge draws the one above.

import { useRef, useState } from "react";
import { DockHandle } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import { useColumns } from "../cockpit/deck/columns";
import { useDispatching } from "../dispatch-panel/dispatching";
import { issueHit } from "../numbered";
import { GroupedPane } from "./grouped";
import { ProjectsKeys } from "./keys";
import { ViewsColumn } from "./layout-a";
import { NamingProject, Rail, Toolbar } from "./layout-b";
import { layoutOf } from "./layout";
import { ProjectsList, bandsIn } from "./list";
import type { Project, ProjectRow } from "./model";
import { ProjectPane } from "./project";
import { allRows, useProjects, useProjectsStore } from "./store";
import { GROUP_IDS, VIEW_MARKS, groupsOf, type Entry, type GroupBy, type View } from "./views";
import "../cockpit/cockpit.css";
import "../cockpit/deck/deck.css";
import "./projects-tab.css";
import "./layouts.css";

type Props = {
  picked: RepositorySummary | null;
  repositories: readonly RepositorySummary[];
  onOpenSession: (id: string) => void;
  onOpenLink: (address: string) => void;
};

export function ProjectsSurface({ picked, repositories, onOpenSession, onOpenLink }: Props) {
  const store = useProjectsStore()!;
  const snap = useProjects();
  const dispatching = useDispatching();
  const grid = useRef<HTMLDivElement>(null);
  const { columns, resize, reset } = useColumns(grid, "projects");
  const bands = bandsIn(picked, repositories);
  const project = snap.projects.find((one) => one.id === snap.open && bands.some((band) => band.manifestId === one.manifestId));
  const band = bands.find((one) => one.manifestId === project?.manifestId);

  // Tasks ride as lines, each with its subtasks listed under it, and each issue as a tag, as a `#` chosen in the request
  // would: its reference and title in the words, and the tag with them. An issue a task carries rides as a tag too. Every
  // row goes with the project it stands in; the rows picked are in one repository, which the sheet opens on.
  const send = (manifestId: string, picks: readonly Entry[]) => {
    const root = bands.find((one) => one.manifestId === manifestId)?.root;
    if (dispatching === undefined || picks.length === 0) return;
    const rows = picks.map((one) => one.row);
    const issues = rows.flatMap((row) => (row.kind === "issue" ? [row] : (row.issues ?? [])));
    const tags = issues.map((one) => issueHit({ reference: one.reference, title: one.title, url: one.url }).tag);
    const lines = rows.flatMap((row) => (row.kind === "task" ? [row.title, ...(row.subtasks ?? []).map((one) => `  - ${one.title}`)] : []));
    const words = [...lines, ...tags.map((tag) => tag.title)].join("\n");
    dispatching.choose("session");
    dispatching.openSheet(`${words} `, root, tags, picks.map((one) => ({ project: one.project.id, row: one.row.id })));
  };
  const dispatch = (from: Project, rows: readonly ProjectRow[]) => send(from.manifestId, rows.map((row) => ({ project: from, row })));
  const dispatchPicks = (picks: Entry[]) => {
    const first = picks[0];
    if (first !== undefined) send(first.project.manifestId, picks);
  };
  const dispatchRow = (rowId: string) => {
    const holder = snap.projects.find((one) => allRows(one).some((row) => row.id === rowId));
    const row = holder === undefined ? undefined : allRows(holder).find((one) => one.id === rowId);
    if (holder !== undefined && row !== undefined) dispatch(holder, [row]);
  };

  // The layout takes: the view, the repository the rail narrows to and the grouping live here, since the column and the pane read them.
  const layout = layoutOf();
  const [view, setView] = useState<View>("all");
  const [repo, setRepo] = useState<string | undefined>(undefined);
  const [by, setBy] = useState<GroupBy>("project");
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set());
  // A view takes the stage back from a project open.
  const chooseView = (next: View) => {
    store.open(undefined);
    setView(next);
  };
  const fold = (manifestId: string) => setFolded((was) => (was.has(manifestId) ? new Set([...was].filter((one) => one !== manifestId)) : new Set([...was, manifestId])));
  const cycle = () => setBy((was) => GROUP_IDS[(GROUP_IDS.indexOf(was) + 1) % GROUP_IDS.length]!);
  const hosts = { onOpenSession, onOpenLink, onDispatch: dispatch };
  const mixed = bands.length > 1;

  const left =
    layout === "a" ? (
      <ViewsColumn bands={bands} view={view} onView={chooseView} folded={folded} onFold={fold} />
    ) : layout === "b" ? (
      <Rail bands={bands} view={view} onView={setView} repo={repo} onRepo={setRepo} />
    ) : (
      <ProjectsList bands={bands} />
    );
  const centre =
    layout === "a" && project === undefined ? (
      <GroupedPane
        {...hosts}
        label={`View ${VIEW_MARKS[view].said}`}
        head={<h2 className="deck-pane__title">{VIEW_MARKS[view].said}</h2>}
        groups={groupsOf(snap.projects, bands, { view, by: "project", repo: undefined, prefix: mixed, keepEmpty: false })}
        bands={bands}
        adds={false}
        acts={false}
        onDispatchPicks={dispatchPicks}
      />
    ) : layout === "b" ? (
      <GroupedPane
        {...hosts}
        label="Tasks grouped"
        head={<Toolbar bands={bands} repo={repo} by={by} onBy={setBy} />}
        below={<NamingProject bands={bands} />}
        groups={groupsOf(snap.projects, bands, { view, by, repo, prefix: mixed && repo === undefined, keepEmpty: view === "all" })}
        bands={bands}
        adds={view === "all"}
        acts
        onDispatchPicks={dispatchPicks}
      />
    ) : (
      <ProjectPane band={band} project={project} onOpenSession={onOpenSession} onOpenLink={onOpenLink} onDispatch={dispatch} />
    );
  // Take B's rail is a narrow strip of a fixed width: it has no handle to drag.
  const rail = layout === "b";
  const widths = rail ? ({ "--deck-left": "calc(var(--space-12) * 1.5)" } as React.CSSProperties) : columns === undefined ? undefined : ({ "--deck-left": `${columns.left}px`, "--deck-right": `${columns.right}px` } as React.CSSProperties);

  return (
    <div className="armada-cockpit deck projects-surface" data-layout={layout === "current" ? undefined : layout}>
      <div ref={grid} className="deck-grid" data-projects style={widths}>
        <div className="deck-left">
          {left}
          {columns === undefined || rail ? null : <DockHandle quiet className="deck-handle" grows="right" label="the left column" width={columns.left} {...columns.range.left} onResize={(width) => resize("left", width)} onReset={() => reset("left")} />}
        </div>
        <div className="deck-centre">
          <div className="deck-stage">{centre}</div>
        </div>
      </div>
      <ProjectsKeys onDispatchRow={dispatchRow} {...(layout === "current" ? {} : { onView: layout === "a" ? chooseView : setView })} {...(layout === "b" ? { onGroup: cycle } : {})} />
    </div>
  );
}
