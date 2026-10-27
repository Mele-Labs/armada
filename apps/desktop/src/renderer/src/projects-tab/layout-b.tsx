// Take B of the Projects tab, one grouped list and a thin rail. The rail holds the three views as glyphs and one glyph for each
// repository; choosing narrows the list. The toolbar groups the list and writes a new project, which asks its repository only
// while the rail is on All.

import { Plus } from "lucide-react";
import { DropdownMenu, Kbd, Tabs, Tooltip } from "@armada/components";

import { NameProject, type Band } from "./list";
import { PROJECTS_KEYS, capOf } from "./key-table";
import { useProjects, useProjectsStore } from "./store";
import { GROUP_IDS, VIEW_IDS, VIEW_MARKS, type GroupBy, type View } from "./views";

type RailProps = {
  bands: readonly Band[];
  view: View;
  onView: (view: View) => void;
  /** The repository the list is narrowed to, by Manifest id, or all of them. */
  repo: string | undefined;
  onRepo: (manifestId: string | undefined) => void;
};

export function Rail({ bands, view, onView, repo, onRepo }: RailProps) {
  return (
    <section className="deck-panel deck-list projects-list projects-rail" aria-label="Projects" tabIndex={-1}>
      <ul className="projects-rail__views" aria-label="Views">
        {VIEW_IDS.map((id, at) => {
          const { Glyph, short, said } = VIEW_MARKS[id];
          const here = view === id;
          return (
            <li key={id}>
              <Tooltip label={said} shortcut={PROJECTS_KEYS.view(at)} asChild>
                <button type="button" className="projects-rail__stop" data-stop data-view={id} data-selected={here || undefined} aria-current={here || undefined} aria-label={short} onClick={() => onView(id)}>
                  <Glyph size={16} strokeWidth={2} aria-hidden />
                </button>
              </Tooltip>
            </li>
          );
        })}
      </ul>
      <ul className="projects-rail__repos" aria-label="Repositories">
        {bands.map((band) => {
          const here = repo === band.manifestId;
          return (
            <li key={band.manifestId} className="projects-band" data-manifest={band.manifestId}>
              <Tooltip label={band.label} asChild>
                <button type="button" className="projects-rail__stop" data-stop data-repo={band.manifestId} data-selected={here || undefined} aria-current={here || undefined} aria-label={band.label} onClick={() => onRepo(here ? undefined : band.manifestId)}>
                  <span className="projects-rail__initial" aria-hidden="true">
                    {band.label.slice(0, 1).toUpperCase()}
                  </span>
                </button>
              </Tooltip>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

type ToolbarProps = {
  bands: readonly Band[];
  repo: string | undefined;
  by: GroupBy;
  onBy: (by: GroupBy) => void;
};

const BY_LABEL: Readonly<Record<GroupBy, string>> = { project: "Project", repository: "Repository", state: "State" };

/** Group by, and New project. The name field it asks for is drawn by the pane, under the toolbar. */
export function Toolbar({ bands, repo, by, onBy }: ToolbarProps) {
  const store = useProjectsStore()!;
  return (
    <div className="projects-toolbar" role="toolbar" aria-label="List">
      <Tooltip label="Group by" shortcut={PROJECTS_KEYS.group}>
        <Kbd>{PROJECTS_KEYS.group}</Kbd>
      </Tooltip>
      <Tabs items={GROUP_IDS.map((id) => ({ id, label: BY_LABEL[id], hint: `Group by ${BY_LABEL[id].toLowerCase()}` }))} value={by} onChange={(id) => onBy(id as GroupBy)} />
      <span className="projects-toolbar__new" data-new-project>
        {repo === undefined ? (
          <DropdownMenu
            icon={Plus}
            triggerLabel="New project"
            triggerTooltip={`New project, ${capOf(PROJECTS_KEYS.newProject)}`}
            entries={bands.map((band) => ({ kind: "item" as const, id: band.manifestId, label: band.label }))}
            onSelect={(id) => store.name(id)}
          />
        ) : (
          <Tooltip label="New project" shortcut={capOf(PROJECTS_KEYS.newProject)} asChild>
            <button type="button" className="projects-band__new" aria-label="New project" onClick={() => store.name(repo)}>
              <Plus size={16} strokeWidth={2} aria-hidden />
            </button>
          </Tooltip>
        )}
      </span>
    </div>
  );
}

/** The new project's name field, under the toolbar, in the repository it was asked for. */
export function NamingProject({ bands }: { bands: readonly Band[] }) {
  const { naming } = useProjects();
  const band = bands.find((one) => one.manifestId === naming);
  return band === undefined ? null : (
    <div className="projects-naming" role="group" aria-label={`New project in ${band.label}`}>
      <NameProject band={band} open={false} />
    </div>
  );
}
